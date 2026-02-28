#!/bin/bash
# Comprehensive Slowloris Attack Testing Script
# Tests AegisGate's Slowloris protection using popular attack tools

set -e

PROXY_HOST="localhost"
PROXY_PORT="8080"
METRICS_PORT="9090"
ATTACK_DURATION=30
CONNECTIONS=100

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║  AegisGate Slowloris Protection Testing Suite             ║${NC}"
echo -e "${BLUE}║  Testing with Real Attack Tools                           ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check if proxy is running
echo -e "${YELLOW}🔍 Checking prerequisites...${NC}"
if ! nc -z $PROXY_HOST $PROXY_PORT 2>/dev/null; then
    echo -e "${RED}❌ Error: Proxy is not running on $PROXY_HOST:$PROXY_PORT${NC}"
    echo "   Please start the proxy first with: cargo run --release"
    exit 1
fi
echo -e "${GREEN}✅ Proxy is running${NC}"

# Check for attack tools
TOOLS_AVAILABLE=0

echo ""
echo -e "${YELLOW}📦 Checking for Slowloris testing tools...${NC}"

# Check for slowloris.py
if command -v slowloris &> /dev/null || [ -f "slowloris.py" ] || [ -f "../slowloris/slowloris.py" ]; then
    echo -e "${GREEN}✅ slowloris.py found${NC}"
    SLOWLORIS_PY_AVAILABLE=1
    TOOLS_AVAILABLE=$((TOOLS_AVAILABLE + 1))
else
    echo -e "${YELLOW}⚠️  slowloris.py not found${NC}"
    echo "   Install: git clone https://github.com/gkbrk/slowloris.git"
    SLOWLORIS_PY_AVAILABLE=0
fi

# Check for SlowHTTPTest
if command -v slowhttptest &> /dev/null; then
    echo -e "${GREEN}✅ slowhttptest found${NC}"
    SLOWHTTPTEST_AVAILABLE=1
    TOOLS_AVAILABLE=$((TOOLS_AVAILABLE + 1))
else
    echo -e "${YELLOW}⚠️  slowhttptest not found${NC}"
    echo "   Install: sudo apt-get install slowhttptest"
    SLOWHTTPTEST_AVAILABLE=0
fi

# Check for hping3 (for TCP-level attacks)
if command -v hping3 &> /dev/null; then
    echo -e "${GREEN}✅ hping3 found${NC}"
    HPING3_AVAILABLE=1
    TOOLS_AVAILABLE=$((TOOLS_AVAILABLE + 1))
else
    echo -e "${YELLOW}⚠️  hping3 not found (optional)${NC}"
    HPING3_AVAILABLE=0
fi

if [ $TOOLS_AVAILABLE -eq 0 ]; then
    echo ""
    echo -e "${RED}❌ No attack tools found. Installing at least one is recommended.${NC}"
    echo ""
    echo "Quick install options:"
    echo "  1. slowloris.py:   git clone https://github.com/gkbrk/slowloris.git"
    echo "  2. slowhttptest:   sudo apt-get install slowhttptest"
    echo ""
    echo "Continuing with manual tests only..."
    MANUAL_ONLY=1
else
    echo -e "${GREEN}✅ Found $TOOLS_AVAILABLE attack tool(s)${NC}"
    MANUAL_ONLY=0
fi

# Function to get metric value
get_metric() {
    local metric_name=$1
    curl -s http://$PROXY_HOST:$METRICS_PORT/metrics | grep "^$metric_name " | awk '{print $2}' || echo "0"
}

# Function to print metric delta
print_metric_delta() {
    local name=$1
    local before=$2
    local after=$3
    local delta=$((after - before))

    if [ $delta -gt 0 ]; then
        echo -e "${GREEN}   $name: $before → $after (+$delta)${NC}"
    else
        echo -e "${YELLOW}   $name: $before → $after (no change)${NC}"
    fi
}

echo ""
echo -e "${BLUE}════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  Starting Attack Tests${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════════${NC}"
echo ""

# Get initial metrics
INITIAL_HTTP=$(get_metric "aegis_http_rejections_total")
INITIAL_SLOWLORIS=$(get_metric "aegis_slowloris_rejections_total")
INITIAL_PROTOCOL=$(get_metric "aegis_protocol_rejections_total")
INITIAL_REJECTED=$(get_metric "aegis_rejected_connections_total")

