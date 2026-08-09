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

# Martin serves the areamap tiles under two sources (see martin-config.yaml):
#   areamap-mvt -> classic MVT protobuf
#   areamap-mlt -> MapLibre Tile (MLT), what the web app actually renders
# (BaseMap.vue requests these with `Accept: application/vnd.maplibre-tile`).

# MVT protobuf tile
run_binary_test "vector tile (MVT)" \
    "$FRONTEND_URL/tileserver/areamap-mvt/$ZOOM/$TILE_X/$TILE_Y" \
    "application/x-protobuf" \
    100

# MLT tile, negotiated exactly as the web app requests it
run_binary_test "vector tile (MLT)" \
    "$FRONTEND_URL/tileserver/areamap-mlt/$ZOOM/$TILE_X/$TILE_Y" \
    "application/vnd.maplibre-tile" \
    100 \
    "application/vnd.maplibre-tile"

print_test_summary "Tileserver"
