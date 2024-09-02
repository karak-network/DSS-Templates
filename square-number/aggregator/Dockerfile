FROM rust:1.80.1-slim AS builder

WORKDIR /app

COPY . .

WORKDIR /app/aggregator
RUN apt-get update && apt-get install -y pkg-config libssl-dev
RUN cargo build --release --bin square-number-dss-aggregator

FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/square-number-dss-aggregator /app/square-number-dss-aggregator
COPY --from=builder /app/contracts /app/contracts

ENTRYPOINT ["/app/square-number-dss-aggregator"]
