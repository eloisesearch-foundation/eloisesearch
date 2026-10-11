# eloisesearch

Full-text search for your application, without running a search cluster.

Your users type `"spider man" -homecoming`, pick a genre, ask for movies rated 7+,
and expect relevant results, facets, highlighted matches and suggestions as they
type. Eloisesearch gives a Rails, Node or any HTTP-speaking application exactly that,
from a single binary with a documented, versioned HTTP/JSON API, and keeps your
indexes on local disk so they survive restarts.

> **Status:** early development. Phase 1 (service foundation) is done; indexing,
> persistence and search are next. The API is not stable yet. See the
> [roadmap](https://github.com/eloisesearch-foundation/eloisesearch-workspace/blob/main/docs/ROADMAP-MVP.md).

## Who it is for

- Application developers who need product search (catalogs, content, records),
  not log analytics.
- Teams that want one process and one data directory to operate, with behavior
  they can read in a contract and verify with tests.
- Rails and JavaScript apps first: official clients are planned in
  `eloisesearch-ruby` and `eloisesearch-javascript`.

## What it does (MVP target)

| Your users need | Eloisesearch provides |
| --- | --- |
| Broad results for plain words | OR matching across weighted fields (title above description) |
| Exact phrases and exclusions | `"quoted phrases"`, `-term`, `+` as AND, prefix `term*`, fuzzy `term~1` |
| Filters that do not distort ranking | Keyword and numeric filters (genre, minimum rating) |
| Navigation | Facet counts, sorting, pagination |
| Search-as-you-type | Prefix suggestions with accent folding |
| To see why a result matched | Highlighted fragments |
| Data that survives a crash | Durable writes with near-real-time visibility |

Today the server answers `GET /health`; the rest is being built in the order of
the roadmap. The full contract is in [`docs/api-contract.md`](docs/api-contract.md).

## Quick start

From source (Rust is installed automatically through `rust-toolchain.toml`):

```bash
git clone https://github.com/eloisesearch-foundation/eloisesearch.git
cd eloisesearch
make run                       # http://127.0.0.1:7700, data in ./data
curl localhost:7700/health     # {"status":"ok","version":"0.1.0"}
```

With Docker:

```bash
make docker-up                 # http://localhost:7700, data in a named volume
make docker-down
```

Prebuilt binaries will be published with the first tagged release.

## Reference guides

- [Configuration](docs/configuration.md): environment variables and defaults
- [API contract](docs/api-contract.md): routes, search features, error model
- [Architecture](docs/ARCHITECTURE.md): how the code is shaped and why
- [Design decisions](docs/design-decisions.md): what we chose and the trade-offs
- [Quality plan](docs/quality-plan.md): how correctness, relevance and performance are validated
- [Contributing](CONTRIBUTING.md): the gate every change passes
- [Changelog](CHANGELOG.md)

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for
inclusion in this project by you, as defined in the Apache-2.0 license, shall be dual
licensed as above, without any additional terms or conditions.
