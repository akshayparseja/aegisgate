FROM rust:1.84-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/aegisgate

COPY Cargo.toml Cargo.lock ./
COPY crates/aegis-common/Cargo.toml crates/aegis-common/
COPY crates/aegis-proxy/Cargo.toml crates/aegis-proxy/

# Copy full crate sources and configuration before building so cargo sees the real sources
# and dependency graph within the builder. This avoids placeholder-source caching issues
# and ensures the image builds reliably.
COPY crates/ ./crates/
COPY config/ ./config/

# Build the release binary
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
