# Design: Persistence and search visibility

- **Status:** Accepted (eloisesearch#2)
- **Date:** 2026-10-11
- **Roadmap:** Phase 2, indexing and persistence ([ROADMAP-MVP.md](https://github.com/eloisesearch-foundation/eloisesearch-workspace/blob/main/docs/ROADMAP-MVP.md))

## Objective

An application that indexes a movie with `PUT /movies/_doc/603` and receives
`201 Created` must never lose that movie, whether the process is stopped
gracefully, killed, or the machine loses power right after the response. The same
application must be able to choose how quickly the change becomes searchable:
within about a second by default, immediately with `refresh=true`, or by waiting
for the next refresh with `refresh=wait_for`. A `GET` of the document by ID must
see the latest acknowledged version even before it is searchable, as clients of
the reference API expect.

This design specifies how Eloisesearch stores indexes and documents on a single
node so that these guarantees hold, and how the HTTP routes of Phase 2 map onto
it. It is the prerequisite for implementing `PUT /{index}`, the `_doc` routes,
`_bulk` and `_refresh`.

## Method

### Current state

Verified against the code at `ebc6783`:

- `eloisesearch-core` has `IndexName` (validated, used as a directory name) and a
  `#[non_exhaustive]` `Error` enum. It has no Tantivy dependency and no I/O.
- `eloisesearch-server` creates `ELOISESEARCH_DATA_DIR` at startup
  (`lib.rs::run`) and serves `GET /health`. Every error goes through `ApiError`
  with the reference envelope; timeouts become 408 through `HandleErrorLayer`.
- Invariants that constrain this design (see `AGENTS.md`): the core does not
  depend on tokio, axum or HTTP types; Tantivy types never appear in public
  signatures; acknowledged writes are durable; tests are hermetic.

Nothing below exists yet; nothing that exists has to be rebuilt.

### Concepts

| Term | Meaning in Eloisesearch |
| --- | --- |
| **Acknowledged** | The HTTP response for the write has been sent with a 2xx status. |
| **Durable** | The write is in the write-ahead log (WAL) and the WAL has been synced to stable storage. |
| **Applied** | The write has been handed to the Tantivy `IndexWriter`. |
| **Committed** | A Tantivy commit that includes the write has completed; its checkpoint is recorded. |
| **Visible** | The search reader has been reloaded to a commit that includes the write. |
| **Refresh** | Commit, then reload the search reader (makes writes visible). |
| **Flush** | Commit without reloading the search reader (bounds WAL size and recovery time, does not change visibility). |

Tantivy can only search committed segments; there is no near-real-time search of
uncommitted data. Eloisesearch therefore implements a refresh as a commit followed
by a reader reload, and keeps durability independent of commits through the WAL.

### On-disk layout

```
$ELOISESEARCH_DATA_DIR/
├── node.json                      # format version, node id, created at
└── indexes/
    ├── movies/
    │   ├── index.json             # settings and mappings as accepted, format version, uuid
    │   ├── tantivy/               # Tantivy directory (segments, meta.json)
    │   └── wal/
    │       ├── 00000000000000000001.wal
    │       └── 00000000000000004097.wal   # file name = first sequence number it holds
    ├── .creating-<uuid>/          # index being created (removed or renamed on startup)
    └── .deleting-<uuid>/          # index being deleted (removed on startup)
```

- Directory names come from `IndexName`, whose rules already exclude path
  separators, dot-prefixed and uppercase names.
- `node.json` and `index.json` carry a `format_version`. A newer format than the
  binary understands makes startup fail with a clear message instead of guessing.
- Every metadata file is written atomically: write `name.tmp`, `fsync` it,
  rename over `name`, `fsync` the parent directory.

### Index lifecycle (not in the WAL)

- **Create** (`PUT /{index}`): validate settings and mappings against the
  supported subset, build the directory under `.creating-<uuid>/` (index.json,
  empty Tantivy index, empty WAL), `fsync` it, then rename it to `indexes/<name>/`
  and `fsync` `indexes/`. The rename is the commit point. If the name exists,
  return `resource_already_exists_exception` (400).
- **Delete** (`DELETE /{index}`): stop accepting writes, close the writer and
  readers, rename to `.deleting-<uuid>/`, `fsync` `indexes/`, acknowledge, then
  remove the directory in the background.
- **Startup:** remove leftover `.creating-*` and `.deleting-*` directories, then
  open every index under `indexes/`.

Index creation and deletion are rare and already atomic through rename, so they
do not need a log.

### Write path

Each open index has one **writer thread** that owns the Tantivy `IndexWriter`, the
WAL and the in-memory version map. Requests reach it through a channel and get
their result back through a one-shot reply. The core exposes a blocking API; the
server calls it with `tokio::task::spawn_blocking`, which keeps tokio out of the
core.

For each write (index, create, delete) the writer thread:

1. **Validates** the operation against the mapping and, for `op_type=create`,
   checks existence (version map first, then the realtime reader). Failures are
   returned before anything is logged.
2. **Assigns** the next sequence number (`u64`, per index, starting at 1) and the
   document's next `_version`.
3. **Appends** the record to the current WAL file.
4. **Syncs** the WAL (group commit, below). Only after `fdatasync` returns is the
   write durable.
5. **Applies** it to the `IndexWriter`: an index operation is
   `delete_term(_id)` followed by `add_document`; a delete is `delete_term(_id)`.
   Tantivy applies a delete only to documents added before it, so this order
   replaces documents correctly.
6. **Records** it in the version map: `_id → (seq_no, _version, source or tombstone)`.
7. **Replies.** The server answers with the reference response shape:
   `_index`, `_id`, `_version`, `result` (`created`, `updated`, `deleted`,
   `not_found`), `_shards` (`total: 1, successful: 1, failed: 0`), `_seq_no`,
   `_primary_term` (always 1).

**Group commit.** Writes that arrive while a sync is in progress are appended and
covered by the next single sync. One `_bulk` request is one batch: all its valid
items are appended and synced once. Each item still gets its own result; an
invalid item fails alone.

**Sync policy.** The only policy in the MVP is the reference default `request`:
every acknowledged write is synced first. `index.translog.durability: async` is
rejected as unsupported. On macOS, `File::sync_data` must reach stable storage
(`F_FULLFSYNC`); this is verified in the implementation and documented if not.

**Sync failure.** If a WAL write or sync fails, the writer cannot know what
reached the disk. The index switches to a **failed** state: pending and new
writes get an error (`503`, type to align with the reference), searches of
already committed data keep working, and a restart runs recovery. Retrying a
failed `fsync` is never treated as success.

### WAL record format

Little-endian, one record per operation:

```
offset  size  field
0       4     length      u32, bytes from `version` to the end of `payload`
4       4     crc32c      over the bytes from `version` to the end of `payload`
8       1     version     record format version (1)
9       1     op          1 = index, 2 = delete
10      8     seq_no      u64
18      8     doc_version u64 (the `_version` assigned)
26      2     id_len      u16
28      n     id          UTF-8 document `_id`
28+n    …     payload     index: the `_source` JSON bytes as received; delete: empty
```

- Files roll over at 64 MiB; a new file is named after its first `seq_no`.
- A record is never split across files.
- Index metadata changes are not WAL records (see lifecycle above).

### Refresh, flush and checkpoints

- **Checkpoint.** Every Tantivy commit is prepared with
  `prepare_commit()` and given a payload with the highest applied `seq_no`
  (`{"checkpoint": 4211}`) before `commit()`. Tantivy writes the payload into
  `meta.json` atomically with the commit, so the checkpoint can never disagree with
  the committed segments.
- **Refresh** = commit with checkpoint, then reload the **search reader**. Runs
  every `index.refresh_interval` (default `1s`; `-1` disables scheduled
  refreshes), on `POST /{index}/_refresh`, and on writes with `refresh=true`. A
  scheduled refresh is skipped when nothing was applied since the last commit.
- **Flush** = commit with checkpoint, without reloading the search reader. Runs
  when the WAL since the last checkpoint exceeds 512 MiB or the version map
  exceeds its memory budget, so `refresh_interval: -1` cannot make the WAL or
  memory grow without bound.
- **Realtime reader.** A second reader is reloaded after every commit (refresh or
  flush). It serves `GET /{index}/_doc/{id}` and existence checks together with
  the version map; it is never used for search. This keeps GET realtime without
  changing what search sees.
- **Version map pruning.** After a commit with checkpoint `C`, entries with
  `seq_no <= C` are removed: the realtime reader now holds them.
- **WAL truncation.** After a commit with checkpoint `C`, every WAL file whose
  records all have `seq_no <= C` is deleted.

**Refresh parameter on writes.**

| `refresh` | Behavior before the response |
| --- | --- |
| `false` (default) | Durable; visible after the next refresh. |
| `true` | Durable, then a refresh runs; the write is searchable when the response arrives. |
| `wait_for` | Durable, then waits until a refresh covers the write's `seq_no`. With refreshes disabled (`-1`), forces one. To be verified against both references and recorded in the contract. |

### Realtime GET

`GET /{index}/_doc/{id}`: look up the version map; a hit returns the stored
source (or `found: false` for a tombstone). A miss falls back to the realtime
reader. `_source` is stored in Tantivy as a stored bytes field together with
`_id`, `_version` and `_seq_no` (fast fields), so a committed document can be
returned exactly as indexed.

### Recovery

On startup, for each index:

1. Open the Tantivy index and read the checkpoint `C` from the commit payload
   (`0` if there is no commit yet).
2. Read the WAL files in `seq_no` order and re-apply every record with
   `seq_no > C` through the normal apply step (steps 5 and 6 of the write path),
   without syncing again.
3. **Torn tail:** a record that is incomplete or fails its CRC **at the end of the
   last file** is an interrupted append that was never acknowledged; truncate the
   file there and continue.
4. **Corruption elsewhere** (bad CRC before the end, a gap in `seq_no`, a missing
   file): stop, mark the index failed and log the exact file and offset. Never
   skip records silently.
5. Commit (checkpoint = last replayed `seq_no`) and reload both readers.
6. Only after all indexes are recovered does `GET /_cluster/health` report
   `green` and do writes get accepted. Until then the server answers with a
   documented "not ready" error.

Replaying a record twice is harmless, because applying an index operation is
`delete_term(_id)` plus `add_document` and the `_version` comes from the record.

A complete record that reached the disk but whose request was never answered
(the process died between the append and the response) is replayed. This is
allowed: a client that got no response cannot assume the write failed, the same
contract as the reference API. Only acknowledged writes are guaranteed.

### Phase 2 HTTP surface

| Route | Behavior |
| --- | --- |
| `PUT /{index}` | Create index (settings and mappings subset); 200 with `acknowledged`, `shards_acknowledged`, `index`. |
| `GET /{index}`, `HEAD /{index}` | Settings and mappings as accepted; 404 `index_not_found_exception`. |
| `DELETE /{index}` | Delete index; `acknowledged: true`. |
| `PUT /{index}/_doc/{id}`, `POST /{index}/_doc` | Index or replace; `op_type=create` → 409 `version_conflict_engine_exception` if it exists; `refresh`. |
| `PUT /{index}/_create/{id}` | Same as `op_type=create`. |
| `GET /{index}/_doc/{id}` | Realtime get; `_source` includes/excludes; `found: false` with 404. |
| `DELETE /{index}/_doc/{id}` | Delete; `result: not_found` with 404 when missing. |
| `POST /_bulk`, `POST /{index}/_bulk` | NDJSON `index`, `create`, `delete`; per-item results; one sync; `refresh` once at the end. |
| `POST /{index}/_refresh` | Refresh; `_shards` summary. |

Exact response bodies are taken from the references during implementation and
pinned by compatibility tests. `_update`, `_update_by_query`, `_delete_by_query`,
optimistic concurrency parameters (`if_seq_no`, `if_primary_term`, `version`) and
`routing` are rejected as unsupported in Phase 2.

### Plan

Each step is one pull request with its tests and docs; each commit passes
`make check`.

1. **Storage layout and index lifecycle** (core): `DataDir`, atomic metadata
   writes, create, open, delete, startup cleanup. Tests with temp dirs. *Done.*
   The lifecycle only relies on rename and directory fsync, so its crash windows
   are tested by leaving `.creating-*` / `.deleting-*` directories behind and
   reopening; the fault-injecting file layer moves to step 2, where partial
   writes matter. Deletion removes the renamed directory synchronously until
   step 3 adds writer threads to stop first.
2. **WAL** (core): record codec, append, group sync, roll-over, reader with
   torn-tail and corruption handling, truncation. Property tests for the codec;
   fault-injection tests for partial writes.
3. **Tantivy index and recovery** (core): add `tantivy` (pinned); schema from the
   Phase 2 mapping subset plus `_id`, `_source`, `_version`, `_seq_no`; writer
   thread; commit payload checkpoint; replay on open.
4. **Visibility** (core): version map, realtime reader, refresh scheduler, flush
   thresholds, `refresh=true|wait_for`.
5. **HTTP routes** (server): the table above, error types from the references,
   `GET /_cluster/health` readiness.
6. **Crash tests**: a test harness that runs the real binary, writes, kills it with
   `SIGKILL` at chosen points, restarts it and checks every acknowledged write.
7. **Compatibility checks** for the Phase 2 routes against Elasticsearch 9.5.4 and
   OpenSearch 3.9.0 (in `eloisesearch-integration` once it exists), and the
   contract updated to Supported where they pass.

## Constraints

- **Not in scope:** replication, snapshots and backups through the API,
  `translog.durability: async`, update APIs, by-query APIs, optimistic
  concurrency parameters, multiple shards, cross-index transactions.
- **Must not:** acknowledge a write before the WAL sync returns; skip or reorder
  WAL records during recovery; resurrect a deleted or older version after a
  restart; expose Tantivy types outside `eloisesearch-core`; add tokio, axum or
  HTTP types to the core.
- **Single storage device:** these guarantees survive process crashes and power
  loss on storage that honors sync requests. They do not survive losing or
  corrupting the only disk; that needs replication or backups, which are out of
  scope.
- **Compatibility:** response shapes and error types follow Elasticsearch 9.5.4;
  differences with OpenSearch 3.9.0 are recorded in `api-contract.md`.

## Validation

- [ ] Codec: every record round-trips; any single-byte change fails the CRC
      (property tests).
- [ ] An acknowledged write survives graceful restart, `SIGKILL` and a simulated
      loss of unsynced data (fault-injecting file layer), without an explicit
      refresh.
- [ ] A write that was not acknowledged because the process died during append
      leaves at most a torn tail, which recovery truncates.
- [ ] Corruption in the middle of the WAL stops recovery for that index with a
      clear error; other indexes still open.
- [ ] With default refresh, a new document is not searchable immediately and is
      after the interval; `refresh=true` and `refresh=wait_for` make it searchable
      before their response.
- [ ] `GET /_doc/{id}` returns the latest acknowledged version before any refresh,
      and `found: false` after a delete.
- [ ] Replace and delete followed by `SIGKILL` and restart return the latest
      version only; no older version reappears.
- [ ] `_bulk` reports item-level failures; valid items are durable after one
      sync; `refresh` at the end makes them searchable.
- [ ] WAL files are deleted after a commit covers them; with `refresh_interval:
      -1`, flushes keep the WAL under its threshold.
- [ ] A failed WAL sync puts the index in the failed state; no later write is
      acknowledged until restart.
- [ ] Create and delete of an index are atomic across `SIGKILL` (no half-created
      or half-deleted index visible after restart).
- [ ] The Phase 2 routes return the reference response shapes and error types,
      checked against Elasticsearch 9.5.4 and OpenSearch 3.9.0.

## Open questions

- Error `type` and status for a failed index and for "not ready" (align with the
  references during step 5).
- Exact `wait_for` behavior with refreshes disabled in both references.
- Default memory budget for the version map and the Tantivy writer heap.
