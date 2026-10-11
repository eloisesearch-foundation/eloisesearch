# eloisesearch — Architecture

> The operational map: what the code is, how a request flows, and where things
> live. Why it is shaped this way is in [`design-decisions.md`](design-decisions.md);
> what the API promises is in [`api-contract.md`](api-contract.md).

## Purpose

Give an application full-text search (weighted fields, phrases, exclusions,
filters, facets, suggestions, highlighting, pagination) from a single process
with a local data directory, through a documented subset of the Elasticsearch
9.5.4 REST API so existing applications and tools can migrate. The first use case
it must serve end to end is a movie-search app with a Rails API and a Nuxt
frontend.

## Shape

```mermaid
flowchart TD
    CLIENT["Applications and SDKs<br/>(eloisesearch-ruby, eloisesearch-javascript)"]
    HTTP["eloisesearch-server<br/>axum: routing, validation, error model, config, logging"]
    CORE["eloisesearch-core<br/>index lifecycle, schema, documents, query execution"]
    ENGINE["Tantivy (Phase 2+)"]
    DISK[("Data directory<br/>indexes, write-ahead log, checkpoints")]

    CLIENT <-->|"HTTP / JSON"| HTTP
    HTTP <-->|"typed Rust calls"| CORE
    CORE <--> ENGINE
    ENGINE <--> DISK
```

Single node. No clustering, sharding or replication in the MVP.

## Crates

| Crate | Responsibility | Must not |
| --- | --- | --- |
| `eloisesearch-core` | Domain types (`IndexName`, …), errors, the data directory (`DataDir`: node metadata, exclusive lock, crash-safe index create, list and delete), and from the next Phase 2 steps the WAL, schema, document store and query execution. The API is blocking. | Depend on axum, tokio, HTTP types or the server. Expose Tantivy types. |
| `eloisesearch-server` | `Config` from `ELOISESEARCH_*`, tracing setup, the axum `Router`, `ApiError`, graceful shutdown, and the compatibility layer: parsing reference-API requests (routes, mappings, Query DSL) into core types and rendering core results in the reference response shapes. `lib.rs` holds everything testable; `main.rs` only wires config → tracing → `run`. | Contain search logic. Handlers parse, call the core, map the result. |

## Request flow

1. `TraceLayer` opens a span per request; the timeout layer aborts requests that
   exceed `ELOISESEARCH_REQUEST_TIMEOUT_SECS`, and `HandleErrorLayer` turns that
   into a 408 with the standard JSON error envelope.
2. The router matches the route. Unknown paths go to the `not_found` fallback,
   known paths with the wrong method to `method_not_allowed`. Both return
   `ApiError`.
3. A handler parses input into typed values, calls the core and returns JSON.
4. Core errors convert into `ApiError` through `From<eloisesearch_core::Error>`:
   validation errors become 400 with a specific `type` (`invalid_index_name_exception`,
   `illegal_argument_exception`, `resource_already_exists_exception`), a missing
   index becomes 404 `index_not_found_exception`; anything unexpected is logged
   and returned as a generic 500 without internal details.
5. Core calls do blocking file I/O, so handlers run them through
   `tokio::task::spawn_blocking`.

## Startup and shutdown

`main` reads `Config::from_env()`; an invalid value prints the variable, the
value and the reason, then exits with code 2. `run` opens the data directory
through `DataDir::open` (creating it on first start, taking the exclusive lock on
`node.lock`, checking the format version and removing interrupted index
operations), binds the listener and serves until Ctrl+C or SIGTERM, then drains
in-flight connections before exiting. A second process on the same data
directory fails at startup.

## Data directory

```
$ELOISESEARCH_DATA_DIR/
├── node.json        # format version, node id, creation time
├── node.lock        # held exclusively while a server runs
└── indexes/
    └── <name>/
        ├── index.json   # format version, uuid, settings, mappings
        ├── tantivy/     # filled from Phase 2, step 3
        └── wal/         # filled from Phase 2, step 2
```

Metadata files are written atomically (temp file, fsync, rename, fsync the
directory). Index creation and deletion go through `.creating-<uuid>` and
`.deleting-<uuid>` renames, so a crash never leaves a partial index visible. The
full design is in [`design-persistence.md`](design-persistence.md).

## Where things live

| Path | What |
| --- | --- |
| `crates/eloisesearch-core/src/index_name.rs` | Index name rules; names become directory names. |
| `crates/eloisesearch-core/src/error.rs` | Core error enum (`#[non_exhaustive]`). |
| `crates/eloisesearch-core/src/data_dir.rs` | `DataDir`: open, lock, node metadata, format checks, startup cleanup. |
| `crates/eloisesearch-core/src/indexes.rs` | Index lifecycle on disk: create, metadata, list, delete. |
| `crates/eloisesearch-core/src/fsio.rs` | Crash-safe file helpers (atomic write, directory fsync). |
| `crates/eloisesearch-server/src/config.rs` | Environment parsing; `from_lookup` is the test seam. |
| `crates/eloisesearch-server/src/error.rs` | `ApiError` and the JSON error body. |
| `crates/eloisesearch-server/src/routes.rs` | Router, middleware stack (`with_middleware`), handlers. |
| `crates/eloisesearch-server/tests/http.rs` | In-process HTTP tests (no socket). |

## Cross-cutting invariants

The list is maintained in [`AGENTS.md`](../AGENTS.md#cross-cutting-invariants-do-not-violate).
When you add one, add the test that fails if it is broken and reference it there.

## Testing strategy

- **Core unit tests**: pure, no I/O except temp directories.
- **Server HTTP tests**: drive `routes::router` with `tower::ServiceExt::oneshot`;
  no ports, no network.
- **Contract tests** (Phase 3): the behavioral fixtures from
  `eloisesearch-integration` run against a live server.

See [`quality-plan.md`](quality-plan.md) for relevance evaluation and benchmarks.
