# Contributing to eloisesearch

Thanks for helping. This page is the gate every change passes. The rules behind
it, and the invariants you must not break, are in [`AGENTS.md`](AGENTS.md).

## Dev setup

- Rust: installed automatically from `rust-toolchain.toml` by rustup. MSRV is 1.85.
- Docker (optional): only for `make docker-*`.

```bash
make run     # http://127.0.0.1:7700
make help    # all targets
```

The [workspace repository](https://github.com/eloisesearch-foundation/eloisesearch-workspace)
clones every Eloisesearch repository side by side and opens them in one VS Code
window or a Codespace.

## Before you open a PR

```bash
make check   # fmt --check, clippy -D warnings, tests, changelog check
```

CI runs the same commands, plus the MSRV build, `cargo audit` and the Docker image
build.

## Checklist

- [ ] `make check` passes locally.
- [ ] Every new guarantee has a test that fails without the change.
- [ ] User-visible changes have a `CHANGELOG.md` entry under `## [Unreleased]`.
- [ ] Docs updated in the same PR (`docs/`, README) when behavior, API,
      configuration or deployment changed.
- [ ] Non-trivial features link their `docs/design-<topic>.md`.
- [ ] `docs/api-contract.md` updated when a route, mapping or query feature changed
      status; compatible behavior has a test against the reference.
- [ ] Third-party product names only in compatibility contexts (see `AGENTS.md`).

## Design docs

Larger work starts with `docs/design-<topic>.md`, written before the code, in four
parts:

1. **Objective:** the problem, for whom, and the use case it unlocks.
2. **Method:** current state (verified against the code) and the proposed design.
3. **Constraints:** non-goals, invariants, what must not happen.
4. **Validation:** the tests and acceptance criteria that prove it works.

From the workspace: `make docs-new-design NAME=<topic>`.

## CHANGELOG

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). Write entries for
users: what changed for them, in the past tense. `scripts/check-changelog.sh`
rejects a missing `[Unreleased]` section and repeated category headings within a
section. Released sections are never edited.

## Commits

Use [Conventional Commits](https://www.conventionalcommits.org/) prefixes
(`feat:`, `fix:`, `docs:`, `chore:`, `test:`, `refactor:`). One logical change per
commit.
