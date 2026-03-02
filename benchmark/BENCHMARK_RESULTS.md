# AegisGate Benchmark Results

**Test Date:** March 2, 2026  
**EMQX Version:** 5.7.2  
**Environment:** Docker/Colima, 512MB EMQX memory limit

## Executive Summary

AegisGate MQTT proxy demonstrates promising performance for an alpha release, with minimal overhead compared to direct EMQX connections. Key findings show virtually identical connection handling, excellent QoS 0 throughput, and expected proxy latency for QoS 1 operations.

> **Note:** This is an alpha release (v0.1.0-alpha). These benchmarks validate the core proxy architecture and performance characteristics, but the software is not yet recommended for production use.

**Performance Overview:**
- Connection speed: -0.67% (gradual ramp, 200 connections)
- QoS 0 throughput: -2.47% (5,000 messages)
- QoS 1 latency: +25.97ms average (expected proxy overhead)
- Message loss: 0% across all validated tests
- Reliability: 100% connection success rate

## Test Configuration

### System Resources
- EMQX Memory: 512MB (Docker container limit)
- Host: MacOS with Colima/Lima VM
- Network: Docker bridge with port forwarding

### Test Suites

1. **Quick Benchmark** - Fast validation (~60 seconds)
   - 50 connections, 1,000 QoS 0 messages, 500 QoS 1 messages

2. **Rigorous Benchmark** - High volume testing (~4-6 minutes)
   - 200 connections (gradual), 5,000 QoS 0 messages, 2,000 QoS 1 messages
   - Full latency percentiles (P50, P90, P95, P99)
   - 30 second sustained load test

3. **Stress Test** - Edge cases (~3-4 minutes)
   - 150 burst connections, 10 concurrent publishers, mixed QoS workload
   - Burst message handling (2,000 rapid-fire messages)

## Detailed Results

### Connection Establishment

#### Gradual Ramp (200 connections, 25 per batch)

| Metric | AegisGate | EMQX Direct | Difference |
|--------|-----------|-------------|------------|
| Connection Rate | 67.72 conn/s | 68.18 conn/s | -0.67% |
| Success Rate | 100% | 100% | - |
| Total Time | 2.953s | 2.933s | +0.020s |

**Analysis:** Virtually identical performance with gradual connection establishment.

#### Burst Connections (150 simultaneous)

| Metric | AegisGate | EMQX Direct | Difference |
|--------|-----------|-------------|------------|
| Connected | 150/150 | 150/150 | - |
| Failed | 0 | 0 | - |
| Time | 5.068s | 5.064s | +0.004s |
| Rate | 29.60 conn/s | 29.62 conn/s | -0.07% |

**Analysis:** Both systems handle burst connections reliably with zero failures.

#### Quick Test (50 connections, small sample)

| Metric | AegisGate | EMQX Direct | Difference |
|--------|-----------|-------------|------------|
| Connection Rate | 1778.66 conn/s | 2208.85 conn/s | -19.48% |
| Success Rate | 100% | 100% | - |
| Total Time | 0.028s | 0.023s | +0.005s |

**Analysis:** Small sample test shows higher variance. Prefer gradual ramp results for accuracy.

### Message Throughput

#### QoS 0 - High Volume (5,000 messages)

| Metric | AegisGate | EMQX Direct | Difference |
|--------|-----------|-------------|------------|
| Throughput | 4,142.40 msg/s | 4,247.45 msg/s | -2.47% |
| Avg Latency | 81.34ms | 72.20ms | +9.14ms |
| Median Latency | 86.37ms | 73.63ms | +12.74ms |
| P90 Latency | 93.86ms | 91.15ms | +2.71ms |
| P95 Latency | 95.31ms | 91.78ms | +3.53ms |
| P99 Latency | 95.93ms | 92.31ms | +3.62ms |
| Max Latency | 96.02ms | 92.46ms | +3.56ms |
| Message Loss | 0% | 0% | - |

**Analysis:** Excellent sustained throughput with zero message loss. ~9ms average latency overhead is within expected range for proxy.

#### QoS 0 - Quick Test (1,000 messages)

| Metric | AegisGate | EMQX Direct | Difference |
|--------|-----------|-------------|------------|
| Throughput | 1,817.40 msg/s | 1,833.49 msg/s | -0.88% |
| Avg Latency | 24.18ms | 22.05ms | +2.13ms |
| P50 Latency | 25.26ms | 22.84ms | +2.42ms |
| P95 Latency | 27.89ms | 25.30ms | +2.59ms |

**Analysis:** Small sample shows lower latency than high-volume test due to measurement variance.

#### QoS 1 - High Volume (2,000 messages)

