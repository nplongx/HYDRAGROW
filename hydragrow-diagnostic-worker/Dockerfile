# Build from the repository root so path dependencies resolve.
FROM rust:1.89-bookworm AS builder
WORKDIR /src

COPY hydragrow-shared ./hydragrow-shared
COPY hydragrow-supervisor-query ./hydragrow-supervisor-query
COPY hydragrow-diagnostic-worker ./hydragrow-diagnostic-worker

RUN cargo build --release --manifest-path hydragrow-diagnostic-worker/Cargo.toml

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /src/target/release/hydragrow-diagnostic-worker /usr/local/bin/hydragrow-diagnostic-worker

ENV RUST_LOG=info
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/hydragrow-diagnostic-worker"]
