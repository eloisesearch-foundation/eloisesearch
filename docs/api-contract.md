# API contract and compatibility matrix

Eloisesearch exposes a documented subset of the **Elasticsearch 9.5.4** REST API
and Query DSL, so applications and tools built for Elasticsearch or OpenSearch can
switch servers with little or no code change. An item is marked **Supported** only
after it has been checked against both references below; differences with
OpenSearch are recorded here. The harness that runs these checks is still being
built (roadmap Phase 0), so no compatible route is marked Supported yet.

This is a bounded, tested subset, not full compatibility. A request that uses
anything outside it fails with a documented error; it is never silently ignored or
answered with different semantics.

The subset is chosen from one use case: a movie-search application (TMDB dataset)
with a Rails API and a Nuxt frontend. The OpenAPI document, once published,
becomes the source of truth for request and response shapes; this page stays as
the matrix and the reasoning behind it.

## References

| Reference | Version | Role |
| --- | --- | --- |
| Elasticsearch REST API and Query DSL | 9.5.4 | Primary reference for every supported route and query feature. |
| OpenSearch REST API | 3.9.0 | Each supported item is verified against it; differences are listed per item. |

Reference versions are pinned so compatibility tests are reproducible
(container images `docker.elastic.co/elasticsearch/elasticsearch:9.5.4` and
`opensearchproject/opensearch:3.9.0`). Moving a reference to a newer release is a
deliberate change: re-run the compatibility suite, update this table and record
any new differences.

## Status legend

- **Supported:** implemented, covered by tests, and verified against the reference.
- **Planned:** in the MVP subset, not implemented or not verified yet.
- **Partial:** accepted with documented differences.
- **Unsupported:** rejected with a documented error.
- **To decide:** depends on a roadmap decision.
- **Out of scope:** not planned for the MVP.

## Using existing clients and tools

| Client | Expected to work | Notes |
| --- | --- | --- |
| Plain HTTP (curl, `fetch`, Faraday, generic HTTP tools) | Yes, for the supported subset | The primary target. |
| OpenSearch language clients | Yes, for the supported subset | To be verified per client in the integration suite. |
| Official Elasticsearch clients 7.14+ (including 8.x and 9.x) | **No, without a client-side change** | These clients run a product check and refuse servers that do not send `X-Elastic-Product: Elasticsearch`. Eloisesearch does not send that header, because it would present the server as another vendor's product. The migration guide will document the client-side options. |

`GET /` reports Eloisesearch's own name and version, plus the reference version it
is compatible with, so tools can detect what they are talking to.

## Error model

Every non-2xx response uses the same envelope as the reference API:

```json
{
  "error": {
    "root_cause": [{ "type": "index_not_found_exception", "reason": "no such index [movies]" }],
    "type": "index_not_found_exception",
    "reason": "no such index [movies]"
  },
  "status": 404
}
```

- `type` is stable and machine-readable. Where the reference has an equivalent
  error, Eloisesearch uses the same `type` and status.
- `reason` is for humans and may change between versions.
- `root_cause` lists underlying causes; without a deeper cause it repeats the error.

| `type` | HTTP | When | Status |
| --- | --- | --- | --- |
| `invalid_index_name_exception` | 400 | Index name breaks the naming rules. | **Supported** (core validation; exposed once index routes exist) |
| `not_found` | 404 | Unknown route. The reference answers unknown routes differently; documented difference. | **Supported** |
| `method_not_allowed` | 405 | Known route, unsupported method. | **Supported** |
| `request_timeout` | 408 | Request exceeded `ELOISESEARCH_REQUEST_TIMEOUT_SECS`. | **Supported** |
| `internal_error` | 500 | Unexpected failure; details are logged, never returned. | **Supported** |
| `index_not_found_exception` | 404 | Index does not exist. | Planned (mapped in the server; exposed with the index routes) |
| `resource_already_exists_exception` | 400 | Creating an index that already exists. | Planned (mapped in the server; exposed with the index routes) |
| `version_conflict_engine_exception` | 409 | `op_type=create` or `_create` for an ID that exists. | Planned |
| `parsing_exception`, `illegal_argument_exception` | 400 | Malformed or unsupported request body, query or parameter. | Planned (`illegal_argument_exception` mapped in the server) |

Missing documents follow the reference per route (for example `"found": false` on
get).

## Endpoints