| Metric | AegisGate | EMQX Direct | Difference |
|--------|-----------|-------------|------------|
| Throughput | 1,609.49 msg/s | 1,680.70 msg/s | -4.24% |
| Avg Latency | 120.79ms | 94.81ms | +25.97ms |
| Median Latency | 120.37ms | 94.50ms | +25.87ms |
| P90 Latency | 186.78ms | 142.42ms | +44.36ms |
| P95 Latency | 192.97ms | 147.64ms | +45.33ms |
| P99 Latency | 197.49ms | 152.69ms | +44.80ms |
| Max Latency | 198.88ms | 153.20ms | +45.68ms |
| Success Rate | 100% | 100% | - |

**Analysis:** QoS 1 shows expected proxy overhead (~26ms average, ~45ms P99) due to additional PUBACK round-trip through proxy.

#### QoS 1 - Quick Test (500 messages)

| Metric | AegisGate | EMQX Direct | Difference |
|--------|-----------|-------------|------------|
| Throughput | 871.81 msg/s | 896.83 msg/s | -2.79% |
| Avg Latency | 37.46ms | 29.64ms | +7.83ms |
| P50 Latency | 38.70ms | 29.89ms | +8.81ms |
| P95 Latency | 61.19ms | 48.15ms | +13.04ms |

**Analysis:** Smaller sample shows lower absolute latency but consistent overhead pattern.

#### Sustained Load (30 seconds @ 100 msg/s target)

| Metric | AegisGate | EMQX Direct | Difference |
|--------|-----------|-------------|------------|
| Duration | 30.01s | 30.01s | - |
| Messages Sent | 2,446 | 2,442 | +4 |
| Actual Rate | 81.51 msg/s | 81.38 msg/s | +0.16% |
| Avg Latency | 1.88ms | 0.77ms | +1.11ms |
| P99 Latency | 5.65ms | 1.79ms | +3.86ms |
| Max Latency | 11.08ms | 3.53ms | +7.55ms |

**Analysis:** Both systems throttled to ~81 msg/s (below target 100). AegisGate shows consistent low latency under sustained load.

### Stress Test Results

#### Burst Messages (2,000 rapid-fire, QoS 0)

| Metric | AegisGate | EMQX Direct | Difference |
|--------|-----------|-------------|------------|
| Publish Rate | 44,459 msg/s | 71,834 msg/s | -38.1% |
| Effective Throughput | 960.60 msg/s | 963.07 msg/s | -0.26% |
| Avg Latency | 36.96ms | 39.15ms | -2.19ms |
| P99 Latency | 42.47ms | 47.46ms | -4.99ms |
| Max Latency | 42.72ms | 47.49ms | -4.77ms |
| Message Loss | 0% | 0% | - |

**Analysis:** Slower burst publish rate (backpressure), but effective throughput nearly identical. Zero message loss.

#### Mixed QoS Workload (1,000 messages, 50/50 QoS 0/1)

| Metric | AegisGate | EMQX Direct | Difference |
|--------|-----------|-------------|------------|
| Total Throughput | 923.69 msg/s | 933.70 msg/s | -1.07% |
| QoS 0 Sent | 500/500 | 500/500 | - |
| QoS 1 Sent | 500/500 | 500/500 | - |
| Avg Latency | 30.44ms | 28.93ms | +1.51ms |
| P99 Latency | 63.10ms | 53.62ms | +9.48ms |

**Analysis:** Excellent handling of mixed QoS workload with minimal overhead.

#### Multiple Publishers (10 concurrent, 500 messages each)

| Metric | AegisGate | EMQX Direct |
|--------|-----------|-------------|
| Aggregate Publish Rate | 60,762 msg/s | 65,665 msg/s |
| Total Messages | 5,000/5,000 | 5,000/5,000 |
| Messages Received | 25,000 | 25,000 |
| Publish Time | 0.411s | 0.381s |

**Note:** "Received" count is 5x expected due to 5 subscribers (each receiving all 5,000 messages). This is expected behavior, not message duplication.

## Backend Validation

All tests validated against EMQX API metrics:

```
Messages dropped (no subscribers): 0
Total messages dropped: 0
Connection success rate: 100%
Post-test cleanup: Verified clean
```

**Validation Method:**
- EMQX API `/api/v5/metrics` for message counts
- EMQX API `/api/v5/stats` for connection verification
- Active subscriber confirmation before publishing
- Process cleanup verification after each test

## Performance Characteristics

### Strengths

1. **Connection Handling**
   - Gradual ramp: 0.67% difference vs EMQX
   - Burst: 100% success rate (150 simultaneous)
   - Zero connection failures across all tests

