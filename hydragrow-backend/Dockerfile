# Rust 1.85+ is required because the project uses Rust edition 2024.
FROM rust:1.85-bookworm AS builder

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Backend depends on the sibling hydragrow-shared crate.
COPY hydragrow-shared ./hydragrow-shared
COPY hydragrow-backend ./hydragrow-backend

WORKDIR /app/hydragrow-backend
RUN cargo build --release

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/hydragrow-backend/target/release/hydragrow-backend /usr/local/bin/hydragrow-backend
COPY hydragrow-backend/migrations /migrations

ENV RUST_LOG=info
ENV SERVER_HOST=0.0.0.0
ENV SERVER_PORT=8080

EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/hydragrow-backend"]
