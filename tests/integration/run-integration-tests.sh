#!/bin/bash
# One-shot integration test runner for Headway
# Starts services, runs tests, and cleans up

set -e

# The tests assert against Bogota - its coordinates, its transit - so the build they run
# against isn't interchangeable.
CONFIG_DIR="builds/Bogota"

APP_ROOT=$(git rev-parse --show-toplevel)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$APP_ROOT"

source bin/_source-env.sh "$CONFIG_DIR"

FRONTEND_URL="${FRONTEND_URL:-http://localhost:8080}"

if [ "${HEADWAY_ENABLE_TRANSIT_ROUTING:-0}" != 0 ]; then
    # The build writes dated transit artifacts - link the latest ones to the names
    # docker compose serves them from.
    TRANSIT_DATA_ROOT=./data "$APP_ROOT/bin/link-latest-transit" "$CONFIG_DIR"
fi

echo "========================================"
echo "Headway Integration Test Runner"
echo "========================================"
echo "Build dir: $CONFIG_DIR"
echo ""

cleanup() {
    echo ""
    "$APP_ROOT/bin/stop-and-remove-services" "$CONFIG_DIR"
}
trap cleanup EXIT INT TERM

"$APP_ROOT/bin/start-services" --no-follow-logs "$CONFIG_DIR"

echo ""
echo "Waiting for services to be ready..."
export FRONTEND_URL
"$APP_ROOT/bin/wait-for-services"

echo ""
"$SCRIPT_DIR/run-tests.sh"
