#!/bin/bash
# One-shot integration test runner for Headway
# Starts services, runs tests, and cleans up

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

cd "$PROJECT_ROOT"

FRONTEND_URL="${FRONTEND_URL:-http://localhost:8080}"

echo "========================================"
echo "Headway Integration Test Runner"
echo "========================================"
echo ""

# Function to cleanup on exit
cleanup() {
    echo ""
    "$SCRIPT_DIR/stop-services.sh"
}

# Register cleanup function
trap cleanup EXIT INT TERM

# Start services
"$SCRIPT_DIR/start-services.sh"

echo ""
echo "Waiting for services to be ready..."
export FRONTEND_URL
"$SCRIPT_DIR/wait-for-services.sh"

echo ""
# Run tests
"$SCRIPT_DIR/run-tests.sh"
