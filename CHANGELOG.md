# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `eloisesearch` server binary with a liveness endpoint, `GET /health`, that
  reports the running version.
- A single JSON error shape for every failed request,
  `{"error":{"type","reason"},"status"}`, including unknown routes (404) and
  unsupported methods (405).
- Configuration through `ELOISESEARCH_*` environment variables (listen address,
  data directory, log format, request timeout); invalid values stop the server
  at startup with exit code 2.
- Structured logging in text or JSON, and graceful shutdown on Ctrl+C or
  SIGTERM.
- Docker image (distroless, non-root) and a Compose file with a persistent data
  volume.
- Index name validation in the core: lowercase letters, digits, `-` and `_`,
  up to 128 bytes.

[Unreleased]: https://github.com/eloisesearch-foundation/eloisesearch/commits/main
