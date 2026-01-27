# AegisGate

A high-performance, asynchronous MQTT proxy and security gateway built in Rust. 

## Core Features
* **Per-IP Rate Limiting:** Distributed Token Bucket algorithm for granular traffic shaping.
* **Protocol Gatekeeper:** Deep packet inspection (DPI) to enforce MQTT connection standards.
* **Resource Management:** Automated background GC for state stability.

## Configuration
Settings are managed via `config/aegis_config.yaml`.

### Security Parameters
| Parameter | Description |
| :--- | :--- |
| `max_tokens` | Burst capacity per unique source. |
| `refill_rate` | Sustained throughput allowance. |
| `cleanup_interval_secs` | Metadata pruning frequency. |

## Technical Architecture
AegisGate implements a multi-stage validation pipeline:
1. **Transport Validation:** IP-based filtering via concurrent sharded state.
2. **Application Validation:** MQTT fixed-header verification before proxying.
3. **High-Throughput Tunneling:** Zero-copy bidirectional streaming to upstream brokers.

## Development

### Prerequisites
* Rust 1.75+
* Cargo
* Make

### Tooling & Commands
A `Makefile` is provided to encapsulate common development tasks.

| Command | Description |
| :--- | :--- |
| `make build` | Compiles the project in release mode. |
| `make run` | Starts the AegisGate proxy service. |
| `make test` | Executes the test suite across the workspace. |
| `make clean` | Removes build artifacts and target directories. |

### Manual Execution
If `make` is unavailable, use standard Cargo commands:
```bash
cargo build --release
cargo run -p aegis-proxy
