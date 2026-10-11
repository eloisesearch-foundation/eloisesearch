# syntax=docker/dockerfile:1

FROM rust:1.97-slim-bookworm AS build
WORKDIR /src
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked --bin eloisesearch \
 && cp target/release/eloisesearch /usr/local/bin/eloisesearch \
 && mkdir -p /data

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=build /usr/local/bin/eloisesearch /usr/local/bin/eloisesearch
COPY --from=build --chown=nonroot:nonroot /data /var/lib/eloisesearch
ENV ELOISESEARCH_HOST=0.0.0.0 \
    ELOISESEARCH_PORT=7700 \
    ELOISESEARCH_DATA_DIR=/var/lib/eloisesearch \
    ELOISESEARCH_LOG_FORMAT=json
VOLUME ["/var/lib/eloisesearch"]
EXPOSE 7700
USER nonroot
ENTRYPOINT ["/usr/local/bin/eloisesearch"]
