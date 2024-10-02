FROM rust:1.80.1-slim AS builder

WORKDIR /app

COPY . .

WORKDIR /app/operator
RUN apt-get update && apt-get install -y pkg-config libssl-dev
RUN cargo build --release --bin square-number-dss-operator

FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/square-number-dss-operator /app/square-number-dss-operator

ENV HOST=0.0.0.0
ENV PORT=9177

ENTRYPOINT ["/app/square-number-dss-operator"]