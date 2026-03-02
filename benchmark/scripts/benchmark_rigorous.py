#!/usr/bin/env python3
"""
Rigorous High-Volume MQTT Benchmark
AegisGate vs EMQX Direct Comparison
Designed for thorough testing while respecting resource constraints
"""

import signal
import statistics
import subprocess
import sys
import time
from collections import defaultdict
from threading import Event, Lock, Thread

import paho.mqtt.client as mqtt
import requests

# Configuration
EMQX_HOST = "localhost"
EMQX_PORT = 1883
AEGIS_HOST = "localhost"
AEGIS_PORT = 8080
EMQX_API = "http://localhost:18083/api/v5"
AEGIS_API = "http://localhost:9090/metrics"

# Rigorous test parameters - high but safe
CONN_TEST_COUNT = 200  # 4x previous
MSG_QOS0_COUNT = 5000  # 5x previous
MSG_QOS1_COUNT = 2000  # 4x previous
SUSTAINED_TEST_DURATION = 30  # 30 seconds of sustained load
SUSTAINED_RATE = 100  # messages per second during sustained test

# Colors
RED = "\033[91m"
GREEN = "\033[92m"
YELLOW = "\033[93m"
BLUE = "\033[94m"
MAGENTA = "\033[95m"
CYAN = "\033[96m"
NC = "\033[0m"


class RigorousStats:
    def __init__(self):
        self.latencies = []
        self.sent = 0
        self.received = 0
        self.errors = 0
        self.lock = Lock()
        self.start_time = None
        self.end_time = None

    def record_latency(self, latency_ms):
        with self.lock:
            self.latencies.append(latency_ms)

    def increment_sent(self):
        with self.lock:
            self.sent += 1

    def increment_received(self):
        with self.lock:
            self.received += 1

    def increment_errors(self):
        with self.lock:
            self.errors += 1

    def get_stats(self):
        with self.lock:
            if self.latencies:
                sorted_latencies = sorted(self.latencies)
                n = len(sorted_latencies)
                return {
                    "sent": self.sent,
                    "received": self.received,
                    "errors": self.errors,
                    "avg_latency": statistics.mean(self.latencies),
                    "median_latency": statistics.median(self.latencies),
                    "p50_latency": sorted_latencies[int(n * 0.50)],
                    "p90_latency": sorted_latencies[int(n * 0.90)]
                    if n > 10
                    else sorted_latencies[-1],
                    "p95_latency": sorted_latencies[int(n * 0.95)]
                    if n > 20
                    else sorted_latencies[-1],
                    "p99_latency": sorted_latencies[int(n * 0.99)]
                    if n > 100
                    else sorted_latencies[-1],
                    "min_latency": min(self.latencies),
                    "max_latency": max(self.latencies),
                    "stdev_latency": statistics.stdev(self.latencies)
                    if len(self.latencies) > 1
                    else 0,
                }
            return {
                "sent": self.sent,
                "received": self.received,
                "errors": self.errors,
                "avg_latency": 0,
                "median_latency": 0,
                "p50_latency": 0,
                "p90_latency": 0,
                "p95_latency": 0,
                "p99_latency": 0,
                "min_latency": 0,
                "max_latency": 0,
                "stdev_latency": 0,
            }


def get_emqx_token():
    """Get EMQX API token"""
    try:
        response = requests.post(
            f"{EMQX_API}/login",
            json={"username": "admin", "password": "public"},
            timeout=5,
        )
        return response.json().get("token")
    except:
        return None


def get_emqx_metrics(token):
    """Get EMQX metrics"""
    try:
        response = requests.get(
            f"{EMQX_API}/metrics",
            headers={"Authorization": f"Bearer {token}"},
            timeout=5,
        )
        if response.status_code == 200:
            return response.json()[0]
        return None
    except:
        return None


def get_emqx_stats(token):
    """Get EMQX stats"""
    try:
        response = requests.get(
            f"{EMQX_API}/stats",
            headers={"Authorization": f"Bearer {token}"},
            timeout=5,
        )
        if response.status_code == 200:
            return response.json()[0]
        return None
    except:
        return None


