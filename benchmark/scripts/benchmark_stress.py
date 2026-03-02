#!/usr/bin/env python3
"""
Comprehensive Stress Test Benchmark
AegisGate vs EMQX Direct Comparison
Tests: Multiple publishers, burst loads, mixed QoS, latency under load
"""

import random
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

# Stress test parameters - aggressive but safe for 512MB EMQX
BURST_CONNECTIONS = 150  # Burst connection test
MULTI_PUB_COUNT = 10  # Multiple concurrent publishers
MESSAGES_PER_PUB = 500  # Messages per publisher
BURST_MSG_COUNT = 2000  # Messages in burst test
MIXED_QOS_COUNT = 1000  # Mixed QoS test
CONCURRENT_SUBS = 5  # Multiple subscribers

# Colors
RED = "\033[91m"
GREEN = "\033[92m"
YELLOW = "\033[93m"
BLUE = "\033[94m"
MAGENTA = "\033[95m"
CYAN = "\033[96m"
NC = "\033[0m"


class StressStats:
    def __init__(self):
        self.latencies = []
        self.sent = 0
        self.received = 0
        self.errors = 0
        self.timeouts = 0
        self.lock = Lock()
        self.start_time = None
        self.end_time = None
        self.per_publisher_stats = defaultdict(lambda: {"sent": 0, "errors": 0})

    def record_latency(self, latency_ms):
        with self.lock:
            self.latencies.append(latency_ms)

    def increment_sent(self, publisher_id=None):
        with self.lock:
            self.sent += 1
            if publisher_id:
                self.per_publisher_stats[publisher_id]["sent"] += 1

    def increment_received(self):
        with self.lock:
            self.received += 1

    def increment_errors(self, publisher_id=None):
        with self.lock:
            self.errors += 1
            if publisher_id:
                self.per_publisher_stats[publisher_id]["errors"] += 1

    def increment_timeouts(self):
        with self.lock:
            self.timeouts += 1

    def get_stats(self):
        with self.lock:
            if self.latencies:
                sorted_latencies = sorted(self.latencies)
                n = len(sorted_latencies)
                return {
                    "sent": self.sent,
                    "received": self.received,
                    "errors": self.errors,
                    "timeouts": self.timeouts,
                    "avg_latency": statistics.mean(self.latencies),
                    "median_latency": statistics.median(self.latencies),
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
                "timeouts": self.timeouts,
                "avg_latency": 0,
                "median_latency": 0,
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


def test_burst_connections(host, port, count, name):
    """Test burst connection establishment (all at once)"""
    print(f"\n{CYAN}Testing {name} - BURST {count} connections{NC}")
    print(f"  {YELLOW}⚠ Attempting all connections simultaneously{NC}")

    clients = []
    connected_count = 0
    failed_count = 0
    lock = Lock()
    connect_times = []

    def on_connect(client, userdata, flags, rc):
        nonlocal connected_count, failed_count
        with lock:
            if rc == 0:
                connected_count += 1
                connect_times.append(time.time())
            else:
                failed_count += 1

    start_time = time.time()

    # Burst: create all connections at once
    for i in range(count):
        client = mqtt.Client(client_id=f"{name}_burst_{i}", clean_session=True)
        client.on_connect = on_connect
        try:
            client.connect_async(host, port, 60)
            client.loop_start()
            clients.append(client)
        except Exception as e:
            with lock:
                failed_count += 1

    # Wait for connections to establish
    time.sleep(5)
    connect_time = time.time() - start_time

    print(f"  {GREEN}✓{NC} Connected: {connected_count}/{count}")
    if failed_count > 0:
        print(f"  {RED}✗{NC} Failed: {failed_count}")
    print(f"  {GREEN}✓{NC} Total time: {connect_time:.3f}s")
    print(f"  {GREEN}✓{NC} Effective rate: {connected_count / connect_time:.2f} conn/s")

    # Check EMQX backend
    token = get_emqx_token()
    if token:
        stats = get_emqx_stats(token)
        if stats:
            backend_conns = stats.get("connections.count", 0)
            print(f"  Backend connections: {backend_conns}")
            if backend_conns != connected_count:
                print(
                    f"  {YELLOW}⚠ Mismatch: Client count ({connected_count}) != Backend ({backend_conns}){NC}"
                )

    # Cleanup
    time.sleep(1)
    print(f"  Disconnecting...")
    for client in clients:
        try:
            client.loop_stop()
            client.disconnect()
        except:
            pass

    time.sleep(2)

    return {
        "connected": connected_count,
        "failed": failed_count,
        "time": connect_time,
        "rate": connected_count / connect_time if connect_time > 0 else 0,
    }


def test_multiple_publishers(host, port, num_publishers, msgs_per_pub, qos, name):
    """Test multiple concurrent publishers"""
    print(
        f"\n{CYAN}Testing {name} - {num_publishers} concurrent publishers x {msgs_per_pub} msgs (QoS {qos}){NC}"
    )

    stats = StressStats()
    topic_base = f"benchmark/multipub/{name}"
    messages_timestamps = {}
    all_publishers_done = Event()
    publishers_done_count = 0
    publishers_done_lock = Lock()

    # Multiple subscribers (one per publisher)
    subscribers = []

    def create_subscriber(sub_id):
        def on_sub_connect(client, userdata, flags, rc):
            if rc == 0:
                # Subscribe to all publisher topics
                for pub_id in range(num_publishers):
                    client.subscribe(f"{topic_base}/{pub_id}", qos=qos)

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

        sub_client = mqtt.Client(client_id=f"{name}_sub_{sub_id}", clean_session=True)
        sub_client.on_connect = on_sub_connect
        sub_client.on_message = on_message
        return sub_client

    # Setup subscribers
    print(f"  Setting up {CONCURRENT_SUBS} subscribers...")
    for i in range(CONCURRENT_SUBS):
        sub = create_subscriber(i)
        try:
            sub.connect(host, port, 60)
            sub.loop_start()
            subscribers.append(sub)
        except Exception as e:
            print(f"{RED}Subscriber {i} error: {e}{NC}")

    time.sleep(1)
    print(f"  {GREEN}✓{NC} Subscribers ready")

    # Publisher worker function
    def publisher_worker(pub_id):
        nonlocal publishers_done_count

        pub_topic = f"{topic_base}/{pub_id}"
        pub_client = mqtt.Client(client_id=f"{name}_pub_{pub_id}", clean_session=True)

        try:
            pub_client.connect(host, port, 60)
            pub_client.loop_start()
            time.sleep(0.2)

            # Publish messages
            for i in range(msgs_per_pub):
                msg_id = f"p{pub_id}_m{i}"
                payload = f"multipub_msg_{msg_id}"
                send_time = time.time()
                messages_timestamps[msg_id] = send_time

                result = pub_client.publish(pub_topic, payload, qos=qos)
                if result.rc == mqtt.MQTT_ERR_SUCCESS:
                    stats.increment_sent(pub_id)
                else:
                    stats.increment_errors(pub_id)

            pub_client.loop_stop()
            pub_client.disconnect()

            with publishers_done_lock:
                publishers_done_count += 1
                if publishers_done_count == num_publishers:
                    all_publishers_done.set()

        except Exception as e:
            print(f"{RED}Publisher {pub_id} error: {e}{NC}")
            with publishers_done_lock:
                publishers_done_count += 1
                if publishers_done_count == num_publishers:
                    all_publishers_done.set()

    # Start all publishers
    print(f"  Starting {num_publishers} publishers...")
    stats.start_time = time.time()

    publisher_threads = []
    for pub_id in range(num_publishers):
        thread = Thread(target=publisher_worker, args=(pub_id,))
        thread.start()
        publisher_threads.append(thread)

    # Wait for all publishers to finish
    all_publishers_done.wait(timeout=60)
    for thread in publisher_threads:
        thread.join(timeout=5)

    stats.end_time = time.time()

    # Wait for remaining messages
    print(f"  Waiting for messages to be received...")
    time.sleep(3)

    # Results
    final_stats = stats.get_stats()
    total_time = stats.end_time - stats.start_time
    throughput = final_stats["received"] / total_time if total_time > 0 else 0
    expected_total = num_publishers * msgs_per_pub
    loss = expected_total - final_stats["received"]
    loss_pct = (loss / expected_total * 100) if expected_total > 0 else 0

    print(f"\n  {CYAN}Results:{NC}")
    print(f"    Expected: {expected_total} messages")
    print(f"    Sent: {final_stats['sent']}/{expected_total}")
    print(f"    Received: {final_stats['received']}/{expected_total}")
    print(f"    Loss: {loss} ({loss_pct:.2f}%)")
    print(f"    Time: {total_time:.3f}s")
    print(f"    Throughput: {GREEN}{throughput:.2f} msg/s{NC}")
    print(f"    Avg Latency: {final_stats['avg_latency']:.3f}ms")
    print(f"    P99 Latency: {final_stats['p99_latency']:.3f}ms")
    print(f"    Max Latency: {final_stats['max_latency']:.3f}ms")

    # Cleanup
    for sub in subscribers:
        try:
            sub.loop_stop()
            sub.disconnect()
        except:
            pass

    time.sleep(1)

    return {
        "sent": final_stats["sent"],
        "received": final_stats["received"],
        "expected": expected_total,
        "loss": loss,
        "loss_pct": loss_pct,
        "throughput": throughput,
        "avg_latency": final_stats["avg_latency"],
        "p99_latency": final_stats["p99_latency"],
        "max_latency": final_stats["max_latency"],
        "total_time": total_time,
    }


def test_mixed_qos_workload(host, port, msg_count, name):
    """Test mixed QoS 0 and QoS 1 messages"""
    print(f"\n{CYAN}Testing {name} - {msg_count} messages (MIXED QoS 0/1){NC}")

    stats = StressStats()
    topic = f"benchmark/mixed/{name}"
    messages_timestamps = {}
    received_event = Event()

    qos0_sent = 0
    qos1_sent = 0
    qos0_received = 0
    qos1_received = 0

    # Subscriber
    def on_sub_connect(client, userdata, flags, rc):
        if rc == 0:
            client.subscribe(topic, qos=1)  # Subscribe with QoS 1 to receive all

    def on_message(client, userdata, msg):
        nonlocal qos0_received, qos1_received
        recv_time = time.time()
        try:
            payload = msg.payload.decode()
            parts = payload.split("_")
            msg_qos = int(parts[2].replace("qos", ""))
            msg_id = parts[-1]

            if msg_qos == 0:
                qos0_received += 1
            else:
                qos1_received += 1

            if msg_id in messages_timestamps:
                latency_ms = (recv_time - messages_timestamps[msg_id]) * 1000
                stats.record_latency(latency_ms)
            stats.increment_received()

            if stats.received >= msg_count:
                received_event.set()
        except Exception as e:
            stats.increment_errors()

    sub_client = mqtt.Client(client_id=f"{name}_sub_mixed", clean_session=True)
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
    pub_client = mqtt.Client(client_id=f"{name}_pub_mixed", clean_session=True)

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

    # Publish mixed QoS messages (50% QoS 0, 50% QoS 1)
    print(f"  Publishing {msg_count} mixed QoS messages...")
    start_time = time.time()

    for i in range(msg_count):
        qos = 0 if i % 2 == 0 else 1  # Alternate between QoS 0 and 1
        msg_id = f"{i}"
        payload = f"mixed_msg_qos{qos}_{msg_id}"
        send_time = time.time()
        messages_timestamps[msg_id] = send_time

        result = pub_client.publish(topic, payload, qos=qos)
        if result.rc == mqtt.MQTT_ERR_SUCCESS:
            stats.increment_sent()
            if qos == 0:
                qos0_sent += 1
            else:
                qos1_sent += 1
        else:
            stats.increment_errors()

        if (i + 1) % 500 == 0:
            print(f"    Sent: {i + 1}/{msg_count}")

    # Wait for messages
    received_event.wait(timeout=10)
    time.sleep(1)

    total_time = time.time() - start_time
    final_stats = stats.get_stats()
    throughput = final_stats["received"] / total_time if total_time > 0 else 0

    print(f"\n  {CYAN}Results:{NC}")
    print(f"    QoS 0 sent: {qos0_sent}, received: {qos0_received}")
    print(f"    QoS 1 sent: {qos1_sent}, received: {qos1_received}")
    print(f"    Total received: {final_stats['received']}/{msg_count}")
    print(f"    Throughput: {GREEN}{throughput:.2f} msg/s{NC}")
    print(f"    Avg Latency: {final_stats['avg_latency']:.3f}ms")
    print(f"    P99 Latency: {final_stats['p99_latency']:.3f}ms")

    # Cleanup
    pub_client.loop_stop()
    pub_client.disconnect()
    sub_client.loop_stop()
    sub_client.disconnect()

    time.sleep(1)

    return {
        "sent": final_stats["sent"],
        "received": final_stats["received"],
        "qos0_sent": qos0_sent,
        "qos1_sent": qos1_sent,
        "qos0_received": qos0_received,
        "qos1_received": qos1_received,
        "throughput": throughput,
        "avg_latency": final_stats["avg_latency"],
        "p99_latency": final_stats["p99_latency"],
    }


def test_burst_messages(host, port, burst_count, qos, name):
    """Test burst message publishing (all messages as fast as possible)"""
    print(f"\n{CYAN}Testing {name} - BURST {burst_count} messages (QoS {qos}){NC}")

    stats = StressStats()
    topic = f"benchmark/burst/{name}"
    messages_timestamps = {}
    received_event = Event()

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
            if stats.received >= burst_count:
                received_event.set()
        except:
            stats.increment_errors()

    sub_client = mqtt.Client(client_id=f"{name}_sub_burst", clean_session=True)
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
    pub_client = mqtt.Client(client_id=f"{name}_pub_burst", clean_session=True)

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

    # Publish burst (as fast as possible, no delays)
    print(f"  Publishing {burst_count} messages in BURST mode...")
    start_time = time.time()

    for i in range(burst_count):
        msg_id = f"{i}"
        payload = f"burst_msg_{msg_id}"
        send_time = time.time()
        messages_timestamps[msg_id] = send_time

        result = pub_client.publish(topic, payload, qos=qos)
        if result.rc == mqtt.MQTT_ERR_SUCCESS:
            stats.increment_sent()
        else:
            stats.increment_errors()

    publish_time = time.time() - start_time
    print(
        f"  Publishing completed in {publish_time:.3f}s ({burst_count / publish_time:.2f} msg/s)"
    )

    # Wait for messages
    print(f"  Waiting for messages to be received...")
    received_event.wait(timeout=15)
    time.sleep(2)

    total_time = time.time() - start_time
    final_stats = stats.get_stats()
    throughput = final_stats["received"] / total_time if total_time > 0 else 0
    loss = burst_count - final_stats["received"]

    print(f"\n  {CYAN}Results:{NC}")
    print(f"    Sent: {final_stats['sent']}/{burst_count} in {publish_time:.3f}s")
    print(f"    Received: {final_stats['received']}/{burst_count}")
    print(f"    Loss: {loss} ({loss / burst_count * 100:.2f}%)")
    print(f"    Effective throughput: {GREEN}{throughput:.2f} msg/s{NC}")
    print(f"    Avg Latency: {final_stats['avg_latency']:.3f}ms")
    print(f"    P99 Latency: {final_stats['p99_latency']:.3f}ms")
    print(f"    Max Latency: {final_stats['max_latency']:.3f}ms")

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
        "publish_time": publish_time,
        "total_time": total_time,
        "throughput": throughput,
        "avg_latency": final_stats["avg_latency"],
        "p99_latency": final_stats["p99_latency"],
        "max_latency": final_stats["max_latency"],
    }


