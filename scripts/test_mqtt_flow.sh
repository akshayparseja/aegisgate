#!/usr/bin/env bash
set -euo pipefail

# End-to-end MQTT flow test using mosquitto clients.
# This script publishes a message via the proxy and verifies a subscriber receives it.
#
# Configurable via environment variables:
#  - PROXY_HOST (default 127.0.0.1)
#  - PROXY_PORT (default 8080)
#  - HEALTH_PORT (default 9090)
#  - TOPIC (default aegis/test)
#  - MESSAGE (default AegisGate_Flow_Check_<timestamp>)
#  - TIMEOUT (seconds to wait for health; default 15)

PROXY_HOST="${PROXY_HOST:-127.0.0.1}"
PROXY_PORT="${PROXY_PORT:-8080}"
HEALTH_PORT="${HEALTH_PORT:-9090}"
TOPIC="${TOPIC:-aegis/test}"
MESSAGE="${MESSAGE:-AegisGate_Flow_Check_$(date +%s)}"
TIMEOUT="${TIMEOUT:-15}"

# Explicit client IDs to avoid broker anonymous-client rejections during tests.
# These can be overridden by setting the env vars `SUB_CLIENT_ID` and `PUB_CLIENT_ID`.
SUB_CLIENT_ID="${SUB_CLIENT_ID:-aegisgate_sub_$(date +%s)}"
PUB_CLIENT_ID="${PUB_CLIENT_ID:-aegisgate_pub_$(date +%s)}"

# How long the subscriber should wait for the first message (mosquitto_sub -W).
# Increase from the previous hardcoded 5s to allow more time for network/broker response.
SUB_WAIT="${SUB_WAIT:-10}"

echo "--- Starting AegisGate Flow Test ---"
echo "Proxy: ${PROXY_HOST}:${PROXY_PORT}  Health: ${PROXY_HOST}:${HEALTH_PORT}  Topic: ${TOPIC}  SubID: ${SUB_CLIENT_ID}  PubID: ${PUB_CLIENT_ID}"
echo

# Ensure required tools are available
for cmd in mosquitto_pub mosquitto_sub curl; do
    if ! command -v "$cmd" >/dev/null 2>&1; then
        echo "Error: required command '$cmd' not found. Please install mosquitto-clients and curl."
        exit 2
    fi
done

# 1. Readiness Check: Wait for Proxy to be healthy
echo "Waiting for AegisGate to be healthy (timeout ${TIMEOUT}s)..."
count=0
until curl -s "http://${PROXY_HOST}:${HEALTH_PORT}/health" | grep -q "OK"; do
    sleep 1
    count=$((count+1))
    if [ $count -ge $TIMEOUT ]; then
        echo "Error: Timeout waiting for AegisGate health check."
        exit 1
    fi
done

# 2. Start a subscriber in the background
TMPFILE="$(mktemp /tmp/aegis_received.XXXXXX)"
cleanup() {
    rm -f "$TMPFILE"
}
trap cleanup EXIT

echo "Starting subscriber..."
# -C 1 => exit after receiving 1 message; -W 5 => wait up to 5 seconds for first message
mosquitto_sub -h "$PROXY_HOST" -p "$PROXY_PORT" -t "$TOPIC" -C 1 -W "$SUB_WAIT" -i "$SUB_CLIENT_ID" > "$TMPFILE" &
SUB_PID=$!

# Give the subscriber a moment to establish the TCP session
sleep 1

# 3. Publish a message through the proxy
echo "Publishing message to ${TOPIC} via proxy..."
mosquitto_pub -h "$PROXY_HOST" -p "$PROXY_PORT" -t "$TOPIC" -m "$MESSAGE" -i "$PUB_CLIENT_ID"

# 4. Wait for the subscriber to finish (it will exit after 1 message or timeout)
wait "$SUB_PID" || true

# 5. Verify results
RECEIVED="$(cat "$TMPFILE" 2>/dev/null || true)"
if [ "$RECEIVED" = "$MESSAGE" ]; then
    echo "SUCCESS: Message forwarded correctly."
    exit 0
else
    echo "FAILURE: Received '$RECEIVED' but expected '$MESSAGE'"
    echo "Proxy logs and metrics may help diagnose the issue."
    exit 1
fi
