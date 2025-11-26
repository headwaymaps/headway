#!/bin/bash
# Test tileserver endpoints via frontend proxy

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/_test-lib.sh"

FRONTEND_URL="${FRONTEND_URL:-http://localhost:8080}"

# Bogota tile coordinates (verified from maps.earth)
# Zoom 13, tile covering central Bogota
ZOOM=13
TILE_X=2409
TILE_Y=3990

echo "Testing Tileserver..."
echo ""

# Test a single vector tile
run_binary_test "vector tile" \
    "$FRONTEND_URL/tileserver/data/default/$ZOOM/$TILE_X/$TILE_Y.pbf" \
    "application/x-protobuf" \
    100

print_test_summary "Tileserver"
