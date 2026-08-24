FROM rust:1.82-slim AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY hydragrow-shared ./hydragrow-shared
COPY hydragrow-backend ./hydragrow-backend
WORKDIR /app/hydragrow-backend
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/hydragrow-backend/target/release/hydragrow-backend /usr/local/bin/hydragrow-backend
COPY hydragrow-backend/migrations /migrations
EXPOSE 8080
CMD ["hydragrow-backend"]
