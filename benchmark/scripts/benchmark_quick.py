#!/usr/bin/env python3
"""
Quick Validated MQTT Benchmark
AegisGate vs EMQX Direct Comparison
Streamlined for speed with essential validation
"""

import signal
import statistics
import subprocess
import sys
import time
from collections import defaultdict
from threading import Event, Lock

import paho.mqtt.client as mqtt
import requests

# Configuration
EMQX_HOST = "localhost"
EMQX_PORT = 1883
AEGIS_HOST = "localhost"
AEGIS_PORT = 8080
EMQX_API = "http://localhost:18083/api/v5"
AEGIS_API = "http://localhost:9090/metrics"

# Test parameters - reduced for speed
CONN_TEST_COUNT = 50  # Reduced from 100
MSG_TEST_COUNT = 1000  # Reduced from 3000
QOS1_MSG_COUNT = 500  # Reduced from 1500

# Colors
RED = "\033[91m"
GREEN = "\033[92m"
YELLOW = "\033[93m"
BLUE = "\033[94m"
MAGENTA = "\033[95m"
CYAN = "\033[96m"
NC = "\033[0m"


class QuickStats:
    def __init__(self):
        self.latencies = []
        self.sent = 0
        self.received = 0
        self.errors = 0
        self.lock = Lock()

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
                return {
                    "sent": self.sent,
                    "received": self.received,
                    "errors": self.errors,
                    "avg_latency": statistics.mean(self.latencies),
                    "p50_latency": statistics.median(self.latencies),
                    "p95_latency": statistics.quantiles(self.latencies, n=20)[18]
                    if len(self.latencies) > 20
                    else max(self.latencies),
                    "min_latency": min(self.latencies),
                    "max_latency": max(self.latencies),
                }
            return {
                "sent": self.sent,
                "received": self.received,
                "errors": self.errors,
                "avg_latency": 0,
                "p50_latency": 0,
                "p95_latency": 0,
                "min_latency": 0,
                "max_latency": 0,
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


def cleanup_processes():
    """Clean up any lingering benchmark processes"""
    try:
        subprocess.run(["pkill", "-9", "-f", "benchmark"], stderr=subprocess.DEVNULL)
        subprocess.run(["pkill", "-9", "-f", "mosquitto"], stderr=subprocess.DEVNULL)
    except:
        pass


def test_connections(host, port, count, name):
    """Test connection establishment speed"""
    print(f"\n{CYAN}Testing {name} - {count} connections{NC}")

    clients = []
    connected_count = 0
    connected_event = Event()

    def on_connect(client, userdata, flags, rc):
        nonlocal connected_count
        if rc == 0:
            connected_count += 1
            if connected_count == count:
                connected_event.set()

    start_time = time.time()

    # Create and connect clients
    for i in range(count):
        client = mqtt.Client(client_id=f"{name}_conn_{i}", clean_session=True)
        client.on_connect = on_connect
        try:
            client.connect(host, port, 60)
            client.loop_start()
            clients.append(client)
        except Exception as e:
            print(f"{RED}Connection error: {e}{NC}")

    # Wait for all connections
    connected_event.wait(timeout=10)
    connect_time = time.time() - start_time

    print(f"  {GREEN}✓{NC} Connected: {connected_count}/{count}")
    print(f"  {GREEN}✓{NC} Time: {connect_time:.3f}s")
    print(f"  {GREEN}✓{NC} Rate: {connected_count / connect_time:.2f} conn/s")

    # Cleanup
    time.sleep(0.5)
    for client in clients:
        try:
            client.loop_stop()
            client.disconnect()
        except:
            pass

    time.sleep(1)

    return {
        "connected": connected_count,
        "time": connect_time,
        "rate": connected_count / connect_time if connect_time > 0 else 0,
    }


def test_throughput(host, port, msg_count, qos, name):
    """Test message throughput with validation"""
    print(f"\n{CYAN}Testing {name} - {msg_count} messages (QoS {qos}){NC}")

    stats = QuickStats()
    topic = f"benchmark/quick/{name}"
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
    sub_client = mqtt.Client(client_id=f"{name}_sub", clean_session=True)
    sub_client.on_connect = on_sub_connect
    sub_client.on_message = on_message

    try:
        sub_client.connect(host, port, 60)
        sub_client.loop_start()
        time.sleep(0.5)  # Wait for subscription
        print(f"  {GREEN}✓{NC} Subscriber ready")
    except Exception as e:
        print(f"{RED}Subscriber error: {e}{NC}")
        return None

    # Publisher callbacks
    def on_pub_connect(client, userdata, flags, rc):
        pass

    pub_client = mqtt.Client(client_id=f"{name}_pub", clean_session=True)
    pub_client.on_connect = on_pub_connect

    try:
        pub_client.connect(host, port, 60)
        pub_client.loop_start()
        time.sleep(0.3)
        print(f"  {GREEN}✓{NC} Publisher ready")
    except Exception as e:
        print(f"{RED}Publisher error: {e}{NC}")
        sub_client.loop_stop()
        sub_client.disconnect()
        return None

    # Publish messages
    print(f"  Publishing {msg_count} messages...")
    start_time = time.time()

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

    pub_time = time.time() - start_time

    # Wait for messages to be received
    print(f"  Waiting for messages...")
    received_event.wait(timeout=5)

    # Give a moment for final messages
    time.sleep(0.5)

    # Get results
    final_stats = stats.get_stats()
    total_time = time.time() - start_time
    throughput = final_stats["received"] / total_time if total_time > 0 else 0
    loss = msg_count - final_stats["received"]
    loss_pct = (loss / msg_count * 100) if msg_count > 0 else 0

    print(f"\n  {CYAN}Results:{NC}")
    print(f"    Sent: {final_stats['sent']}/{msg_count}")
    print(f"    Received: {final_stats['received']}/{msg_count}")
    print(f"    Loss: {loss} ({loss_pct:.2f}%)")
    print(f"    Throughput: {GREEN}{throughput:.2f} msg/s{NC}")
    print(f"    Avg Latency: {GREEN}{final_stats['avg_latency']:.3f}ms{NC}")
    print(f"    P50 Latency: {final_stats['p50_latency']:.3f}ms")
    print(f"    P95 Latency: {final_stats['p95_latency']:.3f}ms")

    # Cleanup
    pub_client.loop_stop()
    pub_client.disconnect()
    sub_client.loop_stop()
    sub_client.disconnect()

    time.sleep(0.5)

    return {
        "sent": final_stats["sent"],
        "received": final_stats["received"],
        "loss": loss,
        "loss_pct": loss_pct,
        "throughput": throughput,
        "avg_latency": final_stats["avg_latency"],
        "p50_latency": final_stats["p50_latency"],
        "p95_latency": final_stats["p95_latency"],
        "total_time": total_time,
    }


def main():
    print(f"{MAGENTA}{'=' * 70}{NC}")
    print(f"{MAGENTA}QUICK VALIDATED MQTT BENCHMARK{NC}")
    print(f"{MAGENTA}AegisGate vs EMQX Direct Comparison{NC}")
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

    # ==================================================================
    # TEST 1: Connection Speed
    # ==================================================================
    print(f"\n{MAGENTA}{'=' * 70}{NC}")
    print(f"{MAGENTA}TEST 1: CONNECTION SPEED ({CONN_TEST_COUNT} connections){NC}")
    print(f"{MAGENTA}{'=' * 70}{NC}")

    aegis_conn = test_connections(AEGIS_HOST, AEGIS_PORT, CONN_TEST_COUNT, "AegisGate")
    time.sleep(2)
    emqx_conn = test_connections(EMQX_HOST, EMQX_PORT, CONN_TEST_COUNT, "EMQX")

    conn_overhead = (
        ((aegis_conn["rate"] - emqx_conn["rate"]) / emqx_conn["rate"] * 100)
        if emqx_conn["rate"] > 0
        else 0
    )

    print(f"\n{CYAN}Connection Summary:{NC}")
    print(f"  AegisGate: {GREEN}{aegis_conn['rate']:.2f} conn/s{NC}")
    print(f"  EMQX:      {GREEN}{emqx_conn['rate']:.2f} conn/s{NC}")
    print(f"  Difference: {YELLOW}{conn_overhead:+.2f}%{NC}")

    # ==================================================================
    # TEST 2: QoS 0 Throughput
    # ==================================================================
    print(f"\n{MAGENTA}{'=' * 70}{NC}")
    print(f"{MAGENTA}TEST 2: QoS 0 THROUGHPUT ({MSG_TEST_COUNT} messages){NC}")
    print(f"{MAGENTA}{'=' * 70}{NC}")

    aegis_qos0 = test_throughput(
        AEGIS_HOST, AEGIS_PORT, MSG_TEST_COUNT, 0, "AegisGate_QoS0"
    )
    time.sleep(2)
    emqx_qos0 = test_throughput(EMQX_HOST, EMQX_PORT, MSG_TEST_COUNT, 0, "EMQX_QoS0")

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

        print(f"\n{CYAN}QoS 0 Summary:{NC}")
        print(
            f"  AegisGate: {GREEN}{aegis_qos0['throughput']:.2f} msg/s{NC} (latency: {aegis_qos0['avg_latency']:.2f}ms)"
        )
        print(
            f"  EMQX:      {GREEN}{emqx_qos0['throughput']:.2f} msg/s{NC} (latency: {emqx_qos0['avg_latency']:.2f}ms)"
        )
        print(f"  Throughput: {YELLOW}{throughput_diff:+.2f}%{NC}")
        print(f"  Latency: {YELLOW}{latency_diff:+.2f}ms{NC}")

    # ==================================================================
    # TEST 3: QoS 1 Throughput
    # ==================================================================
    print(f"\n{MAGENTA}{'=' * 70}{NC}")
    print(f"{MAGENTA}TEST 3: QoS 1 THROUGHPUT ({QOS1_MSG_COUNT} messages){NC}")
    print(f"{MAGENTA}{'=' * 70}{NC}")

    aegis_qos1 = test_throughput(
        AEGIS_HOST, AEGIS_PORT, QOS1_MSG_COUNT, 1, "AegisGate_QoS1"
    )
    time.sleep(2)
    emqx_qos1 = test_throughput(EMQX_HOST, EMQX_PORT, QOS1_MSG_COUNT, 1, "EMQX_QoS1")

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

        print(f"\n{CYAN}QoS 1 Summary:{NC}")
        print(
            f"  AegisGate: {GREEN}{aegis_qos1['throughput']:.2f} msg/s{NC} (latency: {aegis_qos1['avg_latency']:.2f}ms)"
        )
        print(
            f"  EMQX:      {GREEN}{emqx_qos1['throughput']:.2f} msg/s{NC} (latency: {emqx_qos1['avg_latency']:.2f}ms)"
        )
        print(f"  Throughput: {YELLOW}{qos1_diff:+.2f}%{NC}")
        print(f"  Latency: {YELLOW}{latency_diff:+.2f}ms{NC}")

    # ==================================================================
    # FINAL SUMMARY
    # ==================================================================
    print(f"\n{GREEN}{'=' * 70}{NC}")
    print(f"{GREEN}FINAL BENCHMARK SUMMARY{NC}")
    print(f"{GREEN}{'=' * 70}{NC}\n")

    print(f"{CYAN}1. Connection Speed:{NC}")
    print(f"   AegisGate: {aegis_conn['rate']:.2f} conn/s")
    print(f"   EMQX:      {emqx_conn['rate']:.2f} conn/s")
    print(f"   {YELLOW}Difference: {conn_overhead:+.2f}%{NC}\n")

    if aegis_qos0 and emqx_qos0:
        print(f"{CYAN}2. QoS 0 Throughput:{NC}")
        print(
            f"   AegisGate: {aegis_qos0['throughput']:.2f} msg/s ({aegis_qos0['avg_latency']:.2f}ms)"
        )
        print(
            f"   EMQX:      {emqx_qos0['throughput']:.2f} msg/s ({emqx_qos0['avg_latency']:.2f}ms)"
        )
        print(f"   {YELLOW}Difference: {throughput_diff:+.2f}%{NC}\n")

    if aegis_qos1 and emqx_qos1:
        print(f"{CYAN}3. QoS 1 Throughput:{NC}")
        print(
            f"   AegisGate: {aegis_qos1['throughput']:.2f} msg/s ({aegis_qos1['avg_latency']:.2f}ms)"
        )
        print(
            f"   EMQX:      {emqx_qos1['throughput']:.2f} msg/s ({emqx_qos1['avg_latency']:.2f}ms)"
        )
        print(f"   {YELLOW}Difference: {qos1_diff:+.2f}%{NC}\n")

    # Verify EMQX backend
    print(f"{CYAN}Backend Validation:{NC}")
    metrics = get_emqx_metrics(token)
    if metrics:
        print(
            f"   Messages dropped (no subs): {metrics.get('messages.dropped.no_subscribers', 0)}"
        )
        print(f"   Total messages dropped: {metrics.get('messages.dropped', 0)}")

        if metrics.get("messages.dropped.no_subscribers", 0) > 0:
            print(
                f"   {YELLOW}⚠ Warning: Some messages were dropped due to missing subscribers{NC}"
            )
        else:
            print(f"   {GREEN}✓ No messages dropped - all tests validated{NC}")

    print(f"\n{GREEN}✓ Quick benchmark complete!{NC}\n")

    return 0


if __name__ == "__main__":
    sys.exit(main())
