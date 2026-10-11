# API contract and feature matrix

What the Eloisesearch HTTP/JSON API promises. The contract is Eloisesearch's own
and versioned; it will be published as OpenAPI, which then becomes the source of
truth for request and response shapes. This page stays as the feature matrix and
the reasoning behind it.

The behavior targets come from one use case: a movie-search application (TMDB
dataset) with a Rails API and a Nuxt frontend. Everything listed as **Planned**
is required for that use case.

## Status legend

- **Supported:** implemented and covered by tests.
- **Planned:** required for the MVP, not implemented or not yet verified.
- **Partial:** accepted with documented limitations.
- **Out of scope:** deliberately excluded from the MVP.

Nothing moves to Supported without a test that fails when the behavior breaks.

## Error model (Supported)

Every non-2xx response has the same body:

```json
{ "error": { "type": "not_found", "reason": "no route for GET /nope" }, "status": 404 }
```

- `type` is stable and machine-readable; clients branch on it.
- `reason` is for humans and may change between versions.
- `status` repeats the HTTP status for clients that only see the body.

| `type` | Status | When |
| --- | --- | --- |
| `not_found` | 404 | Unknown route (later: unknown index or document). |
| `method_not_allowed` | 405 | Known route, unsupported method. |
| `invalid_index_name` | 400 | Index name breaks the naming rules. |
| `internal_error` | 500 | Unexpected failure; details are logged, never returned. |

A request that exceeds the configured timeout gets HTTP 408.

## Endpoints

Route shapes other than `/health` are a **draft** until the OpenAPI document is
published.

| Capability | Route | Status | Contract notes |
| --- | --- | --- | --- |
| Liveness | `GET /health` | **Supported** | `{"status":"ok","version":"<semver>"}`. |
| Readiness | `GET /ready` | Planned | Reports whether indexes are open and recovery finished. |
| Create / inspect / delete index | `PUT`, `GET`, `DELETE /indexes/{index}` | Planned | Schema (fields, types, analyzers) declared at creation; undeclared fields are rejected. |
| Index or replace document | `PUT /indexes/{index}/documents/{id}` | Planned | Stable IDs, deterministic create-vs-replace, `refresh` parameter. |
| Retrieve document | `GET /indexes/{index}/documents/{id}` | Planned | 404 `document_not_found` when missing. |
| Delete document | `DELETE /indexes/{index}/documents/{id}` | Planned | Idempotency and missing-document behavior to define. |
| Bulk write | `POST /indexes/{index}/documents:bulk` | Planned | Per-item success/error; one refresh at the end of a batch. |
| Search | `POST /indexes/{index}/search` | Planned | Query, filters, sort, pagination, field selection, facets, highlighting. |
| Count | `POST /indexes/{index}/count` | Planned | Same query and filter semantics as search. |
| Suggestions | `POST /indexes/{index}/suggest` | Planned | Prefix suggestions over a configured field. |
| Refresh | `POST /indexes/{index}/refresh` | Planned | Makes acknowledged writes visible to search. |

## Document model for the movie-search scenario

- **ID:** stable TMDB ID; re-importing replaces the same document.
- **title:** full-text, lowercase and accent folding, also indexed for prefix
  suggestions.
- **overview:** full-text.
- **genres, original_language:** exact keyword fields, usable as filters and facets.
- **popularity, vote_average:** numeric, for filtering and sorting.
- **release_date, year:** date plus derived integer year, for sorting and year facets.
- **poster_path, backdrop_path:** stored and returned, not searchable.
- The schema is explicit: fields not declared are not indexed.

## Search features

| Feature | Status | Rule |
| --- | --- | --- |
| User query syntax | Planned | Plain terms use OR for broad recall; `"quoted phrases"`; `-term` excludes; `+` between terms means AND (`space +adventure`); `term*` prefix; `term~1` fuzzy. The exact grammar will be documented with examples. |
| Exclusions | Planned | With an exclusion present, the positive part stays mandatory: `"Spider-Man" -Homecoming` never returns documents that only satisfy the exclusion. |
| Phrase boost | Planned | Phrase matches add ranking signal without becoming an extra required clause. |
| Field weighting | Planned | `title` above `overview` (initial ratio 10:2, tunable). |
| Filters | Planned | Genre (any of the given values) and minimum `vote_average`. Filters constrain results and never change scores. |
| Sorting | Planned | Text search: relevance, then a deterministic tie-break. Browse (empty query): popularity descending, then ID. |
| Pagination | Planned | Offset and limit; default 20, documented maximum; invalid values return 400. |
| Highlighting | Planned | Configurable tags (the demo uses `<mark>`); whole `title`, up to two 200-character `overview` fragments. |
| Facets | Planned | Genre counts (up to 20 buckets) and five-year release-year buckets. Behavior when a facet is also filtered must be specified. |
| Suggestions | Planned | Prefix match on title; blank input returns an empty list; duplicates removed; at most 8 results with `id`, `title`, `release_date`, `poster_path`, `vote_average`. |
| Accent folding | Planned | `amelie` matches accented titles. |
| Field selection | Planned | Return only the requested stored fields. |
| Malformed or incomplete queries | **Decision needed** | Either a documented 400 or lenient parsing that returns an empty result. The demo must never show a 500 while the user is still typing. |
| Advanced aggregations, vector search, scripting | Out of scope | Revisit only with a validated use case. |
| Distributed search, shards, replicas | Out of scope | Single-node MVP. |

## Draft search response

```json
{
  "total": 0,
  "took_ms": 0,
  "facets": {
    "genres": [{ "value": "Drama", "count": 12 }],
    "years": [{ "from": 2020, "to": 2024, "count": 5 }]
  },
  "hits": [
    {
      "id": "603",
      "score": 1.23,
      "fields": { "title": "Example movie", "overview": "Example overview" },
      "highlight": { "title": ["<mark>Example</mark> movie"] }
    }
  ]
}
```

Illustrative only. Open question: does `took_ms` measure engine time or the whole
request?

## Contract rules

1. Every supported feature has request, response and error examples in OpenAPI.
2. Tests assert matching document IDs and relative order where ranking matters,
   never exact scores.
3. Unknown parameters and unsupported query constructs fail with a documented
   error; they are never silently ignored.
4. Breaking changes require a new API version and a CHANGELOG entry.
5. This matrix changes in the same commit as the behavior it describes.