echo -e "${YELLOW}📊 Initial Metrics:${NC}"
echo "   HTTP rejections: $INITIAL_HTTP"
echo "   Slowloris rejections: $INITIAL_SLOWLORIS"
echo "   Protocol rejections: $INITIAL_PROTOCOL"
echo "   Rate limit rejections: $INITIAL_REJECTED"
echo ""

TEST_NUMBER=1

# Test 1: Manual Slowloris Simulation
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}Test ${TEST_NUMBER}: Manual Slowloris Simulation${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo "Sending HTTP headers very slowly (should trigger idle timeout)..."

for i in {1..5}; do
    (
        echo -n "GET / HTTP/1.1"
        echo -ne "\r\n"
        sleep 2
        echo -n "Host: localhost"
        sleep 12  # Exceeds 10s idle timeout
        echo -ne "\r\n\r\n"
    ) | nc -w 1 $PROXY_HOST $PROXY_PORT > /dev/null 2>&1 &
done
wait

sleep 2
AFTER_SLOWLORIS=$(get_metric "aegis_slowloris_rejections_total")
AFTER_HTTP=$(get_metric "aegis_http_rejections_total")
print_metric_delta "Slowloris rejections" $INITIAL_SLOWLORIS $AFTER_SLOWLORIS
print_metric_delta "HTTP rejections" $INITIAL_HTTP $AFTER_HTTP
echo ""
TEST_NUMBER=$((TEST_NUMBER + 1))
INITIAL_SLOWLORIS=$AFTER_SLOWLORIS
INITIAL_HTTP=$AFTER_HTTP

# Test 2: slowloris.py
if [ $SLOWLORIS_PY_AVAILABLE -eq 1 ]; then
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${YELLOW}Test ${TEST_NUMBER}: slowloris.py Attack${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo "Running original Slowloris attack tool..."
    echo "Duration: 20 seconds, Connections: $CONNECTIONS"

    # Find slowloris.py
    SLOWLORIS_CMD=""
    if [ -f "slowloris.py" ]; then
        SLOWLORIS_CMD="python3 slowloris.py"
    elif [ -f "../slowloris/slowloris.py" ]; then
        SLOWLORIS_CMD="python3 ../slowloris/slowloris.py"
    elif command -v slowloris &> /dev/null; then
        SLOWLORIS_CMD="slowloris"
    fi

    if [ -n "$SLOWLORIS_CMD" ]; then
        timeout 20 $SLOWLORIS_CMD $PROXY_HOST -p $PROXY_PORT -s $CONNECTIONS --sleeptime 2 > /dev/null 2>&1 || true

        sleep 2
        AFTER_SLOWLORIS=$(get_metric "aegis_slowloris_rejections_total")
        AFTER_HTTP=$(get_metric "aegis_http_rejections_total")
        print_metric_delta "Slowloris rejections" $INITIAL_SLOWLORIS $AFTER_SLOWLORIS
        print_metric_delta "HTTP rejections" $INITIAL_HTTP $AFTER_HTTP

        DELTA=$((AFTER_SLOWLORIS - INITIAL_SLOWLORIS + AFTER_HTTP - INITIAL_HTTP))
        if [ $DELTA -gt 50 ]; then
            echo -e "${GREEN}✅ Attack successfully detected and blocked!${NC}"
        elif [ $DELTA -gt 10 ]; then
            echo -e "${YELLOW}⚠️  Partial blocking detected${NC}"
        else
            echo -e "${RED}❌ Attack may not have been properly blocked${NC}"
        fi
    fi
    echo ""
    TEST_NUMBER=$((TEST_NUMBER + 1))
    INITIAL_SLOWLORIS=$AFTER_SLOWLORIS
    INITIAL_HTTP=$AFTER_HTTP
fi

# Test 3: SlowHTTPTest
if [ $SLOWHTTPTEST_AVAILABLE -eq 1 ]; then
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${YELLOW}Test ${TEST_NUMBER}: SlowHTTPTest (Slow Headers)${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo "Running professional Slowloris testing tool..."
    echo "Attack mode: Slow Headers (-H)"

    timeout 20 slowhttptest -c 100 -H -i 15 -r 50 -t GET -u http://$PROXY_HOST:$PROXY_PORT -x 200 -p 3 > /tmp/slowhttp_test.log 2>&1 || true

    sleep 2
    AFTER_SLOWLORIS=$(get_metric "aegis_slowloris_rejections_total")
    AFTER_HTTP=$(get_metric "aegis_http_rejections_total")
    print_metric_delta "Slowloris rejections" $INITIAL_SLOWLORIS $AFTER_SLOWLORIS
    print_metric_delta "HTTP rejections" $INITIAL_HTTP $AFTER_HTTP

    # Show test results if available
    if [ -f "/tmp/slowhttp_test.log" ]; then
        CLOSED=$(grep -o "closed:[0-9]*" /tmp/slowhttp_test.log | tail -1 | cut -d: -f2 || echo "0")
        if [ "$CLOSED" != "0" ]; then
            echo -e "${GREEN}   Connections closed by server: $CLOSED${NC}"
        fi
    fi

    echo ""
    TEST_NUMBER=$((TEST_NUMBER + 1))
    INITIAL_SLOWLORIS=$AFTER_SLOWLORIS
    INITIAL_HTTP=$AFTER_HTTP

    # Test 3b: Slow POST
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${YELLOW}Test ${TEST_NUMBER}: SlowHTTPTest (Slow POST Body)${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo "Attack mode: Slow POST body (-B)"

    timeout 20 slowhttptest -c 100 -B -i 15 -r 50 -t POST -u http://$PROXY_HOST:$PROXY_PORT -x 200 -p 3 > /tmp/slowpost_test.log 2>&1 || true

    sleep 2
    AFTER_SLOWLORIS=$(get_metric "aegis_slowloris_rejections_total")
    AFTER_HTTP=$(get_metric "aegis_http_rejections_total")
    print_metric_delta "Slowloris rejections" $INITIAL_SLOWLORIS $AFTER_SLOWLORIS
    print_metric_delta "HTTP rejections" $INITIAL_HTTP $AFTER_HTTP
    echo ""
    TEST_NUMBER=$((TEST_NUMBER + 1))
    INITIAL_SLOWLORIS=$AFTER_SLOWLORIS
    INITIAL_HTTP=$AFTER_HTTP
fi

# Test 4: Connection flood (rapid connects without data)
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}Test ${TEST_NUMBER}: Connection Flood (First Packet Timeout)${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo "Opening connections without sending data..."

for i in {1..20}; do
    timeout 35 nc $PROXY_HOST $PROXY_PORT > /dev/null 2>&1 &
done

echo "Waiting for first_packet_timeout (30s)..."
sleep 32

AFTER_SLOWLORIS=$(get_metric "aegis_slowloris_rejections_total")
print_metric_delta "Slowloris rejections" $INITIAL_SLOWLORIS $AFTER_SLOWLORIS

# Clean up background processes
jobs -p | xargs -r kill 2>/dev/null || true
echo ""
TEST_NUMBER=$((TEST_NUMBER + 1))
INITIAL_SLOWLORIS=$AFTER_SLOWLORIS

# Test 5: Excessive headers attack
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}Test ${TEST_NUMBER}: Header Bomb (Excessive Headers)${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo "Sending 150+ headers (exceeds max_http_header_count)..."

for i in {1..5}; do
    (
        echo -e "GET / HTTP/1.1\r"
        for j in {1..151}; do
            echo -e "X-Custom-Header-$j: value$j\r"
        done
        echo -e "\r"
    ) | nc -w 2 $PROXY_HOST $PROXY_PORT > /dev/null 2>&1 &
done
wait

sleep 2
AFTER_SLOWLORIS=$(get_metric "aegis_slowloris_rejections_total")
AFTER_HTTP=$(get_metric "aegis_http_rejections_total")
print_metric_delta "Slowloris rejections" $INITIAL_SLOWLORIS $AFTER_SLOWLORIS
print_metric_delta "HTTP rejections" $INITIAL_HTTP $AFTER_HTTP
echo ""
TEST_NUMBER=$((TEST_NUMBER + 1))
INITIAL_SLOWLORIS=$AFTER_SLOWLORIS
INITIAL_HTTP=$AFTER_HTTP

# Test 6: Verify MQTT still works
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}Test ${TEST_NUMBER}: MQTT Functionality (Protection Bypass)${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo "Verifying legitimate MQTT traffic is not blocked..."

if command -v mosquitto_pub &> /dev/null; then
    MQTT_SUCCESS=0
    for i in {1..3}; do
        if timeout 5 mosquitto_pub -h $PROXY_HOST -p $PROXY_PORT -t "test/protection" -m "test_message_$i" -q 0 2>&1 | grep -v "Error"; then
            MQTT_SUCCESS=$((MQTT_SUCCESS + 1))
        fi
        sleep 1
    done

    if [ $MQTT_SUCCESS -eq 3 ]; then
        echo -e "${GREEN}✅ All MQTT publishes successful ($MQTT_SUCCESS/3)${NC}"
        echo -e "${GREEN}   Protection does not affect legitimate traffic!${NC}"
    elif [ $MQTT_SUCCESS -gt 0 ]; then
        echo -e "${YELLOW}⚠️  Partial success ($MQTT_SUCCESS/3 MQTT publishes)${NC}"
    else
        echo -e "${RED}❌ MQTT traffic blocked - protection may be too aggressive${NC}"
    fi
else
    echo -e "${YELLOW}⚠️  mosquitto_pub not installed, skipping MQTT test${NC}"
    echo "   Install: sudo apt-get install mosquitto-clients"
fi
echo ""

# Final Summary
echo -e "${BLUE}════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  Final Results${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════════${NC}"
echo ""

FINAL_HTTP=$(get_metric "aegis_http_rejections_total")
FINAL_SLOWLORIS=$(get_metric "aegis_slowloris_rejections_total")
FINAL_PROTOCOL=$(get_metric "aegis_protocol_rejections_total")
FINAL_REJECTED=$(get_metric "aegis_rejected_connections_total")

TOTAL_HTTP_BLOCKED=$((FINAL_HTTP - INITIAL_HTTP))
TOTAL_SLOWLORIS_BLOCKED=$((FINAL_SLOWLORIS - INITIAL_SLOWLORIS))
TOTAL_BLOCKED=$((TOTAL_HTTP_BLOCKED + TOTAL_SLOWLORIS_BLOCKED))

echo -e "${YELLOW}📊 Metrics Summary:${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
printf "%-30s %10s → %-10s (+%d)\n" "HTTP rejections:" "$INITIAL_HTTP" "$FINAL_HTTP" "$TOTAL_HTTP_BLOCKED"
printf "%-30s %10s → %-10s (+%d)\n" "Slowloris rejections:" "$INITIAL_SLOWLORIS" "$FINAL_SLOWLORIS" "$TOTAL_SLOWLORIS_BLOCKED"
printf "%-30s %10s\n" "Protocol rejections:" "$FINAL_PROTOCOL"
printf "%-30s %10s\n" "Rate limit rejections:" "$FINAL_REJECTED"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
printf "${GREEN}%-30s %10d${NC}\n" "Total Attacks Blocked:" "$TOTAL_BLOCKED"
echo ""

# Verdict
if [ $TOTAL_BLOCKED -gt 50 ]; then
    echo -e "${GREEN}╔════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║  ✅ EXCELLENT: Protection is working effectively!         ║${NC}"
    echo -e "${GREEN}║  Blocked $TOTAL_BLOCKED+ attack connections                           ║${NC}"
    echo -e "${GREEN}╚════════════════════════════════════════════════════════════╝${NC}"
    EXIT_CODE=0
elif [ $TOTAL_BLOCKED -gt 10 ]; then
    echo -e "${YELLOW}╔════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${YELLOW}║  ⚠️  GOOD: Protection is working but may need tuning      ║${NC}"
    echo -e "${YELLOW}║  Blocked $TOTAL_BLOCKED attack connections                            ║${NC}"
    echo -e "${YELLOW}╚════════════════════════════════════════════════════════════╝${NC}"
    EXIT_CODE=0
else
    echo -e "${RED}╔════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${RED}║  ❌ WARNING: Low blocking rate detected                    ║${NC}"
    echo -e "${RED}║  Only blocked $TOTAL_BLOCKED connections                               ║${NC}"
    echo -e "${RED}║  Check if features are enabled in config                  ║${NC}"
    echo -e "${RED}╚════════════════════════════════════════════════════════════╝${NC}"
    EXIT_CODE=1
fi

echo ""
echo -e "${BLUE}💡 Tips:${NC}"
echo "  • Install tools: slowloris.py, slowhttptest for more comprehensive testing"
echo "  • View live metrics: curl http://localhost:$METRICS_PORT/metrics"
echo "  • Adjust timeouts in: config/aegis_config.yaml"
echo "  • Check logs for detailed rejection reasons"
echo ""

exit $EXIT_CODE
