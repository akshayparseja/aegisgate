#!/bin/bash

# AegisGate Quick Benchmark Runner
# This script cleans up any old processes, restarts EMQX, and runs benchmarks

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BENCHMARK_DIR="$(dirname "$SCRIPT_DIR")"
PROJECT_DIR="$(dirname "$BENCHMARK_DIR")"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}======================================================================${NC}"
echo -e "${BLUE}AegisGate Benchmark Runner${NC}"
echo -e "${BLUE}======================================================================${NC}\n"

# Step 1: Cleanup any zombie processes
echo -e "${YELLOW}Step 1: Cleaning up zombie processes...${NC}"
pkill -9 -f "benchmark.*\.py" 2>/dev/null || true
pkill -9 -f "mosquitto_pub" 2>/dev/null || true
pkill -9 -f "quick_test" 2>/dev/null || true
echo -e "${GREEN}✓ Cleanup complete${NC}\n"

# Step 2: Stop and remove old EMQX container
echo -e "${YELLOW}Step 2: Resetting EMQX...${NC}"
cd "$PROJECT_DIR"
docker stop aegisgate-mqtt-broker-1 2>/dev/null || true
docker rm aegisgate-mqtt-broker-1 2>/dev/null || true
echo -e "${GREEN}✓ Old container removed${NC}\n"

# Step 3: Start fresh EMQX
echo -e "${YELLOW}Step 3: Starting fresh EMQX instance...${NC}"
docker-compose --profile debug-broker up -d mqtt-broker
echo "Waiting for EMQX to fully start (8 seconds)..."
sleep 8
echo -e "${GREEN}✓ EMQX is ready${NC}\n"

# Step 4: Verify EMQX is clean
echo -e "${YELLOW}Step 4: Verifying clean state...${NC}"
TOKEN=$(curl -s -X POST "http://localhost:18083/api/v5/login" \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"public"}' | python3 -c "import sys, json; print(json.load(sys.stdin)['token'])")

METRICS=$(curl -s "http://localhost:18083/api/v5/metrics" -H "Authorization: Bearer $TOKEN")
MSG_COUNT=$(echo "$METRICS" | python3 -c "import sys, json; print(json.load(sys.stdin)[0]['messages.publish'])")
CONN_COUNT=$(echo "$METRICS" | python3 -c "import sys, json; print(json.load(sys.stdin)[0]['client.connected'])")

echo "  Messages published: $MSG_COUNT"
echo "  Connections: $CONN_COUNT"

if [ "$MSG_COUNT" -eq 0 ] && [ "$CONN_COUNT" -eq 0 ]; then
    echo -e "${GREEN}✓ EMQX is clean${NC}\n"
else
    echo -e "${RED}⚠ Warning: EMQX has non-zero counts${NC}\n"
fi

# Step 5: Check AegisGate
echo -e "${YELLOW}Step 5: Checking AegisGate...${NC}"
if curl -s http://localhost:9090/metrics > /dev/null 2>&1; then
    echo -e "${GREEN}✓ AegisGate is running${NC}\n"
else
    echo -e "${RED}✗ AegisGate is not responding on port 9090${NC}"
    echo -e "${YELLOW}Starting AegisGate...${NC}"
    docker-compose up -d aegis-proxy
    sleep 3
    echo -e "${GREEN}✓ AegisGate started${NC}\n"
fi

# Step 6: Run benchmark
echo -e "${YELLOW}Step 6: Running benchmark...${NC}"
echo -e "${BLUE}======================================================================${NC}\n"

if [ "$1" == "--rigorous" ]; then
    echo "Running RIGOROUS high-volume benchmark (this may take 4-6 minutes)..."
    python3 "$SCRIPT_DIR/benchmark_rigorous.py"
elif [ "$1" == "--stress" ]; then
    echo "Running STRESS TEST benchmark (3-4 minutes)..."
    python3 "$SCRIPT_DIR/benchmark_stress.py"
else
    echo "Running QUICK benchmark (recommended, ~60 seconds)..."
    python3 "$SCRIPT_DIR/benchmark_quick.py"
fi

BENCHMARK_EXIT=$?

echo ""
echo -e "${BLUE}======================================================================${NC}"

# Step 7: Post-benchmark verification
echo -e "${YELLOW}Step 7: Post-benchmark verification...${NC}"

# Wait a moment for cleanup
sleep 2

# Check for zombie processes
ZOMBIE_COUNT=$(ps aux | grep -E "(benchmark.*\.py|mosquitto_pub)" | grep -v grep | wc -l)
if [ "$ZOMBIE_COUNT" -gt 0 ]; then
    echo -e "${YELLOW}⚠ Warning: Found $ZOMBIE_COUNT lingering benchmark processes${NC}"
    echo "Cleaning them up..."
    pkill -9 -f "benchmark.*\.py" 2>/dev/null || true
    pkill -9 -f "mosquitto_pub" 2>/dev/null || true
else
    echo -e "${GREEN}✓ No zombie processes${NC}"
fi

# Check if publishing is still happening
METRICS_BEFORE=$(curl -s "http://localhost:18083/api/v5/metrics" -H "Authorization: Bearer $TOKEN")
MSG_BEFORE=$(echo "$METRICS_BEFORE" | python3 -c "import sys, json; print(json.load(sys.stdin)[0]['messages.publish'])")
sleep 3
METRICS_AFTER=$(curl -s "http://localhost:18083/api/v5/metrics" -H "Authorization: Bearer $TOKEN")
MSG_AFTER=$(echo "$METRICS_AFTER" | python3 -c "import sys, json; print(json.load(sys.stdin)[0]['messages.publish'])")

MSG_DIFF=$((MSG_AFTER - MSG_BEFORE))
if [ "$MSG_DIFF" -gt 0 ]; then
    echo -e "${RED}✗ Publishing is still active! ($MSG_DIFF messages in 3 seconds)${NC}"
    echo "Run: ps aux | grep -E '(benchmark|mosquitto)' to investigate"
else
    echo -e "${GREEN}✓ No active publishing${NC}"
fi

# Check active connections
CONN_ACTIVE=$(curl -s "http://localhost:18083/api/v5/stats" -H "Authorization: Bearer $TOKEN" | python3 -c "import sys, json; print(json.load(sys.stdin)[0]['connections.count'])")
echo "  Active connections: $CONN_ACTIVE"

if [ "$CONN_ACTIVE" -eq 0 ]; then
    echo -e "${GREEN}✓ All connections closed${NC}"
else
    echo -e "${YELLOW}⚠ Warning: $CONN_ACTIVE connections still active${NC}"
fi

echo ""

# Final summary
if [ $BENCHMARK_EXIT -eq 0 ]; then
    echo -e "${GREEN}======================================================================${NC}"
    echo -e "${GREEN}✓ Benchmark completed successfully!${NC}"
    echo -e "${GREEN}======================================================================${NC}"
else
    echo -e "${RED}======================================================================${NC}"
    echo -e "${RED}✗ Benchmark failed with exit code $BENCHMARK_EXIT${NC}"
    echo -e "${RED}======================================================================${NC}"
fi

echo ""
echo -e "${BLUE}Usage:${NC}"
echo "  $0              - Run quick benchmark (default, ~60 seconds)"
echo "  $0 --rigorous   - Run rigorous high-volume benchmark (~4-6 minutes)"
echo "  $0 --stress     - Run stress test benchmark (~3-4 minutes)"
echo ""

exit $BENCHMARK_EXIT
