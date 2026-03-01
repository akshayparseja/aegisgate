# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned
- TLS/mTLS support for client connections
- Persistent rate limit state (Redis backend)
- IPv6 support for rate limiting
- WebSocket support for MQTT
- eBPF fast-path filtering (Linux kernel 5.x+)
- ML-based anomaly detection pipeline
- Connection pooling to backend brokers
- Multi-backend load balancing
- Performance benchmarks and metrics
- Comprehensive integration test suite

## [0.1.0-alpha] - 2025-03-02

### Status
**Alpha Release** - Early development stage. APIs and configuration may change. Not recommended for production use.

### Added
- Initial alpha release
- Per-IP rate limiting with token bucket algorithm
  - Configurable burst capacity and refill rates
  - Automatic cleanup of inactive IP state
- Multi-layer Slowloris attack detection and prevention
  - First packet timeout enforcement
  - Idle timeout between bytes
  - Protocol-specific timeouts (MQTT CONNECT, HTTP headers)
- HTTP protocol detection and rejection
  - Prevents protocol confusion attacks on MQTT port
  - HTTP header size and count limits
- MQTT CONNECT packet validation
  - Deep packet inspection of fixed headers
  - Remaining Length validation with configurable limits
  - Protocol name/version validation (MQTT 3.1/3.1.1)
- Bidirectional TCP proxying with zero-copy streaming
- Prometheus metrics endpoint (`/metrics`)
  - Active connection counter
  - Rate limit rejection counter
  - HTTP protocol rejection counter
  - Slowloris detection counter
  - MQTT protocol validation rejection counter
- Health check endpoint (`/health`)
- Structured JSON logging with configurable verbosity
- Feature toggles for all security subsystems
- Docker and docker-compose deployment support
- Container health checks and readiness probes
- Graceful shutdown on SIGINT/SIGTERM
- YAML-based configuration
- Basic test suite (15 unit/integration tests)

### Documentation
- README with architecture overview and deployment guides
- CONTRIBUTING.md with commit conventions and DCO requirements
- CODE_OF_CONDUCT.md
- Apache 2.0 LICENSE
- Configuration examples
- Integration test scripts

### Known Limitations
- No TLS/SSL support (use reverse proxy for TLS termination)
- In-memory rate limit state (lost on restart)
- No authentication layer (relies on upstream MQTT broker)
- IPv4 only for rate limiting
- Limited test coverage
- No performance benchmarks available yet

---

## Version Scheme

- **Alpha (0.x.x-alpha)**: Early development, expect breaking changes
- **Beta (0.x.x-beta)**: Feature complete for initial release, stabilization phase
- **Stable (x.y.z)**: Production-ready releases
  - Major (x.0.0): Breaking changes
  - Minor (0.y.0): New features, backwards compatible
  - Patch (0.0.z): Bug fixes, security patches

[Unreleased]: https://github.com/YOUR_USERNAME/aegisgate/compare/v0.1.0-alpha...HEAD
[0.1.0-alpha]: https://github.com/YOUR_USERNAME/aegisgate/releases/tag/v0.1.0-alpha