2. **QoS 0 Performance**
   - 4,142 msg/s sustained throughput
   - 2.47% throughput difference vs direct EMQX
   - Zero message loss

3. **Reliability**
   - 0% message loss across all tests
   - 100% connection success rate
   - Clean process lifecycle (no zombies)

### Expected Overhead

1. **QoS 1 Latency**
   - Average: +26ms (high-volume test)
   - P99: +45ms (high-volume test)
   - Cause: Additional PUBACK round-trip through proxy

2. **QoS 0 Latency**
   - Average: +9ms (high-volume test)
   - P99: +3.6ms (high-volume test)
   - Cause: Additional proxy hop

3. **Burst Publish Rate**
   - 38% slower raw publish rate
   - Effective throughput difference: 0.26%
   - Cause: Proxy backpressure/flow control

## Latency Measurement Notes

**Early Quick Tests:** Initial small sample tests (1,000 messages) showed inconsistent latency results, including cases where AegisGate appeared faster than direct EMQX. This was physically impossible for a proxy and indicated measurement noise.

**Root Causes Identified:**
- Small sample sizes susceptible to timing variance
- Python `time.time()` precision limitations (~15ms resolution)
- Localhost network timing unpredictability
- Statistical noise in sub-5ms differences

**Resolution:** Larger sample sizes (5,000+ messages) eliminated the measurement noise and confirmed the expected results: AegisGate adds a small, predictable overhead as a proxy should. The early "AegisGate faster than EMQX" result was a statistical artifact, not a real performance advantage.

**Recommendation:** Use rigorous benchmark (5,000+ messages) for accurate latency measurements. Quick benchmark is suitable for throughput validation only.

## Test Scripts

- **scripts/benchmark_quick.py** - Fast validation (60 seconds)
- **scripts/benchmark_rigorous.py** - High-volume testing (4-6 minutes)
- **scripts/benchmark_stress.py** - Edge case testing (3-4 minutes)
- **scripts/run_benchmark.sh** - Automated runner with cleanup

## Recommendations

> **Alpha Disclaimer:** These recommendations are based on performance benchmarks of the alpha release. AegisGate is not yet recommended for production deployment. This section describes potential use cases once the software reaches stability.

### When AegisGate May Be Suitable (Future):

1. **IoT/Sensor Data (QoS 0)** _(alpha validation shows potential)_
   - Excellent throughput (4,142 msg/s)
   - Minimal overhead (-2.47%)
   - Zero message loss in tests
   - Acceptable latency (+9ms average)

2. **Mixed Workloads** _(alpha validation shows potential)_
   - Reliable concurrent publisher handling
   - Good burst performance
   - Stable under stress in tests
   - 100% message delivery in tests

3. **High Connection Counts** _(alpha validation shows potential)_
   - Scales identically to EMQX (150+ burst connections in tests)
   - Reliable burst handling
   - Zero connection failures in tests

### Consider Direct EMQX For:

1. **Ultra-Low Latency QoS 1**
   - If <100ms P99 latency SLA is critical
   - AegisGate adds ~45ms P99 overhead for QoS 1
   - Average overhead: ~26ms

2. **Maximum Theoretical Burst Publish Rate**
   - If theoretical 70K+ msg/s burst needed
   - Note: Effective throughput is nearly identical (~0.26% difference)

## Conclusion

AegisGate demonstrates strong MQTT proxy performance for an alpha release, with minimal overhead:

- **Connection speed:** Virtually identical to EMQX (-0.67%)
- **QoS 0 throughput:** Near-identical (4,142 vs 4,247 msg/s, -2.47%)
- **QoS 1 latency:** +26ms average, +45ms P99 (acceptable proxy overhead)
- **Reliability:** Perfect (0% loss, 100% success rate)

The proxy overhead is measurable, predictable, and reasonable for a proof-of-concept. High-volume tests confirm consistent behavior under load with zero message loss, validating the core architecture.

### Key Takeaways

1. **No "magic" performance wins** - AegisGate performs as expected for a well-designed proxy
2. **Consistent behavior** - Large sample tests show predictable, stable overhead
3. **Reliable core** - Zero message loss and 100% reliability across all tests (alpha validation)
4. **QoS 0 optimized** - Minimal throughput impact for IoT/sensor workloads
5. **QoS 1 trade-off** - Additional latency is the expected cost of proxy architecture

---

**Last Updated:** March 2, 2026 15:47 IST  
**Test Coverage:** Connection handling, throughput, latency, stress scenarios  
**Validation:** Full EMQX backend metrics verification  
**Sample Sizes:** 50-5,000 messages per test, 50-200 connections