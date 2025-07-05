FROM rust:1.85.0-bullseye AS builder
WORKDIR /25C1-redis-taceans
COPY . .
RUN cargo install --path redis-server

FROM debian:bullseye-slim AS runtime
RUN apt-get update && apt-get install -y redis-tools && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/redis_server /usr/local/bin/redis_server
ENTRYPOINT ["redis_server"]
