# syntax=docker/dockerfile:1

# Build from the repository root because hydragrow-backend depends on
# the sibling hydragrow-shared crate.
FROM rust:1-bookworm AS builder

WORKDIR /app

# Copy manifests first to maximize Docker layer caching.
COPY hydragrow-backend/Cargo.toml hydragrow-backend/Cargo.toml
COPY hydragrow-shared/Cargo.toml hydragrow-shared/Cargo.toml

# Copy source required to resolve and build the workspace-local dependency.
COPY hydragrow-shared hydragrow-shared
COPY hydragrow-backend hydragrow-backend

WORKDIR /app/hydragrow-backend
RUN cargo build --release --locked 2>/dev/null || cargo build --release

# Runtime image: no Rust toolchain or build sources.
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/hydragrow-backend/target/release/hydragrow-backend /usr/local/bin/hydragrow-backend

ENV RUST_LOG=info
ENV PORT=8080

EXPOSE 8080

USER 65532:65532

ENTRYPOINT ["/usr/local/bin/hydragrow-backend"]
