FROM rust:1.88.0 AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN cargo build --release --bin poubelle

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/poubelle /usr/local/bin/poubelle

RUN mkdir -p /data

ARG POUBELLE_DATA_DIR
ARG POUBELLE_HOST
ARG POUBELLE_PORT
ARG POUBELLE_HTTP_HOST
ARG POUBELLE_HTTP_PORT
ARG POUBELLE_USERNAME
ARG POUBELLE_PASSWORD

EXPOSE 5432 3000

CMD ["poubelle"]

