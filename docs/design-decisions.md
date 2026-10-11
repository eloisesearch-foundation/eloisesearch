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

**Alternatives considered.** Adopting an existing search platform (large feature
and operational surface, does not explore this project's hypothesis); search
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

## 4. Our own versioned API contract

**Decision.** Eloisesearch defines its own API, versioned (e.g. `/v1`) and
published as OpenAPI. There is no compatibility target with any other product.

**Why.** The API can be designed around application search, evolve on its own
schedule, and be tested exhaustively, without inheriting someone else's surface
area.

**Consequences.** Query grammar, analysis, scoring and highlighting behavior are
specified by our contract and its fixtures. Every supported feature needs tests;
every unsupported construct fails predictably.

**Contract order.**

1. Application behavior first: the movie-search fixtures.
2. Minimum engine API next: index lifecycle and metadata, stable-ID document
   operations, bulk writes with item-level errors, search and count, bounded
   pagination, sort and field selection, suggestions, facets, highlighting,
   refresh controls, health and readiness.
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

**Open.** WAL record format, sync policy, checkpoint and compaction algorithm,
crash-consistency protocol and the reader refresh mechanism are specified in
`design-persistence.md` before Phase 2 code starts.

## 6. One error shape

**Decision.** Every failed request returns `{"error":{"type","reason"},"status"}`.

**Why.** Clients and SDKs handle errors with one code path and branch on a stable
`type`; humans read `reason`. Internal details (paths, panics, library messages)
never reach clients; they are logged.

## 7. Configuration through the environment

**Decision.** All settings come from `ELOISESEARCH_*` variables with safe defaults;
invalid values stop the process at startup.

**Why.** Works the same in a shell, Docker, Compose and Codespaces, keeps secrets
and host details out of the repository, and fails loudly instead of running with a
half-understood configuration. A configuration file can be added later if the
number of settings grows.
