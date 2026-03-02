# Benchmark Methodology and Technical Details

## Measurement Methodology

### Latency Measurement

End-to-end latency is measured from message publication to subscriber receipt:

```python
send_time = time.time()
messages_timestamps[msg_id] = send_time
publish(topic, payload, qos)

# In subscriber callback
recv_time = time.time()
latency_ms = (recv_time - messages_timestamps[msg_id]) * 1000
```

**What is captured:**
- Client publish time
- Network transit through proxy (if applicable)
- EMQX broker processing
- Subscription delivery
- Network return to subscriber

**Limitations:**
- Python `time.time()` resolution: ~15ms on some systems
- Localhost testing eliminates real network latency
- Results show relative performance, not absolute production latency
- Statistical variance in small samples

### Throughput Measurement

Throughput is calculated as messages received divided by total test duration:

```python
start_time = time.time()
# ... publish messages ...
# ... wait for receipt ...
end_time = time.time()

throughput = messages_received / (end_time - start_time)
```

### Connection Rate Measurement

Connection rate measured during establishment phase:

```python
start_time = time.time()
# ... establish N connections ...
connect_time = time.time() - start_time

connection_rate = N / connect_time
```

## Sample Size Considerations

### Why Small Samples Show Anomalous Results

Quick benchmark (1,000 messages) showed AegisGate with 3.45ms lower latency than EMQX. This is physically impossible for a proxy and indicates measurement variance.

**Contributing Factors:**

1. **Statistical Noise**
   - Small samples susceptible to outliers
   - Network timing variance not averaged out
   - CPU scheduling effects more pronounced

2. **Timing Precision**
   - Python `time.time()` granularity ~15ms
   - Sub-5ms differences within measurement error
   - Requires thousands of samples for accuracy

3. **Localhost Effects**
   - No real network delays
   - Timing affected by context switches
   - Non-deterministic OS scheduling

**Solution:** Use sample sizes of 5,000+ messages for accurate latency statistics.

### Recommended Sample Sizes

| Metric | Minimum Samples | Recommended | Rationale |
|--------|----------------|-------------|-----------|
| Average Latency | 1,000 | 5,000+ | Reduce variance |
| P99 Latency | 5,000 | 10,000+ | Need tail accuracy |
| Throughput | 500 | 1,000+ | Simple counting |
| Connection Rate | 50 | 200+ | Smooth out bursts |

## Percentile Calculations

Latency percentiles calculated from sorted latency array:

```python
sorted_latencies = sorted(all_latencies)
n = len(sorted_latencies)

p50 = sorted_latencies[int(n * 0.50)]  # Median
p90 = sorted_latencies[int(n * 0.90)]  # 90th percentile
p95 = sorted_latencies[int(n * 0.95)]  # 95th percentile
p99 = sorted_latencies[int(n * 0.99)]  # 99th percentile
```

**Interpretation:**
- P50: Half of messages had this latency or less
- P90: 90% of messages had this latency or less
- P95: 95% of messages had this latency or less
- P99: 99% of messages had this latency or less

**Why P99 Matters:** Represents worst-case performance for 99% of requests. More meaningful than max (which may be a single outlier) or average (which can hide tail latency).

## Connection Testing Strategies

### Gradual Ramp

Connections established in batches with delays between batches:

```bash
for batch in {1..8}; do
  for i in {1..25}; do
    connect_client $i &
  done
  sleep 0.1  # 100ms between batches
done
```

