# AegisGate Benchmark Suite

Performance benchmarking for AegisGate MQTT proxy against direct EMQX connections.

## Quick Start

```bash
cd benchmark/scripts
./run_benchmark.sh
```

## Available Benchmarks

### Quick Benchmark (60 seconds)
```bash
./run_benchmark.sh
# or
python3 benchmark_quick.py
```
- 50 connections
- 1,000 messages (QoS 0)
- 500 messages (QoS 1)

### Rigorous Benchmark (4-6 minutes)
```bash
./run_benchmark.sh --rigorous
# or
python3 benchmark_rigorous.py
```
- 200 connections (gradual ramp)
- 5,000 messages (QoS 0)
- 2,000 messages (QoS 1)
- Full latency percentiles (P50, P90, P95, P99)

### Stress Test (3-4 minutes)
```bash
./run_benchmark.sh --stress
# or
python3 benchmark_stress.py
```
- 150 burst connections
- 10 concurrent publishers
- Mixed QoS workload

## Prerequisites

```bash
pip3 install paho-mqtt requests
```

## Documentation

- **BENCHMARK_RESULTS.md** - Complete test results and analysis
- **PERFORMANCE_COMPARISON.md** - Performance metrics comparison
- **BENCHMARK_FAQ.md** - Technical details and methodology

## Key Findings

| Metric | AegisGate | EMQX Direct | Difference |
|--------|-----------|-------------|------------|
| Connection Speed (gradual) | 67.72 conn/s | 68.18 conn/s | -0.67% |
| QoS 0 Throughput | 4,142 msg/s | 4,247 msg/s | -2.47% |
| QoS 1 Avg Latency | 120.79ms | 94.81ms | +25.97ms |
| QoS 1 P99 Latency | 197.49ms | 152.69ms | +44.80ms |
| Message Loss | 0% | 0% | Perfect |

AegisGate demonstrates promising performance for an alpha release, with minimal proxy overhead.

> **Note:** This is an alpha release (v0.1.0-alpha). These benchmarks validate the core proxy architecture, but the software is not yet recommended for production use.

**Last Updated:** March 2, 2026 15:47 IST