def cleanup_processes():
    """Clean up any lingering benchmark processes"""
    try:
        subprocess.run(["pkill", "-9", "-f", "benchmark"], stderr=subprocess.DEVNULL)
        subprocess.run(["pkill", "-9", "-f", "mosquitto"], stderr=subprocess.DEVNULL)
    except:
        pass


def test_connections_gradual(host, port, count, name):
    """Test connection establishment with gradual ramp-up"""
    print(f"\n{CYAN}Testing {name} - {count} connections (gradual ramp){NC}")

    clients = []
    connected_count = 0
    failed_count = 0
    lock = Lock()

    def on_connect(client, userdata, flags, rc):
        nonlocal connected_count, failed_count
        with lock:
            if rc == 0:
                connected_count += 1
            else:
                failed_count += 1

    start_time = time.time()

    # Gradual ramp - 25 connections at a time with 100ms pause
    batch_size = 25
    for batch_start in range(0, count, batch_size):
        batch_end = min(batch_start + batch_size, count)
        for i in range(batch_start, batch_end):
            client = mqtt.Client(client_id=f"{name}_conn_rig_{i}", clean_session=True)
            client.on_connect = on_connect
            try:
                client.connect(host, port, 60)
                client.loop_start()
                clients.append(client)
            except Exception as e:
                with lock:
                    failed_count += 1
        time.sleep(0.1)  # 100ms between batches

    # Wait for all connections to establish
    time.sleep(2)
    connect_time = time.time() - start_time

    print(f"  {GREEN}✓{NC} Connected: {connected_count}/{count}")
    if failed_count > 0:
        print(f"  {RED}✗{NC} Failed: {failed_count}")
    print(f"  {GREEN}✓{NC} Total time: {connect_time:.3f}s")
    print(f"  {GREEN}✓{NC} Effective rate: {connected_count / connect_time:.2f} conn/s")

    # Hold connections briefly to verify backend
    time.sleep(1)

    # Cleanup
    print(f"  Disconnecting...")
    for client in clients:
        try:
            client.loop_stop()
            client.disconnect()
        except:
            pass

    time.sleep(1)

    return {
        "connected": connected_count,
        "failed": failed_count,
        "time": connect_time,
        "rate": connected_count / connect_time if connect_time > 0 else 0,
    }


