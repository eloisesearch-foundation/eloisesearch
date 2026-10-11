# AGENTS.md — eloisesearch contributor guide

This is the single canonical instruction file for AI coding agents working in this
repository (Claude Code, Codex, Cursor, Gemini CLI and other AGENTS-aware tools).
`CLAUDE.md` only imports it. Humans should read it too.

## What this project is for

Eloisesearch lets application developers add full-text search to their product
without operating a search cluster: one binary, one data directory, a versioned
HTTP/JSON API. The first use case it must serve end to end is a movie-search app
(Rails API + Nuxt frontend): weighted text search with phrases and exclusions,
filters, facets, suggestions, highlighting and pagination, over indexes that
survive restarts.

When a design choice is unclear, prefer the option that makes that use case
simpler, more predictable or easier to verify.

## Repository layout

```
crates/
├── eloisesearch-core/     index lifecycle, schema, documents, queries. NO HTTP, NO tokio.
└── eloisesearch-server/   axum HTTP server: config, logging, routing, error model.
                           lib.rs (testable) + main.rs (binary `eloisesearch`).
docs/                      canonical documentation (see the map below).
scripts/                   repository checks (changelog).
.github/workflows/ci.yml   fmt, clippy, tests, MSRV, audit, Docker build.
Dockerfile, compose.yaml   distroless non-root image, persistent volume.
```

Sibling repositories (SDKs, integration tests, the meta-repo with the roadmap)
are described in
[`eloisesearch-workspace`](https://github.com/eloisesearch-foundation/eloisesearch-workspace).

## Commands

```bash
make check     # what CI runs: fmt --check, clippy -D warnings, tests, changelog check
make test      # unit + in-process HTTP tests
make run       # server on 127.0.0.1:7700, data in ./data
make help      # everything else
```

Run `make check` before saying a change is done. If you could not run it, say so.

## Cross-cutting invariants (do not violate)

1. **The core is transport-independent.** `eloisesearch-core` never depends on
   axum, tokio, HTTP types or the server crate. The server translates requests into
   core calls and maps core errors to HTTP.
2. **Tantivy types never cross the public API.** Not in HTTP responses, not in the
   core's public signatures once Tantivy is added.
3. **One error shape.** Every non-2xx response is
   `{"error":{"type","reason"},"status"}`, built through `ApiError`. `type` is a
   stable identifier; `reason` is for humans. Covered by `tests/http.rs`.
4. **Validate at the boundary, once.** Names, IDs and parameters are parsed into
   typed values (`IndexName`, `Config`) and reused. Index names are directory
   names; their rules exist to keep paths safe.
5. **Unsupported input fails predictably.** Never silently ignore a parameter or
   query construct the contract does not support.
6. **Acknowledged writes are durable** (from Phase 2): success is returned only
   after the write is persisted; visibility follows the refresh rules in
   [`docs/design-decisions.md`](docs/design-decisions.md).
7. **Tests are hermetic.** No real `$HOME`, no fixed ports, no network. Use
   `Config::from_lookup` instead of the process environment, temp dirs for data,
   and the router in-process (`tower::ServiceExt::oneshot`).

## Code style

- Match the surrounding code. Comments explain *why*, not *what*.
- Small, behavior-scoped changes. No speculative abstractions or public surface
  without a caller; future work goes into a `docs/design-*.md`, not into stubs.
- No `unwrap`/`expect` in runtime paths; they are fine in tests.
- `unsafe` is forbidden by workspace lint. Clippy runs with `pedantic`.

## Project maintenance rules

- **Docs and CHANGELOG move with the code.** A change to user-visible behavior,
  the API, configuration or deployment updates the relevant `docs/` page, the
  README when needed, and adds a `CHANGELOG.md` entry under `## [Unreleased]`
  (`Added`/`Changed`/`Deprecated`/`Removed`/`Fixed`/`Security`) in the same
  commit. Released sections are frozen.
- **New features start with a design doc.** Non-trivial work gets
  `docs/design-<topic>.md` first, written in four parts: Objective, Method,
  Constraints, Validation (template in the workspace: `make docs-new-design`).
- **Write about use cases and behavior.** Docs say what problem a feature solves
  and how it behaves, then how it is built.
- **No third-party product references.** Do not name other search products or
  services in code, docs, tests or commit messages. Describe behavior on its own
  terms.
- **The human decides.** Agents review, audit and draft; the maintainer approves
  merges, version bumps and tags. Never bump versions or tag without explicit
  approval.
- **Do not trust a PR description.** Review the diff itself: regressions, tests
  for every guarantee, docs and CHANGELOG updated.

## Documentation map

- [`README.md`](README.md): what it is for, quick start, reference guides.
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md): operational map of the code:
  crates, request flow, invariants, where things live.
- [`docs/api-contract.md`](docs/api-contract.md): HTTP routes, document model,
  search feature matrix, error model. The OpenAPI document will supersede the
  route tables once published.
- [`docs/design-decisions.md`](docs/design-decisions.md): why the system is
  shaped this way: engine, transport, own API contract, durability and
  visibility.
- [`docs/quality-plan.md`](docs/quality-plan.md): fixtures, relevance
  evaluation, benchmarks, release gate.
- [`docs/configuration.md`](docs/configuration.md): every environment variable.
- [`CONTRIBUTING.md`](CONTRIBUTING.md): the pre-PR gate and checklist.
- [`CHANGELOG.md`](CHANGELOG.md): user-visible changes.
