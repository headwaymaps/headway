#!/bin/bash
# Test tileserver endpoints via frontend proxy

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/_test-lib.sh"

echo "Testing Tileserver..."
echo ""

# Martin serves the areamap tiles under two sources (see martin-config.yaml):
#   areamap-mvt -> classic MVT protobuf
#   areamap-mlt -> MapLibre Tile (MLT), what the web app actually renders
# (BaseMap.vue requests these with `Accept: application/vnd.maplibre-tile`).

# MVT protobuf tile
run_binary_test "vector tile (MVT)" \
    "$HEADWAY_PUBLIC_URL/tileserver/areamap-mvt/$TILE" \
    "application/x-protobuf" \
    100

# MLT tile, negotiated exactly as the web app requests it
run_binary_test "vector tile (MLT)" \
    "$HEADWAY_PUBLIC_URL/tileserver/areamap-mlt/$TILE" \
    "application/vnd.maplibre-tile" \
    100 \
    "application/vnd.maplibre-tile"

print_test_summary "Tileserver"
