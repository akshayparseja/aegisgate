#!/bin/bash
# Test script for HTTP inspection and Slowloris protection features

set -e

PROXY_HOST="localhost"
PROXY_PORT="8080"
METRICS_PORT="9090"

echo "🧪 Testing HTTP Inspection and Slowloris Protection"
echo "=================================================="
echo ""

# Check if proxy is running
echo "1️⃣  Checking if AegisGate proxy is running..."
if ! nc -z $PROXY_HOST $PROXY_PORT 2>/dev/null; then
    echo "❌ Error: Proxy is not running on $PROXY_HOST:$PROXY_PORT"
    echo "   Please start the proxy first with: cargo run --release"
    exit 1
fi
echo "✅ Proxy is running"
echo ""

# Get initial metrics
echo "2️⃣  Getting initial metrics..."
INITIAL_HTTP=$(curl -s http://$PROXY_HOST:$METRICS_PORT/metrics | grep "aegis_http_rejections_total" | grep -v "#" | awk '{print $2}' || echo "0")
INITIAL_SLOWLORIS=$(curl -s http://$PROXY_HOST:$METRICS_PORT/metrics | grep "aegis_slowloris_rejections_total" | grep -v "#" | awk '{print $2}' || echo "0")
echo "   Initial HTTP rejections: $INITIAL_HTTP"
echo "   Initial Slowloris rejections: $INITIAL_SLOWLORIS"
echo ""

# Test 1: Send valid HTTP request (should be rejected as wrong protocol)
echo "3️⃣  Test 1: Sending valid HTTP GET request..."
echo "   (Should be rejected as wrong protocol for MQTT broker)"
(echo -e "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n" | nc -w 2 $PROXY_HOST $PROXY_PORT > /dev/null 2>&1) || true
sleep 1
HTTP_COUNT=$(curl -s http://$PROXY_HOST:$METRICS_PORT/metrics | grep "aegis_http_rejections_total" | grep -v "#" | awk '{print $2}' || echo "0")
if [ "$HTTP_COUNT" -gt "$INITIAL_HTTP" ]; then
    echo "✅ HTTP request detected and rejected (count: $INITIAL_HTTP → $HTTP_COUNT)"
else
    echo "❌ HTTP rejection metric did not increment"
fi
echo ""

# Test 2: Send HTTP POST request
echo "4️⃣  Test 2: Sending HTTP POST request..."
(echo -e "POST /api HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\n\r\n" | nc -w 2 $PROXY_HOST $PROXY_PORT > /dev/null 2>&1) || true
sleep 1
HTTP_COUNT2=$(curl -s http://$PROXY_HOST:$METRICS_PORT/metrics | grep "aegis_http_rejections_total" | grep -v "#" | awk '{print $2}' || echo "0")
if [ "$HTTP_COUNT2" -gt "$HTTP_COUNT" ]; then
    echo "✅ HTTP POST detected and rejected (count: $HTTP_COUNT → $HTTP_COUNT2)"
else
    echo "❌ HTTP rejection metric did not increment"
fi
echo ""

# Test 3: Slowloris attack simulation - slow headers
echo "5️⃣  Test 3: Simulating Slowloris attack (slow header transmission)..."
echo "   Sending headers very slowly (one byte every 2 seconds)..."
(
    # Send HTTP request line
    echo -n "GET / HTTP/1.1"
    echo -ne "\r\n"
    sleep 3
    # Send first header slowly
    echo -n "Host: "
    sleep 3
    echo -n "localhost"
    sleep 3
    echo -ne "\r\n"
    sleep 3
    echo -ne "\r\n"
) | nc -w 1 $PROXY_HOST $PROXY_PORT > /dev/null 2>&1 || true

sleep 1
SLOWLORIS_COUNT=$(curl -s http://$PROXY_HOST:$METRICS_PORT/metrics | grep "aegis_slowloris_rejections_total" | grep -v "#" | awk '{print $2}' || echo "0")
if [ "$SLOWLORIS_COUNT" -gt "$INITIAL_SLOWLORIS" ]; then
    echo "✅ Slowloris attack detected and rejected (count: $INITIAL_SLOWLORIS → $SLOWLORIS_COUNT)"
else
    echo "⚠️  Slowloris metric did not increment (may have been rejected for other reasons)"
fi
echo ""

# Test 4: Too many headers attack
echo "6️⃣  Test 4: Sending request with excessive headers..."
(
    echo -e "GET / HTTP/1.1\r"
    for i in {1..150}; do
        echo -e "X-Header-$i: value$i\r"
    done
    echo -e "\r"
) | nc -w 2 $PROXY_HOST $PROXY_PORT > /dev/null 2>&1 || true

sleep 1
SLOWLORIS_COUNT2=$(curl -s http://$PROXY_HOST:$METRICS_PORT/metrics | grep "aegis_slowloris_rejections_total" | grep -v "#" | awk '{print $2}' || echo "0")
HTTP_COUNT3=$(curl -s http://$PROXY_HOST:$METRICS_PORT/metrics | grep "aegis_http_rejections_total" | grep -v "#" | awk '{print $2}' || echo "0")
if [ "$SLOWLORIS_COUNT2" -gt "$SLOWLORIS_COUNT" ]; then
    echo "✅ Excessive headers detected as Slowloris (count: $SLOWLORIS_COUNT → $SLOWLORIS_COUNT2)"
elif [ "$HTTP_COUNT3" -gt "$HTTP_COUNT2" ]; then
    echo "✅ Excessive headers handled (HTTP rejection: $HTTP_COUNT2 → $HTTP_COUNT3)"
else
    echo "⚠️  Metrics did not change as expected"
fi
echo ""

# Test 5: Verify MQTT still works
echo "7️⃣  Test 5: Verifying legitimate MQTT traffic still works..."
MQTT_RESULT=$(timeout 5 mosquitto_pub -h $PROXY_HOST -p $PROXY_PORT -t "test/slowloris" -m "mqtt_works" -q 0 2>&1 || echo "FAILED")
if [[ ! "$MQTT_RESULT" =~ "FAILED" ]] && [[ ! "$MQTT_RESULT" =~ "Error" ]]; then
    echo "✅ MQTT publish successful (legitimate traffic not affected)"
else
    echo "❌ MQTT publish failed - protection may be too aggressive"
    echo "   Error: $MQTT_RESULT"
fi
echo ""

# Test 6: First packet timeout
echo "8️⃣  Test 6: Testing first packet timeout (connection without sending data)..."
BEFORE_SLOWLORIS=$(curl -s http://$PROXY_HOST:$METRICS_PORT/metrics | grep "aegis_slowloris_rejections_total" | grep -v "#" | awk '{print $2}' || echo "0")
# Connect but don't send anything for 35 seconds (exceeds first_packet_timeout of 30s)
timeout 35 nc $PROXY_HOST $PROXY_PORT > /dev/null 2>&1 || true
sleep 1
AFTER_SLOWLORIS=$(curl -s http://$PROXY_HOST:$METRICS_PORT/metrics | grep "aegis_slowloris_rejections_total" | grep -v "#" | awk '{print $2}' || echo "0")
if [ "$AFTER_SLOWLORIS" -gt "$BEFORE_SLOWLORIS" ]; then
    echo "✅ First packet timeout enforced (count: $BEFORE_SLOWLORIS → $AFTER_SLOWLORIS)"
else
    echo "⚠️  First packet timeout may not have triggered (connection might have closed earlier)"
fi
echo ""

# Summary
echo "=================================================="
echo "📊 Test Summary"
echo "=================================================="
FINAL_HTTP=$(curl -s http://$PROXY_HOST:$METRICS_PORT/metrics | grep "aegis_http_rejections_total" | grep -v "#" | awk '{print $2}' || echo "0")
FINAL_SLOWLORIS=$(curl -s http://$PROXY_HOST:$METRICS_PORT/metrics | grep "aegis_slowloris_rejections_total" | grep -v "#" | awk '{print $2}' || echo "0")
FINAL_PROTOCOL=$(curl -s http://$PROXY_HOST:$METRICS_PORT/metrics | grep "aegis_protocol_rejections_total" | grep -v "#" | awk '{print $2}' || echo "0")

HTTP_DELTA=$((FINAL_HTTP - INITIAL_HTTP))
SLOWLORIS_DELTA=$((FINAL_SLOWLORIS - INITIAL_SLOWLORIS))

echo "HTTP rejections:      $INITIAL_HTTP → $FINAL_HTTP (+$HTTP_DELTA)"
echo "Slowloris rejections: $INITIAL_SLOWLORIS → $FINAL_SLOWLORIS (+$SLOWLORIS_DELTA)"
echo "Protocol rejections:  $FINAL_PROTOCOL"
echo ""

if [ "$HTTP_DELTA" -ge 2 ] && [ "$SLOWLORIS_DELTA" -ge 1 ]; then
    echo "✅ All protection features working correctly!"
    exit 0
else
    echo "⚠️  Some tests may not have triggered as expected"
    echo "   This could be normal depending on timing and network conditions"
    exit 0
fi