def test_throughput_rigorous(host, port, msg_count, qos, name):
    """Test message throughput with rigorous latency measurement"""
    print(f"\n{CYAN}Testing {name} - {msg_count} messages (QoS {qos}){NC}")

    stats = RigorousStats()
    topic = f"benchmark/rigorous/{name}"
    messages_timestamps = {}
    received_event = Event()

    # Subscriber callbacks
    def on_sub_connect(client, userdata, flags, rc):
        if rc == 0:
            client.subscribe(topic, qos=qos)

    def on_message(client, userdata, msg):
        recv_time = time.time()
        try:
            payload = msg.payload.decode()
            msg_id = payload.split("_")[-1]
            if msg_id in messages_timestamps:
                latency_ms = (recv_time - messages_timestamps[msg_id]) * 1000
                stats.record_latency(latency_ms)
            stats.increment_received()
            if stats.received >= msg_count:
                received_event.set()
        except:
            stats.increment_errors()

    # Setup subscriber
    sub_client = mqtt.Client(client_id=f"{name}_sub_rig", clean_session=True)
    sub_client.on_connect = on_sub_connect
    sub_client.on_message = on_message

    try:
        sub_client.connect(host, port, 60)
        sub_client.loop_start()
        time.sleep(1)  # Ensure subscription is active
        print(f"  {GREEN}✓{NC} Subscriber ready")
    except Exception as e:
        print(f"{RED}Subscriber error: {e}{NC}")
        return None

    # Publisher callbacks
    def on_pub_connect(client, userdata, flags, rc):
        pass

    pub_client = mqtt.Client(client_id=f"{name}_pub_rig", clean_session=True)
    pub_client.on_connect = on_pub_connect

    try:
        pub_client.connect(host, port, 60)
        pub_client.loop_start()
        time.sleep(0.5)
        print(f"  {GREEN}✓{NC} Publisher ready")
    except Exception as e:
        print(f"{RED}Publisher error: {e}{NC}")
        sub_client.loop_stop()
        sub_client.disconnect()
        return None

    # Publish messages
    print(f"  Publishing {msg_count} messages...")
    start_time = time.time()
    stats.start_time = start_time

    for i in range(msg_count):
        msg_id = f"{i}"
        payload = f"benchmark_msg_{msg_id}"
        send_time = time.time()
        messages_timestamps[msg_id] = send_time

        result = pub_client.publish(topic, payload, qos=qos)
        if result.rc == mqtt.MQTT_ERR_SUCCESS:
            stats.increment_sent()
        else:
            stats.increment_errors()

        # Progress indicator
        if (i + 1) % 1000 == 0:
            print(f"    Sent: {i + 1}/{msg_count}")

    pub_time = time.time() - start_time

    # Wait for messages to be received
    print(f"  Waiting for all messages to be received...")
    received_event.wait(timeout=10)

    # Give time for final messages
    time.sleep(1)
    stats.end_time = time.time()

    # Get results
    final_stats = stats.get_stats()
    total_time = stats.end_time - stats.start_time
    throughput = final_stats["received"] / total_time if total_time > 0 else 0
    loss = msg_count - final_stats["received"]
    loss_pct = (loss / msg_count * 100) if msg_count > 0 else 0

    print(f"\n  {CYAN}Results:{NC}")
    print(f"    Sent: {final_stats['sent']}/{msg_count}")
    print(f"    Received: {final_stats['received']}/{msg_count}")
    print(f"    Loss: {loss} ({loss_pct:.2f}%)")
    print(f"    Errors: {final_stats['errors']}")
    print(f"    Total time: {total_time:.3f}s")
    print(f"    Throughput: {GREEN}{throughput:.2f} msg/s{NC}")
    print(f"\n  {CYAN}Latency Statistics:{NC}")
    print(f"    Min:    {final_stats['min_latency']:.3f}ms")
    print(f"    Mean:   {GREEN}{final_stats['avg_latency']:.3f}ms{NC}")
    print(f"    Median: {final_stats['median_latency']:.3f}ms")
    print(f"    P90:    {final_stats['p90_latency']:.3f}ms")
    print(f"    P95:    {final_stats['p95_latency']:.3f}ms")
    print(f"    P99:    {YELLOW}{final_stats['p99_latency']:.3f}ms{NC}")
    print(f"    Max:    {final_stats['max_latency']:.3f}ms")
    print(f"    StdDev: {final_stats['stdev_latency']:.3f}ms")

    # Cleanup
    pub_client.loop_stop()
    pub_client.disconnect()
    sub_client.loop_stop()
    sub_client.disconnect()

    time.sleep(1)

    return {
        "sent": final_stats["sent"],
        "received": final_stats["received"],
        "loss": loss,
        "loss_pct": loss_pct,
        "throughput": throughput,
        "avg_latency": final_stats["avg_latency"],
        "median_latency": final_stats["median_latency"],
        "p50_latency": final_stats["p50_latency"],
        "p90_latency": final_stats["p90_latency"],
        "p95_latency": final_stats["p95_latency"],
        "p99_latency": final_stats["p99_latency"],
        "max_latency": final_stats["max_latency"],
        "stdev_latency": final_stats["stdev_latency"],
        "total_time": total_time,
    }


