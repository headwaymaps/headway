#!/bin/bash
# Test service health endpoints via frontend proxy

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/_test-lib.sh"

echo "Testing Health..."
echo ""

run_jq_test "travelmux alive" "$HEADWAY_PUBLIC_URL/travelmux/health/alive"
run_jq_test "travelmux ready" "$HEADWAY_PUBLIC_URL/travelmux/health/ready"
run_jq_test "valhalla status" "$HEADWAY_PUBLIC_URL/valhalla/status"
run_jq_test "pelias api" "$HEADWAY_PUBLIC_URL/pelias/v1/"
run_jq_test "tileserver health" "$HEADWAY_PUBLIC_URL/tileserver/health"

print_test_summary "Health"
