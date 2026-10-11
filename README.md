# eloisesearch

A search engine for application developers, written in Rust on top of
[Tantivy](https://docs.rs/tantivy) and exposed through a versioned HTTP/JSON API.

> **Status:** early development (MVP Phase 1 — service foundation). The API is not stable.

## Repository layout

```
eloisesearch/
├── crates/
│   ├── eloisesearch-core/     # index lifecycle, schema, documents, queries — no HTTP
│   └── eloisesearch-server/   # HTTP/JSON server (axum), config, logging, error model
├── .github/workflows/ci.yml   # fmt, clippy, tests, MSRV, Docker build
├── Dockerfile                 # distroless, non-root image
├── compose.yaml               # local service with a persistent volume
└── Makefile
```

The core never depends on the transport: the server translates HTTP requests into
calls on `eloisesearch-core`, and Tantivy types are not part of any public API.

## Getting started

Requirements: Rust 1.85+ (installed automatically from `rust-toolchain.toml` via rustup).

```bash
make run                     # http://127.0.0.1:7700, data in ./data
curl localhost:7700/health   # {"status":"ok","version":"0.1.0"}
```

With Docker:

```bash
make docker-up               # http://localhost:7700, data in the eloisesearch-data volume
make docker-down
```

`make help` lists all targets; `make check` runs what CI runs.

## Configuration

| Variable | Default | Description |
| --- | --- | --- |
| `ELOISESEARCH_HOST` | `127.0.0.1` (`0.0.0.0` in Docker) | Listen address |
| `ELOISESEARCH_PORT` | `7700` | Listen port |
| `ELOISESEARCH_DATA_DIR` | `./data` (`/var/lib/eloisesearch` in Docker) | Index and recovery data; must be persistent storage |
| `ELOISESEARCH_LOG_FORMAT` | `text` (`json` in Docker) | `text` or `json` |
| `ELOISESEARCH_REQUEST_TIMEOUT_SECS` | `30` | Per-request timeout |
| `RUST_LOG` | `info` for Eloisesearch | Log verbosity ([`EnvFilter`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html) syntax) |

Invalid values stop the server at startup with exit code 2.

## HTTP API

| Method | Path | Description |
| --- | --- | --- |
| `GET` | `/health` | Liveness: `{"status":"ok","version":"…"}` |

Every error uses the same shape; `type` is stable and machine-readable:

```json
{ "error": { "type": "not_found", "reason": "no route for GET /nope" }, "status": 404 }
```

The full contract will be published as OpenAPI as endpoints are added
(see the API Contract & Feature Matrix in the project docs).

## Development

```bash
make fmt      # format
make lint     # clippy (pedantic), warnings are errors
make test     # unit + HTTP tests (in-process, no socket)
```

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for
inclusion in this project by you, as defined in the Apache-2.0 license, shall be dual
licensed as above, without any additional terms or conditions.
