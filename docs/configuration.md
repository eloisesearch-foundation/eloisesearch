# Configuration

Eloisesearch is configured entirely through environment variables. Every
variable is optional. Invalid values stop the server at startup with exit code 2
and a message naming the variable, the value and the expected format.

| Variable | Default | Description |
| --- | --- | --- |
| `ELOISESEARCH_HOST` | `127.0.0.1` | IP address to listen on. Use `0.0.0.0` inside containers. Must be an IP, not a hostname. |
| `ELOISESEARCH_PORT` | `7700` | TCP port, `0`–`65535`. |
| `ELOISESEARCH_DATA_DIR` | `./data` | Where indexes and recovery data live. Created at startup and locked while the server runs; one server per directory. **Must be persistent storage**; an ephemeral container filesystem loses data on restart. |
| `ELOISESEARCH_LOG_FORMAT` | `text` | `text` for development, `json` (one object per line) for containers and log pipelines. |
| `ELOISESEARCH_REQUEST_TIMEOUT_SECS` | `30` | Maximum duration of a request; slower requests get HTTP 408. Positive integer. |
| `RUST_LOG` | `info` for Eloisesearch, `warn` for dependencies | Log verbosity, in [`EnvFilter`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html) syntax, e.g. `RUST_LOG=debug` or `RUST_LOG=eloisesearch_server=debug,tower_http=debug`. |

## Local development

Copy `.env.example` to `.env` and edit it. `make run` loads `.env` automatically;
the binary itself does not read `.env` files.

## Docker

The image sets `ELOISESEARCH_HOST=0.0.0.0`, `ELOISESEARCH_PORT=7700`,
`ELOISESEARCH_DATA_DIR=/var/lib/eloisesearch` and `ELOISESEARCH_LOG_FORMAT=json`,
and declares `/var/lib/eloisesearch` as a volume. `compose.yaml` mounts the named
volume `eloisesearch-data` there and publishes `${ELOISESEARCH_PORT:-7700}`.
