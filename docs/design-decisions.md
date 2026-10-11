# Design decisions

> Why Eloisesearch is shaped the way it is. Each section states the decision,
> the reasoning and the consequences we accept. For the current code map read
> [`ARCHITECTURE.md`](ARCHITECTURE.md); for what the API promises read
> [`api-contract.md`](api-contract.md). New decisions get a section here and,
> when they need detailed design, a `design-<topic>.md` next to this file.

## 1. Product shape: a single-node search service

**Decision.** One process, one data directory, an HTTP/JSON API. No clustering,
sharding or replication in the MVP.

**Why.** The target is application search (catalogs, content, records) for
developers who do not want to operate a distributed system. A single node keeps
deployment, backup and recovery understandable while the real requirements are
validated.

**Consequences.** Persistence protects against process and machine restarts, not
against losing the only disk. High availability is out of scope until a use case
demands it.

## 2. Engine: Rust and Tantivy

**Decision.** Implement the service in Rust on top of the Tantivy search library.

**Why.** One language for engine and service, a mature indexing and query library,
and control over the API and developer experience.

**Consequences.** Eloisesearch must build everything around the library itself:
persistence and recovery, document lifecycle, the API contract, operations and
SDKs. Rust and Tantivy are implementation choices, not evidence of performance;
speed or footprint claims require measurements (see
[`quality-plan.md`](quality-plan.md)).

**Alternatives considered.** Running an existing distributed search platform (large
feature and operational surface, which is what our users want to avoid); search
embedded in the application database (different deployment boundary, does not fit
a standalone multi-source API).

## 3. Transport: HTTP/JSON first, behind a transport-independent core

**Decision.** The public API is HTTP/JSON, documented with OpenAPI. The search core
knows nothing about HTTP.

**Why.** Easy to inspect with curl, to integrate from Rails and JavaScript, and to
test. Keeping the core independent lets it be tested without a server and leaves
room for another transport later without rewriting query or index logic.

**Consequences.** gRPC or other protocols are added only if a concrete use case or
benchmark justifies a second public protocol.

## 4. API compatibility as the adoption path

**Decision.** The public API is a documented subset of the **Elasticsearch 9.5.4**
REST API and Query DSL, served at the same root paths (`PUT /{index}`,
`/{index}/_doc/{id}`, `/_bulk`, `/{index}/_search`, ...). An item is
marked supported only after it is checked against both Elasticsearch 9.5.4 and
**OpenSearch 3.9.0** by the compatibility suite, and differences are recorded. There is no `/v1`
prefix. Eloisesearch-specific endpoints, when needed, live under
`/_eloisesearch/` or are clearly marked (such as `GET /health`).

**Why.** The fastest way to adopt Eloisesearch is not to learn a new API: teams
with applications, scripts and tools written for Elasticsearch or OpenSearch
should be able to point them at Eloisesearch and keep working, as long as they use
the supported subset. A path prefix would break every existing client.

**Consequences.**

- Compatibility is bounded and versioned: [`api-contract.md`](api-contract.md)
  lists every route, mapping feature and query feature as supported, partial,
  planned, unsupported or out of scope. Anything outside it fails with a
  documented error, never with silently different behavior.
- Results are compared with the reference by document IDs, relative order,
  totals, facets, response shape and error types, never by numeric scores.
- Versioning follows two axes: the reference version the subset tracks, and
  Eloisesearch's own semantic version. `GET /` reports both.
- The product check in the official Elasticsearch clients (7.14 and later)
  requires an `X-Elastic-Product: Elasticsearch` response header. Eloisesearch
  does not send it, because that would present the server as another vendor's
  product. Plain HTTP, OpenSearch clients and generic tools are the supported
  paths; the migration guide documents client-side options. To be revisited only
  with legal advice.
- Third-party product names appear only where compatibility requires them: the
  compatibility matrix, the migration guide and compatibility tests, always
  descriptively and with the trademark notice.

**Contract order.**

1. Application behavior first: the movie-search fixtures, expressed as requests
   in the compatible API.
2. The minimum compatible API next: index lifecycle and mappings, stable-ID
   document operations, bulk with item-level errors, search and count, pagination,
   sort, `_source` filtering, the query subset, aggregations for facets,
   highlighting, suggestions, refresh, server info and health.
3. Each capability is labeled MVP, next phase or out of scope, and supported,
   partial or unsupported, in [`api-contract.md`](api-contract.md).

## 5. Durability and search visibility

**Decision.** Near-real-time semantics that separate **write durability** from
**search visibility**.

- **Durable before acknowledged.** A write returns success only after it is
  persisted, through a write-ahead operation log (WAL) synced before the
  acknowledgment, plus index commits and checkpoints. On restart, operations not
  in the last committed index state are replayed.
- **Visible after refresh.** With the default `refresh=false`, a durable write may
  not be searchable until the next automatic refresh (default about 1 second,
  configurable per index).
- `refresh=true` refreshes before responding (immediate visibility, at a latency
  and throughput cost). `refresh=wait_for` waits for a refresh that includes the
  write, without forcing one.
- An explicit refresh operation exists; bulk ingestion writes the batch and
  refreshes once.
- All index and recovery data lives under `ELOISESEARCH_DATA_DIR`, which must be
  persistent storage.

**Acceptance tests.**

- A successful write survives a restart even without an explicit refresh.
- With default refresh, tests do not assume immediate visibility; the document
  appears after refresh.
- `refresh=true` and `refresh=wait_for` make the document searchable before their
  successful response.
- Updates and deletes recover consistently after restart, without resurrecting
  older versions.
- Bulk responses report item-level failures accurately; the final refresh makes
  the successful items searchable.
- Recovery is exercised after both graceful shutdown and abrupt termination.

**Design.** WAL record format, sync policy, checkpoints, recovery and the
refresh mechanism are specified in [`design-persistence.md`](design-persistence.md).

## 6. One error shape

**Decision.** Every failed request returns the reference API's envelope:
`{"error":{"root_cause":[{"type","reason"}],"type","reason"},"status"}`. Where the
reference has an equivalent error, the same `type` and HTTP status are used.

**Why.** Existing clients already parse this shape, and SDKs handle errors with one
code path that branches on a stable `type`; humans read `reason`. Internal details
(paths, panics, library messages) never reach clients; they are logged.

**Consequences.** Middleware errors (timeouts, for example) are converted into
this envelope too; no layer may answer with an empty or plain-text body.

## 7. Configuration through the environment

**Decision.** All settings come from `ELOISESEARCH_*` variables with safe defaults;
invalid values stop the process at startup.

**Why.** Works the same in a shell, Docker, Compose and Codespaces, keeps secrets
and host details out of the repository, and fails loudly instead of running with a
half-understood configuration. A configuration file can be added later if the
number of settings grows.