| Capability | Route | Status | Notes |
| --- | --- | --- | --- |
| Liveness (Eloisesearch) | `GET /health` | **Supported** | Eloisesearch-specific: `{"status":"ok","version":"<semver>"}`. |
| Server info | `GET /`, `HEAD /` | Planned | Name, Eloisesearch version, compatible reference version. |
| Cluster health | `GET /_cluster/health` | Planned | Single node; `green` once indexes are open and recovery is complete. |
| Create index | `PUT /{index}` | Planned | `settings` (analysis) and `mappings` from the subset below; unknown settings are rejected. |
| Inspect, delete, exists | `GET`, `DELETE`, `HEAD /{index}` | Planned | |
| Index or replace document | `PUT /{index}/_doc/{id}` | Planned | Stable IDs; `refresh=false`, `true` or `wait_for`. Durability and visibility rules: [`design-persistence.md`](design-persistence.md). |
| Index with generated ID | `POST /{index}/_doc` | Planned | |
| Create only | `PUT /{index}/_create/{id}`, `op_type=create` | Planned | 409 if the ID exists. |
| Get, delete document | `GET`, `DELETE /{index}/_doc/{id}` | Planned | `_source` filtering on get. |
| Bulk | `POST /_bulk`, `POST /{index}/_bulk` | Planned | NDJSON `index`, `create`, `delete` actions; per-item results and errors; `refresh`. |
| Search | `GET`, `POST /{index}/_search` | Planned | Query DSL subset below, `from`/`size`, `sort`, `_source`, `aggs`, `highlight`, `suggest`. |
| Count | `GET`, `POST /{index}/_count` | Planned | Same query subset as search. |
| Refresh | `POST /{index}/_refresh` | Planned | |
| Update APIs, by-query APIs, `if_seq_no`/`if_primary_term`/`version` parameters, `routing` | | Unsupported (Phase 2) | Documented error; revisit after the MVP. |
| Everything else | | Unsupported | Documented error. Eloisesearch-specific additions, if any, live under `/_eloisesearch/`. |

Routes are served at the root, as in the reference API, so existing clients work
without path changes. There is no `/v1` prefix: compatibility is versioned by the
reference version above and by Eloisesearch's own semantic version.

## Mappings and analysis (index creation)

| Feature | Status | Notes |
| --- | --- | --- |
| `dynamic: false` and `strict` | Planned | Undeclared fields stay in `_source` without being indexed (`false`) or are rejected (`strict`). |
| Field types `text`, `keyword`, `integer`, `long`, `float`, `double`, `date`, `boolean` | Planned | |
| Multi-fields (`fields`), e.g. `title.keyword` | Planned | |
| `index: false` (stored, not searchable) | Planned | |
| Analyzers: `standard`; custom with `standard` tokenizer and `lowercase`, `asciifolding`, `edge_ngram` filters | Planned | |
| `search_analyzer` distinct from `analyzer` | Planned | |
| `completion` field type | To decide | Needed if suggestions use the completion suggester. |
| Language analyzer `english` (stemming, stop words) | To decide | Depends on the relevance fixtures. |

## Query DSL subset

| Feature | Status | Notes |
| --- | --- | --- |
| `match_all` | Planned | Browse mode. |
| `bool` (`must`, `filter`, `should`, `must_not`, `minimum_should_match`) | Planned | Filters never affect scores. |
| `simple_query_string` (`fields` with boosts such as `title^10`, `default_operator`, `analyze_wildcard`, `lenient`) | Planned | User syntax: `"phrases"`, `-exclusion`, `+` between terms as AND, `term*`, `term~1`. |
| `match` (with `operator`), `multi_match` (`type: phrase` for a phrase boost) | Planned | |
| `term`, `terms`, `range` | Planned | Keyword and numeric filters (genre, minimum rating). |
| `query_string` | To decide | Larger grammar; supported only if the use case needs it. |
| `prefix`, `wildcard`, `fuzzy` as standalone queries | To decide | Covered through `simple_query_string` first. |
| Exclusion semantics | Planned | With an exclusion, the positive part stays mandatory: `"Spider-Man" -Homecoming` never returns documents that only satisfy the exclusion. |
| Scores | Partial (by design) | Relevance order is verified with fixtures; numeric `_score` values are not expected to match the reference. |

## Search request and response features

| Feature | Status | Notes |
| --- | --- | --- |
| `from` and `size` | Planned | Default size 10, as in the reference; documented maximum window. |
| `sort` (fields, `_score`, `asc`/`desc`) | Planned | Deterministic ordering with an explicit tie-break field. |
| `_source` includes and excludes | Planned | |
| `hits.total` (`value`, `relation`) and `track_total_hits` | Planned | |
| `highlight` (`fields`, `pre_tags`, `post_tags`, `number_of_fragments`, `fragment_size`) | Planned | Fragment boundaries may differ; documented per case. |
| `aggs`: `terms`, `histogram` | Planned | Genre facets and five-year release-year buckets. |
| `suggest`: completion suggester with `skip_duplicates` | To decide | Alternative: prefix `match` on an edge-n-gram field. |
| Malformed or incomplete user queries | To decide | `lenient` parsing or 400 `parsing_exception`; the demo must never show a 500 while the user types. |
| Scripting, vector search, ingest pipelines, aliases, snapshots, security APIs | Out of scope | |
| Distributed search, shards, replicas | Out of scope | Single node. Index settings for shards and replicas: accepted values to be decided. |

## Verification rules

1. Every **Supported** row has an Eloisesearch test and, for compatible routes, a
   compatibility test that sends the same request to the reference and compares.
2. Comparisons check document IDs, relative order, totals, facets, response shape
   and error `type` and status. Numeric scores are not compared.
3. Differences found against OpenSearch are listed in the row's notes.
4. A breaking change to supported behavior requires a major version and a
   CHANGELOG entry.
5. This matrix changes in the same commit as the behavior it describes.

---

Elasticsearch is a trademark of Elasticsearch B.V., registered in the U.S. and in
other countries. OpenSearch is a trademark of LF Projects, LLC. Eloisesearch is an
independent project, not affiliated with or endorsed by either; their names are
used only to describe API compatibility.
