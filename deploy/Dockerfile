FROM rust:1-bookworm AS builder
WORKDIR /app/server
COPY server/Cargo.toml ./Cargo.toml
COPY server/Cargo.lock ./Cargo.lock
COPY server/src ./src
RUN cargo build --locked --release --bins

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 app
COPY --from=builder /app/server/target/release/api /usr/local/bin/api
COPY --from=builder /app/server/target/release/expedition-worker /usr/local/bin/expedition-worker
USER app
EXPOSE 3000
ENTRYPOINT ["/usr/local/bin/api"]
