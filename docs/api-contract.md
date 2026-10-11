# API contract and compatibility matrix

Eloisesearch exposes a documented subset of the **Elasticsearch 9.5.4** REST API
and Query DSL, so applications and tools built for Elasticsearch or OpenSearch can
switch servers with little or no code change.

This is a bounded, tested subset, not full compatibility. A request that uses
anything outside it fails with a documented error; it is never silently ignored or
answered with different semantics.

The subset is chosen from one use case: a movie-search application (TMDB dataset)
with a Rails API and a Nuxt frontend. The OpenAPI document, once published,
becomes the source of truth for request and response shapes; this page stays as
the matrix and the reasoning behind it.

## How this matrix is verified

The evidence lives in
[`eloisesearch-integration`](https://github.com/eloisesearch-foundation/eloisesearch-integration):
contract cases written as requests in the compatible API, a frozen movie corpus,
and a harness that runs them against both references and against Eloisesearch.
Every case passes on both references, and their reviewed results are committed as
baselines. The **Cases** column names the case groups behind each row:

| Group | Covers |
| --- | --- |
| [`index-lifecycle`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/index-lifecycle.json) | Index create, inspect, exists, delete, settings, and their errors. |
| [`documents`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/documents.json) | Index, replace, get, delete, `_create`, bulk, refresh and visibility. |
| [`mappings`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/mappings.json) | Dynamic mapping. |
| [`search`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/search.json) | The application's `_search`, `_count` and suggestion requests over the `movies` fixture. |
| [`query-string`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/query-string.json) | `query_string` over the `movies` fixture. |

Intentional and observed differences are recorded in [`divergences.json`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/divergences.json) and summarized under
[Divergences](#divergences).

## References

| Reference | Version | Role |
| --- | --- | --- |
| Elasticsearch REST API and Query DSL | 9.5.4 | Primary reference for every supported route and query feature. |
| OpenSearch REST API | 3.9.0 | Each item is verified against it; differences are listed under Divergences. |

Reference versions are pinned so compatibility tests are reproducible
(container images `docker.elastic.co/elasticsearch/elasticsearch:9.5.4` and
`opensearchproject/opensearch:3.9.0`). Moving a reference to a newer release is a
deliberate change: re-run the compatibility suite, update this table and record
any new differences.

## Legend

**Scope:** **MVP** (needed by the movie-search application), **Next** (after the
MVP), **Out** (not planned).

**Status:**

- **Supported:** implemented in Eloisesearch, covered by its tests, and its
  contract cases pass against Eloisesearch.
- **Specified:** in the MVP; the behavior is pinned by contract cases that pass on
  both references, and Eloisesearch does not implement it yet.
- **Partial:** accepted with documented differences (see Divergences).
- **Unsupported:** rejected with a documented error.

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

| `type` | HTTP | When | Scope | Status | Cases |
| --- | --- | --- | --- | --- | --- |
| `not_found` | 404 | Unknown route. The reference answers unknown routes differently; documented difference. | MVP | **Supported** | core HTTP tests |
| `method_not_allowed` | 405 | Known route, unsupported method. | MVP | **Supported** | core HTTP tests |
| `request_timeout` | 408 | Request exceeded `ELOISESEARCH_REQUEST_TIMEOUT_SECS`. | MVP | **Supported** | core HTTP tests |
| `internal_error` | 500 | Unexpected failure; details are logged, never returned. | MVP | **Supported** | core HTTP tests |
| `invalid_index_name_exception` | 400 | Index name breaks the naming rules. | MVP | Specified (validated in the core) | [`index-lifecycle`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/index-lifecycle.json) |
| `index_not_found_exception` | 404 | Index does not exist. | MVP | Specified (mapped in the server) | [`index-lifecycle`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/index-lifecycle.json), [`documents`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/documents.json) |
| `resource_already_exists_exception` | 400 | Creating an index that already exists. | MVP | Specified (mapped in the server) | [`index-lifecycle`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/index-lifecycle.json) |
| `illegal_argument_exception` | 400 | Unknown index setting (D-001), unsupported shard or replica count (D-003), explicit `dynamic: true` (D-004). | MVP | Specified (mapped in the server) | [`index-lifecycle`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/index-lifecycle.json), [`mappings`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/mappings.json) |
| `mapper_parsing_exception` | 400 | Unknown field type in a mapping. | MVP | Specified | [`index-lifecycle`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/index-lifecycle.json) |
| `version_conflict_engine_exception` | 409 | `_create` or `op_type=create` for an ID that exists, as a whole request or a bulk item. | MVP | Specified | [`documents`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/documents.json) |
| `search_phase_execution_exception` | 400 | Broken `query_string` syntax. | MVP | Specified | [`query-string`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/query-string.json) |
| Malformed JSON body | 400 | Body is not valid JSON; the type is not part of the contract (D-002). | MVP | Partial | [`index-lifecycle`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/index-lifecycle.json) |
| Strict dynamic mapping violation | 400 | A document has a field the `strict` mapping does not declare. | MVP | Specified | [`mappings`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/mappings.json) |

Missing documents follow the reference per route: `GET` answers 404 with
`"found": false`, `DELETE` answers 404 with `"result": "not_found"`, and neither
uses the error envelope.

## Endpoints

| Capability | Route | Scope | Status | Notes | Cases |
| --- | --- | --- | --- | --- | --- |
| Liveness (Eloisesearch) | `GET /health` | MVP | **Supported** | Eloisesearch-specific: `{"status":"ok","version":"<semver>"}`. | core HTTP tests |
| Server info | `GET /`, `HEAD /` | MVP | Specified | Name, Eloisesearch version, compatible reference version. | harness identity check |
| Cluster health | `GET /_cluster/health` | MVP | Specified | Single node; `green` once indexes are open and recovery is complete. | harness readiness check |
| Create index | `PUT /{index}` | MVP | Specified | `settings` and `mappings` from the subset below; unknown settings are rejected. `number_of_shards` 1 and `number_of_replicas` 0 or 1 (or absent) are accepted; other values are rejected (D-003). | [`index-lifecycle`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/index-lifecycle.json) |
| Inspect, delete, exists | `GET`, `DELETE`, `HEAD /{index}` | MVP | Specified | | [`index-lifecycle`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/index-lifecycle.json) |
| Index or replace document | `PUT /{index}/_doc/{id}` | MVP | Specified | `refresh=false`, `true` or `wait_for`. Durability and visibility rules: [`design-persistence.md`](design-persistence.md). | [`documents`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/documents.json) |
| Index with generated ID | `POST /{index}/_doc` | MVP | Specified | | [`documents`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/documents.json) |
| Create only | `PUT /{index}/_create/{id}`, `op_type=create` | MVP | Specified | 409 if the ID exists. | [`documents`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/documents.json) |
| Get, delete document | `GET`, `DELETE /{index}/_doc/{id}` | MVP | Specified | Realtime get; `_source_includes` and `_source_excludes`. | [`documents`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/documents.json) |
| Bulk | `POST /_bulk`, `POST /{index}/_bulk` | MVP | Specified | NDJSON (`application/x-ndjson`) `index`, `create`, `delete`; per-item results and errors; `refresh`. | [`documents`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/documents.json) |
| Search | `GET`, `POST /{index}/_search` | MVP | Specified | Query DSL subset below, plus the request features further down. | [`search`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/search.json), [`query-string`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/query-string.json) |
| Count | `GET`, `POST /{index}/_count` | MVP | Specified | Same query subset as search. | [`search`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/search.json) |
| Refresh | `POST /{index}/_refresh` | MVP | Specified | | [`documents`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/documents.json) |
| Update APIs, by-query APIs, `if_seq_no`/`if_primary_term`/`version` parameters, `routing` | | Next | Unsupported | Documented error. | |
| Aliases, snapshots, ingest pipelines, security APIs, cluster settings | | Out | Unsupported | Documented error. Eloisesearch-specific additions, if any, live under `/_eloisesearch/`. | |

Routes are served at the root, as in the reference API, so existing clients work
without path changes. There is no `/v1` prefix: compatibility is versioned by the
reference version above and by Eloisesearch's own semantic version.

## Mappings and analysis (index creation)

| Feature | Scope | Status | Notes | Cases |
| --- | --- | --- | --- | --- |
| Field types `text`, `keyword`, `integer`, `long`, `float`, `double`, `date`, `boolean` | MVP | Specified | | [`search`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/search.json) (`movies` mapping), [`index-lifecycle`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/index-lifecycle.json) |
| Multi-fields (`fields`), e.g. `title.keyword` | MVP | Specified | | [`search`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/search.json) |
| `index: false` (stored, not searchable) | MVP | Specified | Accepted in the `movies` mapping (poster fields). | [`search`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/search.json) (`movies` mapping) |
| `completion` field type | MVP | Specified | For the completion suggester. | [`search`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/search.json) |
| `dynamic: false` and `strict` | MVP | Specified | `false`: undeclared fields stay in `_source`, not searchable. `strict`: documents with undeclared fields are rejected. | [`mappings`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/mappings.json) |
| No `dynamic` declared | MVP | Partial | Behaves as `dynamic: false`; the references default to `true` and infer types (D-004). | [`mappings`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/mappings.json) |
| `dynamic: true` (type inference) | Next | Unsupported | 400 `illegal_argument_exception` (D-004). | [`mappings`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/mappings.json) |
| Custom analyzer: `standard` tokenizer with `lowercase`, `asciifolding`, `stop` (`_english_`), `stemmer` (`english`, `possessive_english`) | MVP | Specified | Exactly the chain of the `movies` mapping. | [`search`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/search.json) |
| `standard` analyzer | MVP | Specified | Default for `text` fields. | [`index-lifecycle`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/index-lifecycle.json), [`documents`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/documents.json) |
| Built-in language analyzer `english` | Next | Unsupported | The custom chain above covers the use case. | |
| `edge_ngram` filter, `search_analyzer` distinct from `analyzer` | Next | Unsupported | Not needed with the completion suggester. | |

## Query DSL subset

| Feature | Scope | Status | Notes | Cases |
| --- | --- | --- | --- | --- |
| `match_all` | MVP | Specified | Browse mode. | [`search`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/search.json) |
| `bool` (`must`, `filter`, `should`, `must_not`, `minimum_should_match`) | MVP | Specified | Filters never affect scores. | [`search`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/search.json), [`query-string`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/query-string.json) |
| `simple_query_string` (`fields` with boosts, `default_operator`, `lenient`) | MVP | Specified | User syntax: `"phrases"`, `-exclusion`, `+` as AND, `term*`, `term~N`. Never fails on syntax: incomplete input is interpreted as the references do (an unclosed quote becomes separate terms; `field:(` is plain text), so a search box never gets an error while the user types. | [`search`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/search.json) |
| `query_string` (`fields`, `default_operator`, `lenient`) | MVP | Specified | Phrases, `AND`/`OR`/`NOT`, `+`/`-`, groups, `field:term`, ranges `[a TO b]`, `term*`, `term~N`. Broken syntax is a 400 `search_phase_execution_exception`, never a 5xx. | [`query-string`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/query-string.json) |
| `match` (with `operator`), `multi_match` (`best_fields`, `type: phrase`) | MVP | Specified | | [`search`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/search.json) |
| `term`, `terms`, `range` | MVP | Specified | Keyword and numeric filters (genre, minimum rating, year). | [`search`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/search.json), [`query-string`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/query-string.json) |
| Exclusion semantics | MVP | Specified | The positive part stays mandatory: `"Spider-Man" -Homecoming` never returns documents that only lack the excluded term. With `simple_query_string` the application sets `default_operator: AND` when the query has an exclusion; `query_string` needs no switch. | [`search`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/search.json), [`query-string`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/query-string.json) |
| `prefix`, `wildcard`, `fuzzy` as standalone queries | Next | Unsupported | Covered through `simple_query_string` and `query_string` syntax. | |
| Scores | MVP | Partial (by design) | Relevance order is verified with fixtures; numeric `_score` values are not expected to match the references. | |

## Search request and response features

| Feature | Scope | Status | Notes | Cases |
| --- | --- | --- | --- | --- |
| `from` and `size` | MVP | Specified | Default size 10, as in the references. | [`search`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/search.json) |
| `sort` (fields, `_score`, `asc`/`desc`) | MVP | Specified | A `keyword` tie-break sorts as a string. | [`search`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/search.json) |
| `_source` includes and excludes | MVP | Specified | | [`search`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/search.json), [`documents`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/documents.json) |
| `hits.total` (`value`, `relation`) and `track_total_hits` | MVP | Specified | | [`search`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/search.json) |
| `highlight` (`fields`, `pre_tags`, `post_tags`, `number_of_fragments`, `fragment_size`) | MVP | Specified | Highlights keep the original text (`<mark>Amélie</mark>` for `amelie`); fragment boundaries are not compared. | [`search`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/search.json) |
| `aggs`: `terms`, `histogram` | MVP | Specified | Genre facets and five-year release-year buckets. | [`search`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/search.json) |
| `suggest`: completion suggester (`prefix`, `size`, `skip_duplicates`, `fuzzy`) | MVP | Specified | `fuzzy: AUTO` also suggests titles one edit away (`spi` suggests Spaceballs), as in the references. | [`search`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/cases/search.json) |
| Scripting, vector search, other aggregations | Out | Unsupported | | |
| Distributed search, multiple shards, replicas | Out | Unsupported | Single node, one shard per index (D-003). | |

## Divergences

Each entry is in [`divergences.json`](https://github.com/eloisesearch-foundation/eloisesearch-integration/blob/main/contracts/divergences.json), with the cases that reproduce it.

| Id | Capability | Elasticsearch 9.5.4 | OpenSearch 3.9.0 | Eloisesearch |
| --- | --- | --- | --- | --- |
| D-001 | Unknown index setting | 400 `illegal_argument_exception` | 400 `settings_exception` | Follows Elasticsearch. |
| D-002 | Malformed JSON body | 400 `x_content_e_o_f_exception` | 400 `parse_exception` | 400 with the envelope; the type is not part of the contract. |
| D-003 | `number_of_shards` other than 1, `number_of_replicas` above 1 | 200 | 200 | 400 `illegal_argument_exception` (single node, one shard). |
| D-004 | No `dynamic` declared; explicit `dynamic: true` | Defaults to `true`, infers types; accepts `true` | Same | Behaves as `false`; explicit `true` is a 400 until a later phase. |

## Verification rules

1. Every MVP row has contract cases that pass on both references. A row becomes
   **Supported** when those cases also pass against Eloisesearch, alongside its
   own tests.
2. Comparisons check document IDs, relative order, totals, facets, response shape
   and error `type` and status. Numeric scores are not compared.
3. Differences between the references, and intentional differences from both,
   are recorded as divergences with the cases that reproduce them.
4. A breaking change to supported behavior requires a major version and a
   CHANGELOG entry.
5. This matrix changes in the same commit as the behavior it describes.

---

Elasticsearch is a trademark of Elasticsearch B.V., registered in the U.S. and in
other countries. OpenSearch is a trademark of LF Projects, LLC. Eloisesearch is an
independent project, not affiliated with or endorsed by either; their names are
used only to describe API compatibility.