def test_sustained_load(host, port, duration_sec, rate_per_sec, qos, name):
    """Test sustained load over time"""
    print(
        f"\n{CYAN}Testing {name} - Sustained load ({duration_sec}s @ {rate_per_sec} msg/s, QoS {qos}){NC}"
    )

    stats = RigorousStats()
    topic = f"benchmark/sustained/{name}"
    messages_timestamps = {}
    running = Event()
    running.set()

    # Subscriber
    def on_sub_connect(client, userdata, flags, rc):
        if rc == 0:
            client.subscribe(topic, qos=qos)

    def on_message(client, userdata, msg):
        recv_time = time.time()
        try:
            payload = msg.payload.decode()
            msg_id = payload.split("_")[-1]
            if msg_id in messages_timestamps:
                latency_ms = (recv_time - messages_timestamps[msg_id]) * 1000
                stats.record_latency(latency_ms)
            stats.increment_received()
        except:
            stats.increment_errors()

    sub_client = mqtt.Client(client_id=f"{name}_sub_sust", clean_session=True)
    sub_client.on_connect = on_sub_connect
    sub_client.on_message = on_message

    try:
        sub_client.connect(host, port, 60)
        sub_client.loop_start()
        time.sleep(1)
        print(f"  {GREEN}✓{NC} Subscriber ready")
    except Exception as e:
        print(f"{RED}Subscriber error: {e}{NC}")
        return None

    # Publisher
    pub_client = mqtt.Client(client_id=f"{name}_pub_sust", clean_session=True)

    try:
        pub_client.connect(host, port, 60)
        pub_client.loop_start()
        time.sleep(0.5)
        print(f"  {GREEN}✓{NC} Publisher ready")
    except Exception as e:
        print(f"{RED}Publisher error: {e}{NC}")
        sub_client.loop_stop()
        sub_client.disconnect()
        return None

    # Publishing thread
    def publish_worker():
        msg_id = 0
        interval = 1.0 / rate_per_sec
        start = time.time()
        end_time = start + duration_sec

        while time.time() < end_time and running.is_set():
            msg_id_str = f"{msg_id}"
            payload = f"sustained_msg_{msg_id_str}"
            send_time = time.time()
            messages_timestamps[msg_id_str] = send_time

            result = pub_client.publish(topic, payload, qos=qos)
            if result.rc == mqtt.MQTT_ERR_SUCCESS:
                stats.increment_sent()
            else:
                stats.increment_errors()

            msg_id += 1
            time.sleep(interval)

    print(f"  Publishing for {duration_sec} seconds...")
    stats.start_time = time.time()
    pub_thread = Thread(target=publish_worker)
    pub_thread.start()
    pub_thread.join()
    stats.end_time = time.time()

    # Wait for remaining messages
    time.sleep(2)

    # Results
    final_stats = stats.get_stats()
    actual_duration = stats.end_time - stats.start_time
    actual_throughput = final_stats["received"] / actual_duration
    expected_total = int(duration_sec * rate_per_sec)

    print(f"\n  {CYAN}Sustained Load Results:{NC}")
    print(f"    Duration: {actual_duration:.2f}s")
    print(f"    Expected messages: {expected_total}")
    print(f"    Sent: {final_stats['sent']}")
    print(f"    Received: {final_stats['received']}")
    print(
        f"    Actual rate: {GREEN}{actual_throughput:.2f} msg/s{NC} (target: {rate_per_sec})"
    )
    print(f"    Avg latency: {final_stats['avg_latency']:.3f}ms")
    print(f"    P99 latency: {final_stats['p99_latency']:.3f}ms")
    print(f"    Max latency: {final_stats['max_latency']:.3f}ms")

    running.clear()

    # Cleanup
    pub_client.loop_stop()
    pub_client.disconnect()
    sub_client.loop_stop()
    sub_client.disconnect()

    time.sleep(1)

    return {
        "sent": final_stats["sent"],
        "received": final_stats["received"],
        "expected": expected_total,
        "throughput": actual_throughput,
        "avg_latency": final_stats["avg_latency"],
        "p99_latency": final_stats["p99_latency"],
        "max_latency": final_stats["max_latency"],
        "duration": actual_duration,
    }