**Purpose:**
- Prevents resource exhaustion
- More realistic (clients don't all connect simultaneously)
- Safe for memory-limited environments
- Avoids triggering connection rate limits

**Results:** 67.72 conn/s (AegisGate) vs 68.18 conn/s (EMQX) = -0.67%

### Burst Connections

All connections attempted simultaneously:

```bash
for i in {1..150}; do
  connect_client $i &
done
```

**Purpose:**
- Tests maximum capacity
- Reveals queue handling behavior
- Simulates DDoS-like scenarios
- Identifies breaking points

**Results:** Both systems achieved 150/150 connections with 0 failures in ~5 seconds.

**Historical Note:** Previous testing with 500 burst connections caused EMQX crash due to 512MB memory limit.

## Message Loss Detection

### Client-Side Tracking

```python
sent_count = 0
received_count = 0

# On publish
sent_count += 1

# On receive
received_count += 1

# After test
loss = sent_count - received_count
loss_percentage = (loss / sent_count) * 100
```

### Backend Validation

EMQX metrics checked via API:

```bash
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:18083/api/v5/metrics

# Check:
# - messages.publish (total published)
# - messages.received (total received by broker)
# - messages.dropped (total dropped)
# - messages.dropped.no_subscribers (dropped due to no subscribers)
```

### Subscriber Verification

Before publishing, ensure subscribers are active:

```python
# 1. Connect subscriber
subscriber.connect(host, port)

# 2. Subscribe to topic
subscriber.subscribe(topic, qos)

# 3. Wait for backend registration
time.sleep(1)

# 4. Verify in EMQX
stats = get_emqx_stats(token)
assert stats['subscriptions.count'] > 0

# 5. Now safe to publish
publisher.publish(topic, payload, qos)
```

**Historical Issue:** Early tests dropped 70,000+ messages because publishers started before subscribers were registered.

## Test Environment

### Resource Constraints

```yaml
EMQX Container:
  Memory Limit: 512MB
  Typical Usage: ~220MB baseline
  Peak Usage: ~240MB (200 connections)
  Crash Threshold: ~500 burst connections
```

### Network Configuration

- Docker bridge network
- Localhost communication (no real network latency)
- Port forwarding: 1883 (MQTT), 18083 (EMQX API)

### Software Versions

- EMQX: 5.7.2
- Python: 3.x
- paho-mqtt: Latest
- OS: MacOS with Colima/Lima VM
- Test Date: March 2, 2026

## Backend Validation

### EMQX API Endpoints

**Authentication:**
```bash
curl -X POST http://localhost:18083/api/v5/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"public"}'
# Returns: {"token": "eyJ..."}
```

**Connection Statistics:**
```bash
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:18083/api/v5/stats
```

Returns:
- `connections.count` - Current active connections
- `live_connections.count` - Live connections
- `subscriptions.count` - Active subscriptions
- `sessions.count` - Current sessions

**Message Metrics:**
```bash
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:18083/api/v5/metrics
```

Returns:
- `messages.publish` - Total messages published
- `messages.received` - Total messages received by broker
- `messages.delivered` - Total messages delivered to subscribers
- `messages.dropped` - Total messages dropped
- `messages.dropped.no_subscribers` - Dropped due to no subscribers

### Validation Criteria

Test considered valid when:
- Client-side sent count matches backend `messages.publish`
- Client-side received count matches expected
- Backend `messages.dropped` = 0
- Backend `messages.dropped.no_subscribers` = 0
- Post-test connections = 0
- No zombie processes remain

## QoS Behavior

### QoS 0 (At Most Once)

- No acknowledgment required
- Fire-and-forget delivery
- Lowest latency
- No retransmission on failure
- Used for: IoT sensor data, telemetry

**Expected Performance:**
- Near-identical throughput vs direct EMQX (~2-3% overhead)
- Minimal latency overhead (~9ms average in high-volume tests)

### QoS 1 (At Least Once)

- PUBACK acknowledgment required
- Guaranteed delivery
- Higher latency (round-trip)
- Possible duplicates
- Used for: Notifications, commands

**Expected Performance:**
- Slight throughput reduction (~4% in high-volume tests)
- Additional latency (+26ms average in high-volume tests)
- Higher P99 latency (+45ms in high-volume tests)

**Why Proxy Adds Latency:**
```
QoS 0:  Client → AegisGate → EMQX → Subscriber
        (one-way path)

QoS 1:  Client → AegisGate → EMQX → Subscriber
        Client ← AegisGate ← EMQX (PUBACK)
        (acknowledgment round-trip through proxy)
```

## Proxy Overhead Analysis

### Expected Overhead Sources

1. **Additional Network Hop**
   - Client → Proxy → EMQX instead of Client → EMQX
   - Doubles network traversals for QoS 1

2. **Serialization/Deserialization**
   - Proxy must parse MQTT packets
   - Forward to backend
   - Parse responses

3. **Connection Maintenance**
   - Proxy maintains connections to both client and EMQX
   - Additional memory and CPU overhead

4. **Flow Control**
   - Proxy may implement backpressure
   - Prevents overwhelming backend
   - Reduces burst publish rate

### Measured Overhead (Latest Benchmark Results)

**Connection Speed:**
- Gradual: -0.67% (negligible)
- Burst: -0.07% (negligible)

**Throughput:**
- QoS 0 (5,000 msgs): -2.47% (minimal)
- QoS 1 (2,000 msgs): -4.24% (minimal)
- Sustained load: +0.16% (negligible)

**Latency:**
- QoS 0 average: +9.14ms (expected)
- QoS 0 P99: +3.62ms (minimal)
- QoS 1 average: +25.97ms (expected)
- QoS 1 P99: +44.80ms (expected)

## Stress Testing Rationale

### Burst Connections

**Purpose:** Identify maximum connection capacity and failure modes.

**Method:** Attempt all connections simultaneously without delays.

**Historical Context:** 500 burst connections crashed EMQX (512MB limit). Reduced to 150 for safe testing.

### Concurrent Publishers

**Purpose:** Test concurrent load handling and message ordering.

**Method:** Start multiple publishers simultaneously, each publishing to different topics.

**Results:** 60,762 msg/s aggregate publish rate with 10 publishers, all 5,000 messages delivered successfully.

### Mixed QoS Workload

**Purpose:** Test protocol handling under heterogeneous traffic.

**Method:** Alternate QoS 0 and QoS 1 messages in same stream.

**Results:** 923.69 msg/s (AegisGate) vs 933.70 msg/s (EMQX) with proper delivery of both QoS levels.

## Process Management

### Cleanup Strategy

All benchmark scripts implement cleanup:

```python
import signal

def cleanup():
    # Stop publishers
    for pub in publishers:
        pub.loop_stop()
        pub.disconnect()
    
    # Stop subscribers
    for sub in subscribers:
        sub.loop_stop()
        sub.disconnect()

def signal_handler(sig, frame):
    cleanup()
    sys.exit(0)

signal.signal(signal.SIGINT, signal_handler)
signal.signal(signal.SIGTERM, signal_handler)

try:
    run_benchmark()
finally:
    cleanup()
```

### Zombie Process Prevention

Runner script performs cleanup before and after tests:

```bash
# Before test
pkill -9 -f benchmark
pkill -9 -f mosquitto

# After test
ps aux | grep benchmark | grep -v grep
# Verify count is 0
```

### Post-Test Verification

```bash
# Check for lingering processes
ZOMBIE_COUNT=$(ps aux | grep benchmark | grep -v grep | wc -l)

# Check for continued publishing
MSG_BEFORE=$(get_emqx_messages)
sleep 3
MSG_AFTER=$(get_emqx_messages)
RATE=$(( (MSG_AFTER - MSG_BEFORE) / 3 ))

# Should be 0
if [ $RATE -eq 0 ]; then
  echo "Clean"
fi
```

## Benchmark Scripts

### benchmark_quick.py

**Purpose:** Fast validation for CI/CD or quick checks.

**Duration:** ~60 seconds

**Tests:**
- 50 connections
- 1,000 QoS 0 messages
- 500 QoS 1 messages

**Use When:** Need quick feedback or regression testing.

**Note:** Small sample sizes may show measurement variance. Use rigorous benchmark for accurate latency measurements.

### benchmark_rigorous.py

**Purpose:** Detailed performance analysis with statistics.

**Duration:** ~4-6 minutes

**Tests:**
- 200 connections (gradual ramp)
- 5,000 QoS 0 messages (with full percentiles)
- 2,000 QoS 1 messages (with full percentiles)
- 30 second sustained load test
- Full latency percentiles (P50, P90, P95, P99)

**Use When:** Need accurate performance metrics or detailed analysis. Recommended for official performance validation.</end_text>

<old_text line=507>
**Tests:**
- 150 burst connections
- 10 concurrent publishers
- 2,000 burst messages
- Mixed QoS workload

**Use When:** Need to understand system limits or failure modes.

### benchmark_stress.py

**Purpose:** Find limits and edge cases.

**Duration:** ~3-4 minutes

**Tests:**
- 150 burst connections
- 10 concurrent publishers
- 2,000 burst messages
- Mixed QoS workload

**Use When:** Need to understand system limits or failure modes.

### run_benchmark.sh

**Purpose:** Automated execution with cleanup and validation.

**Features:**
- Pre-test cleanup
- EMQX restart with fresh state
- Post-test verification
- Process cleanup

**Use When:** Need reliable, repeatable benchmark execution.

## Interpreting Results

### Acceptable Thresholds

Based on proxy architecture expectations:

| Metric | Threshold | Status if Within | AegisGate Result |
|--------|-----------|------------------|------------------|
| Connection overhead | <10% | Acceptable | -0.67% ✓ |
| QoS 0 throughput | <5% difference | Acceptable | -2.47% ✓ |
| QoS 1 throughput | <10% difference | Acceptable | -4.24% ✓ |
| QoS 0 latency | <15ms overhead | Acceptable | +9.14ms ✓ |
| QoS 1 latency | <35ms overhead | Acceptable | +25.97ms ✓ |
| Message loss | 0% | Required | 0% ✓ |
| Connection failures | 0% | Required | 0% ✓ |

### Red Flags

Investigate if:
- Connection overhead >20%
- Throughput difference >15%
- QoS 1 latency overhead >50ms average
- Any message loss >0%
- Any connection failures

### Early Test Inconsistencies (Resolved)

**Issue:** Initial small sample tests (~1,000 messages) showed inconsistent results, including cases where AegisGate appeared to have lower latency than direct EMQX. This was physically impossible for a proxy.

**Root Causes Identified:**
- Small sample sizes susceptible to timing variance
- Python `time.time()` precision limitations (~15ms resolution)
- Localhost network timing unpredictability
- Statistical noise in sub-5ms differences

**Resolution:** Larger sample sizes (5,000+ messages) eliminated the measurement noise and confirmed the expected results: AegisGate adds small, predictable overhead as a proxy should. The early "AegisGate faster than EMQX" result was a statistical artifact, not a real performance advantage.

**Key Lesson:** Always use rigorous benchmark (5,000+ messages) for accurate latency measurements. Quick benchmark is suitable for throughput validation only.

## Production Considerations

### Differences from Benchmark Environment

**Benchmark:**
- Localhost testing (no network latency)
- Single Docker host
- 512MB EMQX limit
- Controlled load patterns

**Production:**
- Real network latency (1-100ms typical)
- Distributed deployment
- Higher resource limits
- Variable/unpredictable load

### Expected Production Performance

Add network latency to benchmark results:

```
Production Latency = Benchmark Latency + (2 × Network RTT)

Example:
  Network RTT: 10ms
  Benchmark QoS 1: 120.79ms
  Expected Production: 120.79ms + (2 × 10ms) = 140.79ms
```

### Scaling Considerations

Benchmark results are for:
- Single proxy instance
- Single EMQX instance
- 512MB EMQX memory

For higher throughput:
- Deploy multiple proxy instances (horizontal scaling)
- Increase EMQX memory limit
- Use EMQX clustering for backend

---

**Last Updated:** March 2, 2026 15:47 IST
**Benchmark Version:** 3.1
**Test Coverage:** Connection handling, throughput, latency, stress scenarios
**Sample Sizes:** 500-5,000 messages per test, 50-200 connections
**Validation:** Full EMQX backend metrics verification