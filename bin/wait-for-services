#!/bin/bash
# Wait for Headway services to become healthy

set -e

FRONTEND_URL="${FRONTEND_URL:-http://localhost:8080}"
MAX_WAIT="${MAX_WAIT:-300}"  # 5 minutes
SLEEP_INTERVAL=5

echo "Waiting for Headway services to become ready..."
echo "Frontend URL: $FRONTEND_URL"
echo "Max wait time: ${MAX_WAIT}s"
echo ""

elapsed=0

# Function to check if a service is responding
check_service() {
    local name=$1
    local url=$2
    local expected_status=${3:-200}

    if curl -s -f -o /dev/null -w "%{http_code}" "$url" | grep -q "$expected_status"; then
        return 0
    else
        return 1
    fi
}

# Wait for frontend to be up
echo "Checking frontend..."
while [ $elapsed -lt $MAX_WAIT ]; do
    if curl -s -f -o /dev/null "$FRONTEND_URL" 2>/dev/null; then
        echo "✓ Frontend is responding"
        break
    fi
    echo "  Waiting for frontend... (${elapsed}s elapsed)"
    sleep $SLEEP_INTERVAL
    elapsed=$((elapsed + SLEEP_INTERVAL))
done

if [ $elapsed -ge $MAX_WAIT ]; then
    echo "✗ Timeout waiting for frontend"
    exit 1
fi

# Wait for travelmux health endpoint
echo "Checking travelmux health..."
elapsed=0
while [ $elapsed -lt $MAX_WAIT ]; do
    if check_service "travelmux" "$FRONTEND_URL/api/health" 200; then
        echo "✓ Travelmux is healthy"
        break
    fi
    echo "  Waiting for travelmux... (${elapsed}s elapsed)"
    sleep $SLEEP_INTERVAL
    elapsed=$((elapsed + SLEEP_INTERVAL))
done

if [ $elapsed -ge $MAX_WAIT ]; then
    echo "✗ Timeout waiting for travelmux"
    exit 1
fi

# Wait for pelias to respond
echo "Checking pelias..."
elapsed=0
while [ $elapsed -lt $MAX_WAIT ]; do
    if curl -s "$FRONTEND_URL/pelias/v1/autocomplete?text=test" | grep -q "features" 2>/dev/null; then
        echo "✓ Pelias is responding"
        break
    fi
    echo "  Waiting for pelias... (${elapsed}s elapsed)"
    sleep $SLEEP_INTERVAL
    elapsed=$((elapsed + SLEEP_INTERVAL))
done

if [ $elapsed -ge $MAX_WAIT ]; then
    echo "✗ Timeout waiting for pelias"
    exit 1
fi

# Wait for tileserver
echo "Checking tileserver..."
# Use a sample tile request - we just need to check if tileserver responds
# This will be tested more thoroughly in the actual tests
elapsed=0
while [ $elapsed -lt $MAX_WAIT ]; do
    if curl -s -f -o /dev/null "$FRONTEND_URL/tileserver/" 2>/dev/null || \
       curl -s -f -o /dev/null -w "%{http_code}" "$FRONTEND_URL/tileserver/health" 2>/dev/null | grep -q "200"; then
        echo "✓ Tileserver is responding"
        break
    fi
    echo "  Waiting for tileserver... (${elapsed}s elapsed)"
    sleep $SLEEP_INTERVAL
    elapsed=$((elapsed + SLEEP_INTERVAL))
done

if [ $elapsed -ge $MAX_WAIT ]; then
    echo "✗ Timeout waiting for tileserver"
    exit 1
fi

echo ""
echo "All services are ready!"
exit 0