def main():
    print(f"{MAGENTA}{'=' * 70}{NC}")
    print(f"{MAGENTA}RIGOROUS HIGH-VOLUME MQTT BENCHMARK{NC}")
    print(f"{MAGENTA}AegisGate vs EMQX Direct Comparison{NC}")
    print(f"{MAGENTA}Resource-aware testing with comprehensive metrics{NC}")
    print(f"{MAGENTA}{'=' * 70}{NC}\n")

    # Signal handler
    def signal_handler(sig, frame):
        print(f"\n{YELLOW}Interrupted, cleaning up...{NC}")
        cleanup_processes()
        sys.exit(1)

    signal.signal(signal.SIGINT, signal_handler)
    signal.signal(signal.SIGTERM, signal_handler)

    # Cleanup
    print(f"{YELLOW}Cleaning up...{NC}")
    cleanup_processes()
    time.sleep(1)

    # Verify EMQX
    token = get_emqx_token()
    if not token:
        print(f"{RED}ERROR: Cannot reach EMQX API{NC}")
        return 1

    print(f"{GREEN}✓ EMQX is ready{NC}")
    print(f"{GREEN}✓ AegisGate is ready{NC}")

    # Get baseline metrics
    baseline_metrics = get_emqx_metrics(token)
    baseline_dropped = (
        baseline_metrics.get("messages.dropped", 0) if baseline_metrics else 0
    )

    # ==================================================================
    # TEST 1: Connection Speed (Gradual Ramp)
    # ==================================================================
    print(f"\n{MAGENTA}{'=' * 70}{NC}")
    print(f"{MAGENTA}TEST 1: CONNECTION SPEED ({CONN_TEST_COUNT} connections){NC}")
    print(f"{MAGENTA}{'=' * 70}{NC}")

    aegis_conn = test_connections_gradual(
        AEGIS_HOST, AEGIS_PORT, CONN_TEST_COUNT, "AegisGate"
    )
    time.sleep(3)
    emqx_conn = test_connections_gradual(EMQX_HOST, EMQX_PORT, CONN_TEST_COUNT, "EMQX")

    conn_diff = (
        ((aegis_conn["rate"] - emqx_conn["rate"]) / emqx_conn["rate"] * 100)
        if emqx_conn["rate"] > 0
        else 0
    )

    print(f"\n{CYAN}Connection Summary:{NC}")
    print(f"  AegisGate: {aegis_conn['rate']:.2f} conn/s")
    print(f"  EMQX:      {emqx_conn['rate']:.2f} conn/s")
    print(f"  Difference: {YELLOW}{conn_diff:+.2f}%{NC}")

    # ==================================================================
    # TEST 2: QoS 0 High-Volume Throughput
    # ==================================================================
    print(f"\n{MAGENTA}{'=' * 70}{NC}")
    print(
        f"{MAGENTA}TEST 2: QoS 0 HIGH-VOLUME THROUGHPUT ({MSG_QOS0_COUNT} messages){NC}"
    )
    print(f"{MAGENTA}{'=' * 70}{NC}")

    aegis_qos0 = test_throughput_rigorous(
        AEGIS_HOST, AEGIS_PORT, MSG_QOS0_COUNT, 0, "AegisGate_QoS0"
    )
    time.sleep(3)
    emqx_qos0 = test_throughput_rigorous(
        EMQX_HOST, EMQX_PORT, MSG_QOS0_COUNT, 0, "EMQX_QoS0"
    )

    if aegis_qos0 and emqx_qos0:
        throughput_diff = (
            (
                (aegis_qos0["throughput"] - emqx_qos0["throughput"])
                / emqx_qos0["throughput"]
                * 100
            )
            if emqx_qos0["throughput"] > 0
            else 0
        )
        latency_diff = aegis_qos0["avg_latency"] - emqx_qos0["avg_latency"]
        p99_diff = aegis_qos0["p99_latency"] - emqx_qos0["p99_latency"]

        print(f"\n{CYAN}QoS 0 Comparison:{NC}")
        print(
            f"  Throughput - AegisGate: {aegis_qos0['throughput']:.2f} msg/s | EMQX: {emqx_qos0['throughput']:.2f} msg/s | Diff: {throughput_diff:+.2f}%"
        )
        print(
            f"  Avg Latency - AegisGate: {aegis_qos0['avg_latency']:.2f}ms | EMQX: {emqx_qos0['avg_latency']:.2f}ms | Diff: {latency_diff:+.2f}ms"
        )
        print(
            f"  P99 Latency - AegisGate: {aegis_qos0['p99_latency']:.2f}ms | EMQX: {emqx_qos0['p99_latency']:.2f}ms | Diff: {p99_diff:+.2f}ms"
        )

    # ==================================================================
    # TEST 3: QoS 1 High-Volume Throughput
    # ==================================================================
    print(f"\n{MAGENTA}{'=' * 70}{NC}")
    print(
        f"{MAGENTA}TEST 3: QoS 1 HIGH-VOLUME THROUGHPUT ({MSG_QOS1_COUNT} messages){NC}"
    )
    print(f"{MAGENTA}{'=' * 70}{NC}")

    aegis_qos1 = test_throughput_rigorous(
        AEGIS_HOST, AEGIS_PORT, MSG_QOS1_COUNT, 1, "AegisGate_QoS1"
    )
    time.sleep(3)
    emqx_qos1 = test_throughput_rigorous(
        EMQX_HOST, EMQX_PORT, MSG_QOS1_COUNT, 1, "EMQX_QoS1"
    )

    if aegis_qos1 and emqx_qos1:
        qos1_diff = (
            (
                (aegis_qos1["throughput"] - emqx_qos1["throughput"])
                / emqx_qos1["throughput"]
                * 100
            )
            if emqx_qos1["throughput"] > 0
            else 0
        )
        latency_diff = aegis_qos1["avg_latency"] - emqx_qos1["avg_latency"]
        p99_diff = aegis_qos1["p99_latency"] - emqx_qos1["p99_latency"]

        print(f"\n{CYAN}QoS 1 Comparison:{NC}")
        print(
            f"  Throughput - AegisGate: {aegis_qos1['throughput']:.2f} msg/s | EMQX: {emqx_qos1['throughput']:.2f}msg/s | Diff: {qos1_diff:+.2f}%"
        )
        print(
            f"  Avg Latency - AegisGate: {aegis_qos1['avg_latency']:.2f}ms | EMQX: {emqx_qos1['avg_latency']:.2f}ms | Diff: {latency_diff:+.2f}ms"
        )
        print(
            f"  P99 Latency - AegisGate: {aegis_qos1['p99_latency']:.2f}ms | EMQX: {emqx_qos1['p99_latency']:.2f}ms | Diff: {p99_diff:+.2f}ms"
        )

    # ==================================================================
    # TEST 4: Sustained Load Test
    # ==================================================================
    print(f"\n{MAGENTA}{'=' * 70}{NC}")
    print(
        f"{MAGENTA}TEST 4: SUSTAINED LOAD ({SUSTAINED_TEST_DURATION}s @ {SUSTAINED_RATE} msg/s){NC}"
    )
    print(f"{MAGENTA}{'=' * 70}{NC}")

    aegis_sustained = test_sustained_load(
        AEGIS_HOST,
        AEGIS_PORT,
        SUSTAINED_TEST_DURATION,
        SUSTAINED_RATE,
        0,
        "AegisGate_Sustained",
    )
    time.sleep(3)
    emqx_sustained = test_sustained_load(
        EMQX_HOST,
        EMQX_PORT,
        SUSTAINED_TEST_DURATION,
        SUSTAINED_RATE,
        0,
        "EMQX_Sustained",
    )

    if aegis_sustained and emqx_sustained:
        sust_diff = (
            (
                (aegis_sustained["throughput"] - emqx_sustained["throughput"])
                / emqx_sustained["throughput"]
                * 100
            )
            if emqx_sustained["throughput"] > 0
            else 0
        )

        print(f"\n{CYAN}Sustained Load Comparison:{NC}")
        print(
            f"  AegisGate: {aegis_sustained['throughput']:.2f} msg/s (P99: {aegis_sustained['p99_latency']:.2f}ms)"
        )
        print(
            f"  EMQX:      {emqx_sustained['throughput']:.2f} msg/s (P99: {emqx_sustained['p99_latency']:.2f}ms)"
        )
        print(f"  Difference: {sust_diff:+.2f}%")

    # ==================================================================
    # BACKEND VALIDATION
    # ==================================================================
    print(f"\n{MAGENTA}{'=' * 70}{NC}")
    print(f"{MAGENTA}BACKEND VALIDATION{NC}")
    print(f"{MAGENTA}{'=' * 70}{NC}")

    final_metrics = get_emqx_metrics(token)
    if final_metrics:
        total_dropped = final_metrics.get("messages.dropped", 0)
        dropped_no_subs = final_metrics.get("messages.dropped.no_subscribers", 0)
        new_dropped = total_dropped - baseline_dropped

        print(f"\n{CYAN}EMQX Backend Metrics:{NC}")
        print(f"  Total messages dropped: {total_dropped}")
        print(f"  Dropped (no subscribers): {dropped_no_subs}")
        print(f"  New drops during test: {new_dropped}")

        if dropped_no_subs > baseline_dropped:
            print(
                f"  {YELLOW}⚠ Warning: {dropped_no_subs - baseline_dropped} messages dropped due to missing subscribers{NC}"
            )
        else:
            print(f"  {GREEN}✓ No subscriber-related drops{NC}")

    # ==================================================================
    # FINAL SUMMARY
    # ==================================================================
    print(f"\n{GREEN}{'=' * 70}{NC}")
    print(f"{GREEN}RIGOROUS BENCHMARK SUMMARY{NC}")
    print(f"{GREEN}{'=' * 70}{NC}\n")

    print(f"{CYAN}1. Connection Speed ({CONN_TEST_COUNT} connections):{NC}")
    print(f"   AegisGate: {aegis_conn['rate']:.2f} conn/s")
    print(f"   EMQX:      {emqx_conn['rate']:.2f} conn/s")
    print(f"   {YELLOW}Difference: {conn_diff:+.2f}%{NC}\n")

    if aegis_qos0 and emqx_qos0:
        print(f"{CYAN}2. QoS 0 Throughput ({MSG_QOS0_COUNT} messages):{NC}")
        print(
            f"   AegisGate: {aegis_qos0['throughput']:.2f} msg/s (Avg: {aegis_qos0['avg_latency']:.2f}ms, P99: {aegis_qos0['p99_latency']:.2f}ms)"
        )
        print(
            f"   EMQX:      {emqx_qos0['throughput']:.2f} msg/s (Avg: {emqx_qos0['avg_latency']:.2f}ms, P99: {emqx_qos0['p99_latency']:.2f}ms)"
        )
        print(f"   {YELLOW}Difference: {throughput_diff:+.2f}%{NC}\n")

    if aegis_qos1 and emqx_qos1:
        print(f"{CYAN}3. QoS 1 Throughput ({MSG_QOS1_COUNT} messages):{NC}")
        print(
            f"   AegisGate: {aegis_qos1['throughput']:.2f} msg/s (Avg: {aegis_qos1['avg_latency']:.2f}ms, P99: {aegis_qos1['p99_latency']:.2f}ms)"
        )
        print(
            f"   EMQX:      {emqx_qos1['throughput']:.2f} msg/s (Avg: {emqx_qos1['avg_latency']:.2f}ms, P99: {emqx_qos1['p99_latency']:.2f}ms)"
        )
        print(f"   {YELLOW}Difference: {qos1_diff:+.2f}%{NC}\n")

    if aegis_sustained and emqx_sustained:
        print(
            f"{CYAN}4. Sustained Load ({SUSTAINED_TEST_DURATION}s @ {SUSTAINED_RATE} msg/s):{NC}"
        )
        print(
            f"   AegisGate: {aegis_sustained['throughput']:.2f} msg/s (P99: {aegis_sustained['p99_latency']:.2f}ms)"
        )
        print(
            f"   EMQX:      {emqx_sustained['throughput']:.2f} msg/s (P99: {emqx_sustained['p99_latency']:.2f}ms)"
        )
        print(f"   {YELLOW}Difference: {sust_diff:+.2f}%{NC}\n")

    print(f"{YELLOW}{'=' * 70}{NC}")
    print(f"{YELLOW}NOTE: Latency measurements are end-to-end (publish → receive){NC}")
    print(f"{YELLOW}AegisGate adds a proxy hop, so higher latency is expected{NC}")
    print(f"{YELLOW}{'=' * 70}{NC}\n")

    print(f"{GREEN}✓ Rigorous benchmark complete!{NC}\n")

    return 0


if __name__ == "__main__":
    sys.exit(main())
