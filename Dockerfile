FROM rust:1.84-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/aegisgate

COPY Cargo.toml Cargo.lock ./
COPY crates/aegis-common/Cargo.toml crates/aegis-common/
COPY crates/aegis-proxy/Cargo.toml crates/aegis-proxy/

RUN mkdir -p crates/aegis-common/src && echo "pub fn main() {}" > crates/aegis-common/src/lib.rs
RUN mkdir -p crates/aegis-proxy/src && echo "fn main() {}" > crates/aegis-proxy/src/main.rs

RUN cargo build --release

COPY crates/ ./crates/
COPY config/ ./config/

RUN rm -f target/release/deps/aegis*
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /usr/src/aegisgate/target/release/aegis-proxy /app/aegis-proxy
COPY --from=builder /usr/src/aegisgate/config /app/config

EXPOSE 8080 9090

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:9090/health || exit 1

ENTRYPOINT ["/app/aegis-proxy"]
