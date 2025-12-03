#!/bin/bash
# One-shot integration test runner for Headway
# Starts services, runs tests, and cleans up

set -e

CONFIG_DIR="$1"
if [ -z "$CONFIG_DIR" ]; then
    cat <<EOS
Usage: $0 <config-dir>
Example: $0 builds/Bogota
EOS
    exit 1
fi

APP_ROOT=$(git rev-parse --show-toplevel)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$APP_ROOT"

FRONTEND_URL="${FRONTEND_URL:-http://localhost:8080}"

echo "========================================"
echo "Headway Integration Test Runner"
echo "========================================"
echo "Build dir: $CONFIG_DIR"
echo ""

cleanup() {
    echo ""
    "$APP_ROOT/bin/stop-services" "$CONFIG_DIR"
}
trap cleanup EXIT INT TERM

"$APP_ROOT/bin/start-services" "$CONFIG_DIR"

echo ""
echo "Waiting for services to be ready..."
export FRONTEND_URL
"$APP_ROOT/bin/wait-for-services"

echo ""
"$SCRIPT_DIR/run-tests.sh"
