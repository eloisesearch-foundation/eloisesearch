# AGENTS.md — eloisesearch contributor guide

This is the single canonical instruction file for AI coding agents working in this
repository (Claude Code, Codex, Cursor, Gemini CLI and other AGENTS-aware tools).
`CLAUDE.md` only imports it. Humans should read it too.

## What this project is for

Eloisesearch lets application developers add full-text search to their product
without operating a search cluster: one binary, one data directory. Its API targets a
documented subset of the Elasticsearch 9.5.4 REST API and Query DSL, checked
against Elasticsearch and OpenSearch by compatibility suites before an item is
marked supported, so teams can migrate existing applications and tools by
pointing them at Eloisesearch. The first use case it must serve end to end is a movie-search app
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
3. **One error shape, the reference one.** Every non-2xx response is
   `{"error":{"root_cause":[...],"type","reason"},"status"}`, built through
   `ApiError`, including middleware failures such as timeouts. Where the reference
   API has an equivalent error, use its `type` and status. Covered by
   `tests/http.rs`.
4. **Validate at the boundary, once.** Names, IDs and parameters are parsed into
   typed values (`IndexName`, `Config`) and reused. Index names are directory
   names; their rules exist to keep paths safe.
5. **Compatible or explicit.** A supported route or query feature behaves like the
   reference (Elasticsearch 9.5.4) for the documented subset; anything else fails
   with a documented error. Never silently ignore a parameter or query construct,
   and never answer with different semantics. Update
   [`docs/api-contract.md`](docs/api-contract.md) with every change.
6. **Acknowledged writes are durable** (from Phase 2): success is returned only
   after the write is persisted; visibility follows the refresh rules in
   [`docs/design-decisions.md`](docs/design-decisions.md).
7. **Tests are hermetic.** No real `$HOME`, no fixed ports, no network. Use
   `Config::from_lookup` instead of the process environment, temp dirs for data,
   and the router in-process (`tower::ServiceExt::oneshot`).
8. **Routes stay at the root.** Compatible routes use the reference paths (no
   `/v1`). Eloisesearch-only endpoints go under `/_eloisesearch/` or are marked
   as such in the contract (like `GET /health`).

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
- **Third-party names only where compatibility needs them.** Elasticsearch and
  OpenSearch may be named in the compatibility matrix, the migration guide,
  compatibility tests and code comments that explain a compatibility rule, always
  descriptively ("compatible with a subset of ..."), never in marketing
  comparisons, and with the trademark notice in user-facing docs. No logos, no
  competitor research or benchmarks against other products in this repository.
- **Never impersonate another product.** Do not send headers or identifiers that
  present Eloisesearch as another vendor's product (for example
  `X-Elastic-Product`).
- **The human decides.** Agents review, audit and draft; the maintainer approves
  merges, version bumps and tags. Never bump versions or tag without explicit
  approval.
- **Do not trust a PR description.** Review the diff itself: regressions, tests
  for every guarantee, docs and CHANGELOG updated.

## Git workflow: branches, grouped commits, pull requests

Every roadmap item (or a coherent part of one) is delivered on its own branch and
merged through a pull request. Nothing is committed directly to `main`.

1. **Branch from an up-to-date `main`**, named `<type>/<topic>`:
   `feat/index-lifecycle`, `docs/design-persistence`, `fix/timeout-envelope`,
   `ci/meta-repo-checks`. Types follow Conventional Commits: `feat`, `fix`, `docs`,
   `test`, `refactor`, `ci`, `chore`.
2. **Commit in logical groups, not one big commit.** Each commit is one reviewable
   step with a Conventional Commits message, for example:
   `docs: design for persistence` → `test: failing lifecycle tests` →
   `feat(core): durable write log` → `feat(server): document routes` →
   `docs: contract and CHANGELOG`. Every commit builds and passes the repository
   gate (`make check`), so history stays bisectable. Clean up `wip`/`fixup`
   commits before asking for review.
3. **Open a pull request** to `main` with the repository's PR template
   (Objective, Method, Constraints, Validation). Its title is a Conventional
   Commits summary; its body links the roadmap item and the design doc. Docs,
   `CHANGELOG.md` and tests ship in the same PR. CI must be green.
4. **Roadmap checkboxes** live in `eloisesearch-workspace/docs/ROADMAP-MVP.md`.
   Work in another repository ticks its items in a companion workspace PR that
   links the product PR; work inside the workspace ticks them in the same PR.
5. **Review and merge belong to the maintainer.** Merge without squashing
   (merge commit or rebase) so the grouped commits survive; delete the branch
   afterwards. Work that spans repositories is one PR per repository, each
   linking the others.

Agents may create branches, commit and open pull requests (as drafts unless told
otherwise) when asked. They never push to `main`, merge, force-push a branch
someone else is using, tag or bump versions without explicit approval.

## Documentation map

- [`README.md`](README.md): what it is for, quick start, reference guides.
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md): operational map of the code:
  crates, request flow, invariants, where things live.
- [`docs/api-contract.md`](docs/api-contract.md): compatibility matrix: reference
  versions, routes, mappings, Query DSL subset, error model, client support. The
  OpenAPI document will supersede the route tables once published.
- [`docs/design-decisions.md`](docs/design-decisions.md): why the system is
  shaped this way: engine, transport, API compatibility, durability and
  visibility.
- [`docs/design-persistence.md`](docs/design-persistence.md): storage layout,
  WAL, durability, refresh and flush, realtime GET, recovery (Phase 2). Read it
  before touching anything that writes to the data directory.
- [`docs/quality-plan.md`](docs/quality-plan.md): fixtures, relevance
  evaluation, benchmarks, release gate.
- [`docs/configuration.md`](docs/configuration.md): every environment variable.
- [`CONTRIBUTING.md`](CONTRIBUTING.md): the pre-PR gate and checklist.
- [`CHANGELOG.md`](CHANGELOG.md): user-visible changes.