def main():
    print(f"{MAGENTA}{'=' * 70}{NC}")
    print(f"{MAGENTA}COMPREHENSIVE STRESS TEST BENCHMARK{NC}")
    print(f"{MAGENTA}AegisGate vs EMQX Direct Comparison{NC}")
    print(
        f"{MAGENTA}Testing under high load, concurrent publishers, burst scenarios{NC}"
    )
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
    print(
        f"\n{YELLOW}⚠ WARNING: These are stress tests - monitor system resources{NC}\n"
    )

    baseline_metrics = get_emqx_metrics(token)

    # ==================================================================
    # TEST 1: Burst Connection Test
    # ==================================================================
    print(f"\n{MAGENTA}{'=' * 70}{NC}")
    print(
        f"{MAGENTA}TEST 1: BURST CONNECTIONS ({BURST_CONNECTIONS} connections simultaneously){NC}"
    )
    print(f"{MAGENTA}{'=' * 70}{NC}")

    aegis_burst_conn = test_burst_connections(
        AEGIS_HOST, AEGIS_PORT, BURST_CONNECTIONS, "AegisGate"
    )
    time.sleep(3)
    emqx_burst_conn = test_burst_connections(
        EMQX_HOST, EMQX_PORT, BURST_CONNECTIONS, "EMQX"
    )

    print(f"\n{CYAN}Burst Connection Comparison:{NC}")
    print(
        f"  AegisGate: {aegis_burst_conn['connected']}/{BURST_CONNECTIONS} in {aegis_burst_conn['time']:.2f}s ({aegis_burst_conn['failed']} failed)"
    )
    print(
        f"  EMQX:      {emqx_burst_conn['connected']}/{BURST_CONNECTIONS} in {emqx_burst_conn['time']:.2f}s ({emqx_burst_conn['failed']} failed)"
    )

    # ==================================================================
    # TEST 2: Multiple Concurrent Publishers
    # ==================================================================
    print(f"\n{MAGENTA}{'=' * 70}{NC}")
    print(
        f"{MAGENTA}TEST 2: MULTIPLE PUBLISHERS ({MULTI_PUB_COUNT} pubs x {MESSAGES_PER_PUB} msgs){NC}"
    )
    print(f"{MAGENTA}{'=' * 70}{NC}")

    aegis_multipub = test_multiple_publishers(
        AEGIS_HOST,
        AEGIS_PORT,
        MULTI_PUB_COUNT,
        MESSAGES_PER_PUB,
        0,
        "AegisGate_MultiPub",
    )
    time.sleep(3)
    emqx_multipub = test_multiple_publishers(
        EMQX_HOST, EMQX_PORT, MULTI_PUB_COUNT, MESSAGES_PER_PUB, 0, "EMQX_MultiPub"
    )

    if aegis_multipub and emqx_multipub:
        print(f"\n{CYAN}Multiple Publishers Comparison:{NC}")
        print(
            f"  AegisGate: {aegis_multipub['throughput']:.2f} msg/s (Loss: {aegis_multipub['loss_pct']:.2f}%, P99: {aegis_multipub['p99_latency']:.2f}ms)"
        )
        print(
            f"  EMQX:      {emqx_multipub['throughput']:.2f} msg/s (Loss: {emqx_multipub['loss_pct']:.2f}%, P99: {emqx_multipub['p99_latency']:.2f}ms)"
        )

    # ==================================================================
    # TEST 3: Burst Message Test
    # ==================================================================
    print(f"\n{MAGENTA}{'=' * 70}{NC}")
    print(f"{MAGENTA}TEST 3: BURST MESSAGES ({BURST_MSG_COUNT} messages, QoS 0){NC}")
    print(f"{MAGENTA}{'=' * 70}{NC}")

    aegis_burst = test_burst_messages(
        AEGIS_HOST, AEGIS_PORT, BURST_MSG_COUNT, 0, "AegisGate_Burst"
    )
    time.sleep(3)
    emqx_burst = test_burst_messages(
        EMQX_HOST, EMQX_PORT, BURST_MSG_COUNT, 0, "EMQX_Burst"
    )

    if aegis_burst and emqx_burst:
        print(f"\n{CYAN}Burst Messages Comparison:{NC}")
        print(
            f"  AegisGate: {aegis_burst['throughput']:.2f} msg/s (Loss: {aegis_burst['loss']}, Max latency: {aegis_burst['max_latency']:.2f}ms)"
        )
        print(
            f"  EMQX:      {emqx_burst['throughput']:.2f} msg/s (Loss: {emqx_burst['loss']}, Max latency: {emqx_burst['max_latency']:.2f}ms)"
        )

    # ==================================================================
    # TEST 4: Mixed QoS Workload
    # ==================================================================
    print(f"\n{MAGENTA}{'=' * 70}{NC}")
    print(f"{MAGENTA}TEST 4: MIXED QoS WORKLOAD ({MIXED_QOS_COUNT} messages){NC}")
    print(f"{MAGENTA}{'=' * 70}{NC}")

    aegis_mixed = test_mixed_qos_workload(
        AEGIS_HOST, AEGIS_PORT, MIXED_QOS_COUNT, "AegisGate_Mixed"
    )
    time.sleep(3)
    emqx_mixed = test_mixed_qos_workload(
        EMQX_HOST, EMQX_PORT, MIXED_QOS_COUNT, "EMQX_Mixed"
    )

    if aegis_mixed and emqx_mixed:
        print(f"\n{CYAN}Mixed QoS Comparison:{NC}")
        print(
            f"  AegisGate: {aegis_mixed['throughput']:.2f} msg/s (QoS0: {aegis_mixed['qos0_received']}/{aegis_mixed['qos0_sent']}, QoS1: {aegis_mixed['qos1_received']}/{aegis_mixed['qos1_sent']})"
        )
        print(
            f"  EMQX:      {emqx_mixed['throughput']:.2f} msg/s (QoS0: {emqx_mixed['qos0_received']}/{emqx_mixed['qos0_sent']}, QoS1: {emqx_mixed['qos1_received']}/{emqx_mixed['qos1_sent']})"
        )

    # ==================================================================
    # BACKEND VALIDATION
    # ==================================================================
    print(f"\n{MAGENTA}{'=' * 70}{NC}")
    print(f"{MAGENTA}BACKEND VALIDATION{NC}")
    print(f"{MAGENTA}{'=' * 70}{NC}")

    final_metrics = get_emqx_metrics(token)
    if final_metrics and baseline_metrics:
        baseline_dropped = baseline_metrics.get("messages.dropped", 0)
        final_dropped = final_metrics.get("messages.dropped", 0)
        dropped_no_subs = final_metrics.get("messages.dropped.no_subscribers", 0)
        new_drops = final_dropped - baseline_dropped

        print(f"\n{CYAN}EMQX Backend Metrics:{NC}")
        print(f"  Total messages dropped: {final_dropped}")
        print(f"  New drops during stress tests: {new_drops}")
        print(f"  Dropped (no subscribers): {dropped_no_subs}")

        if new_drops > 0:
            print(
                f"  {YELLOW}⚠ {new_drops} messages were dropped during stress testing{NC}"
            )
        else:
            print(f"  {GREEN}✓ No message drops during tests{NC}")

    # ==================================================================
    # FINAL SUMMARY
    # ==================================================================
    print(f"\n{GREEN}{'=' * 70}{NC}")
    print(f"{GREEN}STRESS TEST SUMMARY{NC}")
    print(f"{GREEN}{'=' * 70}{NC}\n")

    print(f"{CYAN}1. Burst Connections ({BURST_CONNECTIONS} simultaneous):{NC}")
    print(
        f"   AegisGate: {aegis_burst_conn['connected']} connected, {aegis_burst_conn['failed']} failed"
    )
    print(
        f"   EMQX:      {emqx_burst_conn['connected']} connected, {emqx_burst_conn['failed']} failed\n"
    )

    if aegis_multipub and emqx_multipub:
        print(f"{CYAN}2. Multiple Publishers ({MULTI_PUB_COUNT} concurrent):{NC}")
        print(
            f"   AegisGate: {aegis_multipub['throughput']:.2f} msg/s, {aegis_multipub['loss_pct']:.2f}% loss"
        )
        print(
            f"   EMQX:      {emqx_multipub['throughput']:.2f} msg/s, {emqx_multipub['loss_pct']:.2f}% loss\n"
        )

    if aegis_burst and emqx_burst:
        print(f"{CYAN}3. Burst Messages ({BURST_MSG_COUNT} rapid-fire):{NC}")
        print(
            f"   AegisGate: {aegis_burst['throughput']:.2f} msg/s, Max latency: {aegis_burst['max_latency']:.2f}ms"
        )
        print(
            f"   EMQX:      {emqx_burst['throughput']:.2f} msg/s, Max latency: {emqx_burst['max_latency']:.2f}ms\n"
        )

    if aegis_mixed and emqx_mixed:
        print(f"{CYAN}4. Mixed QoS Workload:{NC}")
        print(f"   AegisGate: {aegis_mixed['throughput']:.2f} msg/s")
        print(f"   EMQX:      {emqx_mixed['throughput']:.2f} msg/s\n")

    print(f"{YELLOW}{'=' * 70}{NC}")
    print(f"{YELLOW}Key Insights from Stress Testing:{NC}")
    print(f"  • Burst connections test reveals connection handling limits")
    print(f"  • Multiple publishers test concurrent load handling")
    print(f"  • Burst messages test queue/buffer management")
    print(f"  • Mixed QoS tests protocol handling under mixed workloads")
    print(f"{YELLOW}{'=' * 70}{NC}\n")

    print(f"{GREEN}✓ Comprehensive stress test complete!{NC}\n")

    return 0


if __name__ == "__main__":
    sys.exit(main())
