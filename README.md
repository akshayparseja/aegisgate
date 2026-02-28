# AegisGate

A high-performance, security-focused MQTT proxy gateway built in Rust with production-grade protection mechanisms.

## Overview

AegisGate sits between MQTT clients and brokers, providing multi-layered security inspection, rate limiting, and protocol validation. Built with async Rust (Tokio), it delivers low-latency proxying while defending against common attack vectors.

## Features

### Security & Protection

- **Per-IP Rate Limiting**: Token bucket algorithm with configurable burst capacity and refill rates
- **Slowloris Attack Detection**: Multi-layer timeout enforcement to prevent slow-data attacks
- **HTTP Protocol Rejection**: Detects and blocks HTTP traffic targeting MQTT ports
- **MQTT Protocol Validation**: Deep packet inspection of MQTT CONNECT packets
- **Connection Resource Management**: Automatic cleanup of idle connections and expired rate limit state

### Protocol Support

- **MQTT 3.1/3.1.1**: Full CONNECT packet validation
- **Protocol Detection**: Distinguishes between MQTT, HTTP, and malformed traffic
- **Bidirectional Proxying**: Zero-copy streaming between clients and upstream brokers

### Observability

- **Prometheus Metrics**: Real-time connection statistics, rejection counters, and performance metrics
- **Structured Logging**: JSON-formatted logs with configurable verbosity
- **Health Monitoring**: Container health checks and readiness probes

## Architecture

AegisGate implements a multi-stage validation pipeline:

1. **Rate Limiting Layer**: Per-IP token bucket filtering using concurrent hashmap
2. **Slowloris Protection**: Protocol-agnostic timeout enforcement on initial connection
3. **HTTP Detection**: Quick byte pattern matching followed by full HTTP parsing with size limits
4. **MQTT Validation**: Fixed header inspection and CONNECT packet verification
5. **Proxy Layer**: Bidirectional TCP stream forwarding to upstream broker

## Quick Start

### Using Docker Compose

```bash
# Start AegisGate and EMQX broker
docker-compose up -d

# View logs
docker-compose logs -f aegis-proxy

# Access metrics
curl http://localhost:9090/metrics
```

### Manual Build

```bash
# Build release binary
cargo build --release --manifest-path crates/aegis-proxy/Cargo.toml

# Run with default config
./target/release/aegis-proxy
```

## Configuration

Configuration is managed via `config/aegis_config.yaml`.

### Proxy Settings

```yaml
proxy:
  listen_address: "0.0.0.0:8080"        # Proxy listening address
  target_address: "127.0.0.1:1883"      # Upstream MQTT broker
  max_connect_remaining: 65536          # Max MQTT CONNECT packet size (bytes)
```

### Rate Limiting

```yaml
limit:
  max_tokens: 5.0                       # Maximum burst capacity per IP
  refill_rate: 1.0                      # Tokens per second refill rate
  cleanup_interval_secs: 60             # State cleanup interval
  ip_idle_timeout_secs: 60              # Remove IPs idle longer than this
```

### Slowloris Protection

```yaml
slowloris_protection:
  first_packet_timeout_ms: 30000        # Time to receive first packet
  packet_idle_timeout_ms: 10000         # Max idle time between bytes
  connection_timeout_ms: 60000          # Total connection establishment timeout
  mqtt_connect_timeout_ms: 30000        # MQTT CONNECT packet timeout
  http_request_timeout_ms: 30000        # HTTP request line + headers timeout
  max_http_header_size: 8192            # Max total HTTP header size (bytes)
  max_http_header_count: 100            # Max number of HTTP headers
```

### Feature Toggles

```yaml
features:
  enable_mqtt_inspection: true          # MQTT protocol validation
  enable_mqtt_full_inspection: true     # Deep CONNECT packet inspection
  enable_http_inspection: true          # HTTP detection and rejection
  enable_slowloris_protection: true     # Timeout-based attack prevention
  enable_rate_limiter: true             # Per-IP rate limiting
  enable_ebpf: false                    # eBPF fast-path (future)
  enable_ml: false                      # ML anomaly detection (future)
```

### Metrics

```yaml
metrics:
  enabled: true                         # Enable Prometheus endpoint
  port: 9090                            # Metrics server port
```

## Metrics

AegisGate exposes Prometheus metrics on the configured metrics port (default: 9090).

### Available Metrics

- `aegis_active_connections`: Current number of active proxy connections
- `aegis_rejected_connections_total`: Total connections rejected by rate limiting
- `aegis_http_rejections_total`: Total connections rejected due to HTTP protocol detection
- `aegis_slowloris_rejections_total`: Total connections rejected due to Slowloris attacks
- `aegis_protocol_rejections_total`: Total connections rejected by MQTT validation

