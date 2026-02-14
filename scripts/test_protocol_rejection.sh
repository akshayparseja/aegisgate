#!/usr/bin/env bash
#
# Test script: protocol rejection metric validation
#
# Purpose:
# - Sends a deliberately malformed MQTT CONNECT frame to the AegisGate proxy
#   to trigger protocol-level rejection logic.
# - Verifies that the Prometheus metric `aegis_protocol_rejections_total`
#   increments as a result.
#
# Notes:
# - This is a quick integration-style test intended to run on a machine where
#   the proxy is reachable at $PROXY_HOST:$PROXY_PORT and the metrics endpoint
#   is available at $METRICS_HOST:$METRICS_PORT.
# - The malformed payload used here is a CONNECT fixed header with an
#   intentionally truncated Remaining Length (continuation bit set, but
#   no following bytes). This should be rejected by the proxy when full
#   MQTT inspection is enabled.
#
# Requirements:
# - curl
# - one of: nc, ncat, or socat (the script will try them in that order)
#
# Exit codes:
# 0 = success (metric increment observed)
# 2 = metric not present on the endpoint
# 3 = timeout waiting for metric
# 4 = unable to send malformed payload (no suitable tool)
#
set -uo pipefail

PROXY_HOST="${PROXY_HOST:-127.0.0.1}"
PROXY_PORT="${PROXY_PORT:-8080}"
METRICS_HOST="${METRICS_HOST:-127.0.0.1}"
METRICS_PORT="${METRICS_PORT:-9090}"

METRIC_NAME="aegis_protocol_rejections_total"
POLL_INTERVAL=1            # seconds between metric polls
POLL_TIMEOUT=10            # total seconds to wait for metric to increase

# Helper: fetch metric value (returns integer or empty string if not present)
fetch_metric_value() {
    local out
    if ! out="$(curl -sS "http://${METRICS_HOST}:${METRICS_PORT}/metrics" 2>/dev/null)"; then
        echo ""
        return
    fi
    # Use grep anchored to exact metric name; allow optional help/type lines around it.
    # Extract the numeric value from the metric line.
    local line
    line="$(echo "$out" | grep -E "^${METRIC_NAME}[[:space:]]+" || true)"
    if [ -z "$line" ]; then
        echo ""
        return
    fi
    # Get last token which should be the value (handles integers and floats)
    echo "$line" | awk '{print $NF}'
}

# Helper: send malformed MQTT CONNECT bytes to the proxy
send_malformed_connect() {
    # Malformed payload: fixed header 0x10 (CONNECT), remaining length continuation byte 0x80
    # (MSB=1 indicates continuation, but no further bytes are sent -> truncated remaining length)
    local payload
    payload=$'\x10\x80'

    # Try nc, then ncat, then socat
    if command -v nc >/dev/null 2>&1; then
        printf "%s" "$payload" | nc "${PROXY_HOST}" "${PROXY_PORT}" -w 1 || true
        return $?
    fi
    if command -v ncat >/dev/null 2>&1; then
        printf "%s" "$payload" | ncat "${PROXY_HOST}" "${PROXY_PORT}" --send-only --recv-only -w 1 || true
        return $?
    fi
    if command -v socat >/dev/null 2>&1; then
        # socat -u - TCP:host:port
        printf "%s" "$payload" | socat -u - "TCP:${PROXY_HOST}:${PROXY_PORT},connect-timeout=1" >/dev/null 2>&1 || true
        return $?
    fi

    return 4
}

main() {
    echo "AegisGate protocol rejection test"
    echo "Proxy: ${PROXY_HOST}:${PROXY_PORT}"
    echo "Metrics: http://${METRICS_HOST}:${METRICS_PORT}/metrics"
    echo

    echo "Fetching initial metric value for ${METRIC_NAME}..."
    initial="$(fetch_metric_value)"
    if [ -z "$initial" ]; then
        echo "ERROR: Metric ${METRIC_NAME} not found on metrics endpoint."
        echo "Dumping available metrics for debugging:"
        curl -sS "http://${METRICS_HOST}:${METRICS_PORT}/metrics" || true
        exit 2
    fi

    # Ensure we work with integers; metrics are counters but may be represented as floats.
    # We'll convert to an integer floor for comparison.
    initial_int="$(printf '%d\n' "${initial%%.*}" 2>/dev/null || printf '%d\n' "0")"
    echo "Initial ${METRIC_NAME} = ${initial} (interpreted as ${initial_int})"
    echo

    echo "Sending malformed MQTT CONNECT to ${PROXY_HOST}:${PROXY_PORT}..."
    if ! send_malformed_connect; then
        echo "ERROR: Failed to send malformed payload (no suitable netcat/socat found)."
        exit 4
    fi

    echo "Malformed payload sent. Waiting for metric to increment..."
    elapsed=0
    while [ $elapsed -lt $POLL_TIMEOUT ]; do
        sleep "$POLL_INTERVAL"
        elapsed=$((elapsed + POLL_INTERVAL))
        current="$(fetch_metric_value)"
        if [ -z "$current" ]; then
            echo "WARNING: metrics endpoint temporarily unavailable; continuing to poll..."
            continue
        fi
        current_int="$(printf '%d\n' "${current%%.*}" 2>/dev/null || printf '%d\n' "0")"
        if [ "$current_int" -gt "$initial_int" ]; then
            echo "SUCCESS: ${METRIC_NAME} incremented: ${initial} -> ${current}"
            exit 0
        fi
        echo "Still waiting... (${elapsed}s elapsed) current=${current}"
    done

    echo "FAIL: ${METRIC_NAME} did not increment within ${POLL_TIMEOUT}s."
    echo "Final metrics snapshot:"
    curl -sS "http://${METRICS_HOST}:${METRICS_PORT}/metrics" || true
    exit 3
}

main "$@"
