#!/bin/bash
# Test geosearch (Pelias) endpoints via frontend proxy

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/_test-lib.sh"

FRONTEND_URL="${FRONTEND_URL:-http://localhost:8080}"

# Test coordinates for Bogota
# Plaza de Bolivar
PLAZA_LAT=4.5981
PLAZA_LNG=-74.0758

echo "Testing Geosearch..."
echo ""

# Test 1: Autocomplete search
run_jq_test "autocomplete search" \
    "$FRONTEND_URL/pelias/v1/autocomplete?text=Plaza&focus.point.lat=$PLAZA_LAT&focus.point.lon=$PLAZA_LNG" \
    '.features' \
    '.features | length > 0' \
    '.features[0].properties.name' \
    '.features[0].properties.name| test("Plaza de Bolívar"; "i")' \
    '.features[0].geometry.coordinates'

# Test 2: Reverse geocoding
run_jq_test "reverse geocoding" \
    "$FRONTEND_URL/pelias/v1/reverse?point.lat=$PLAZA_LAT&point.lon=$PLAZA_LNG" \
    '.features' \
    '.features | length > 0' \
    '.features[0].geometry.coordinates' \
    '.features[0].properties.name'

print_test_summary "Geosearch"
