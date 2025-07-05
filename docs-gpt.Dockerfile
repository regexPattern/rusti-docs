FROM rust:1.85.0-bullseye AS builder
WORKDIR /25C1-redis-taceans
COPY . .
RUN cargo install --path docs-gpt

FROM debian:bullseye-slim AS runtime
COPY --from=builder /usr/local/cargo/bin/docs_gpt /usr/local/bin/docs_gpt
ENTRYPOINT ["docs_gpt"]
