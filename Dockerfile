FROM rust:slim-bullseye AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev make g++
COPY . /app
RUN cargo build --release

FROM debian:bullseye-slim
WORKDIR /app
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/vrf-oracle /app/vrf-oracle
ENV RUST_LOG=info
CMD ["./vrf-oracle"]
