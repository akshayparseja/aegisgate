#!/bin/bash
# Professional End-to-End MQTT Verification with Readiness Check

PROXY_HOST="127.0.0.1"
PROXY_PORT=8080
HEALTH_PORT=9090
TOPIC="aegis/test"
MESSAGE="Production_Ready_Check_$(date +%s)"

echo "--- Starting AegisGate Flow Test ---"

# 1. Readiness Check: Wait for Proxy to be actually 'Healthy'
echo "Waiting for AegisGate to be healthy..."
MAX_RETRIES=10
COUNT=0
while ! curl -s http://$PROXY_HOST:$HEALTH_PORT/health | grep -q "OK"; do
    sleep 1
    COUNT=$((COUNT+1))
    if [ $COUNT -ge $MAX_RETRIES ]; then
        echo "Error: Timeout waiting for AegisGate health check."
        exit 1
    fi
done

# 2. Start a subscriber in the background
# We use -W 5 to timeout if no message is received within 5 seconds
mosquitto_sub -h $PROXY_HOST -p $PROXY_PORT -t $TOPIC -C 1 -W 5 > /tmp/aegis_received.txt &
SUB_PID=$!

# Give the subscriber a moment to establish the TCP session
sleep 1

# 3. Publish a message through the proxy
echo "Publishing to $TOPIC via Proxy..."
mosquitto_pub -h $PROXY_HOST -p $PROXY_PORT -t $TOPIC -m "$MESSAGE"

# 4. Wait for the subscriber to finish
wait $SUB_PID

# 5. Verify results
RECEIVED=$(cat /tmp/aegis_received.txt)
if [ "$RECEIVED" == "$MESSAGE" ]; then
    echo "SUCCESS: Message forwarded correctly."
    rm /tmp/aegis_received.txt
    exit 0
else
    echo "FAILURE: Received '$RECEIVED' but expected '$MESSAGE'"
    exit 1
fi