### Example Queries

```bash
# View all metrics
curl http://localhost:9090/metrics

# Get active connections
curl -s http://localhost:9090/metrics | grep aegis_active_connections

# Get rejection statistics
curl -s http://localhost:9090/metrics | grep rejections_total
```

## Development

### Prerequisites

- Rust 1.75 or later
- Cargo
- Docker and Docker Compose (for containerized testing)

### Project Structure

```
aegisgate/
├── crates/
│   ├── aegis-common/          # Shared configuration types
│   └── aegis-proxy/           # Main proxy implementation
│       ├── src/
│       │   ├── main.rs        # Entry point
│       │   ├── engine/        # Core proxy logic
│       │   │   ├── connection.rs   # Connection handler
│       │   │   ├── http.rs         # HTTP detection
│       │   │   ├── limiter.rs      # Rate limiting
│       │   │   └── slowloris.rs    # Timeout utilities
│       │   ├── parser/        # Protocol parsers
│       │   │   └── mqtt.rs         # MQTT packet parsing
│       │   └── metrics.rs     # Prometheus metrics
│       └── tests/             # Integration tests
├── config/
│   └── aegis_config.yaml      # Runtime configuration
├── docker-compose.yaml
├── Dockerfile
└── Makefile
```

### Building

```bash
# Development build
cargo build --manifest-path crates/aegis-proxy/Cargo.toml

# Release build
cargo build --release --manifest-path crates/aegis-proxy/Cargo.toml

# Run tests
cargo test --manifest-path crates/aegis-proxy/Cargo.toml
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test suite
cargo test --test http_tests
cargo test --test slowloris_tests

# Run with logging
RUST_LOG=debug cargo test -- --nocapture
```

### Using Make

```bash
# Build release binary
make build

# Run proxy
make run

# Run tests
make test

# Clean build artifacts
make clean
```

## Testing the Proxy

### Publish a Message

```bash
# Through proxy (port 8080)
mosquitto_pub -h localhost -p 8080 -t test/topic -m "Hello AegisGate" -q 1

# Directly to broker (port 1883)
mosquitto_pub -h localhost -p 1883 -t test/topic -m "Direct to broker" -q 1
```

### Subscribe to Messages

```bash
# Through proxy
mosquitto_sub -h localhost -p 8080 -t test/topic -v

# Directly to broker
mosquitto_sub -h localhost -p 1883 -t test/topic -v
```

### Test HTTP Rejection

```bash
# This should be rejected by HTTP detection
curl -v http://localhost:8080/
```

## Deployment

### Docker

```bash
# Build image
docker build -t aegisgate:latest .

# Run container
docker run -d \
  -p 8080:8080 \
  -p 9090:9090 \
  -v $(pwd)/config:/app/config \
  aegisgate:latest
```

### Docker Compose

```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f

# Stop services
docker-compose down
```

## Performance Considerations

- **In-Memory State**: Rate limit state is stored in-memory and cleared on restart
- **Concurrent Processing**: Uses Tokio async runtime with multi-threaded scheduler
- **Zero-Copy Proxying**: Efficient bidirectional streaming with minimal allocations
- **Connection Pooling**: Reuses backend connections when possible

## Security Best Practices

1. **Rate Limits**: Adjust `max_tokens` and `refill_rate` based on expected client behavior
2. **Timeouts**: Configure Slowloris timeouts to balance legitimate slow connections vs attacks
3. **Network Isolation**: Run AegisGate in a DMZ between untrusted clients and MQTT brokers
4. **Monitoring**: Set up alerts on rejection metrics to detect attacks
5. **Regular Updates**: Keep dependencies updated for security patches

## Troubleshooting

### High Rate Limit Rejections

Check `aegis_rejected_connections_total` metric. If legitimate clients are being blocked:
- Increase `max_tokens` for higher burst capacity
- Increase `refill_rate` for sustained higher throughput

### Slowloris False Positives

If legitimate slow connections are timing out:
- Increase `packet_idle_timeout_ms` for slower networks
- Increase protocol-specific timeouts (e.g., `mqtt_connect_timeout_ms`)

### Connection Refused

Verify:
- AegisGate is listening on expected port (`listen_address`)
- Backend broker is reachable (`target_address`)
- No firewall rules blocking connections

## License

Copyright 2024. All rights reserved.

## Contributing

This is a private project. For questions or issues, contact the maintainer.