# syntax=docker/dockerfile:1

ARG RUST_VERSION=1.99

FROM rust:${RUST_VERSION}-bookworm AS builder
ARG BINARY
WORKDIR /src

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked --bin "${BINARY}" && \
    cp "target/release/${BINARY}" /usr/local/bin/service

FROM debian:bookworm-slim AS runtime
ARG BINARY
ENV BINARY=${BINARY}

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*
RUN useradd --system --uid 10001 --create-home kamino

COPY --from=builder /usr/local/bin/service /usr/local/bin/service

USER kamino
EXPOSE 8081 8082

HEALTHCHECK --interval=10s --timeout=3s --start-period=10s --retries=5 \
    CMD curl -fsS "http://127.0.0.1:${PORT:-8081}/health" || exit 1

ENTRYPOINT ["/usr/local/bin/service"]
