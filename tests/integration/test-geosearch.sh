#!/bin/bash
# Test geosearch (Pelias) endpoints via frontend proxy

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/_test-lib.sh"

SEARCH_TEXT_URI=$(jq -rn --arg text "$SEARCH_TEXT" '$text | @uri')

echo "Testing Geosearch..."
echo ""

# Test 1: Autocomplete search
run_jq_test "autocomplete search" \
    "$HEADWAY_PUBLIC_URL/pelias/v1/autocomplete?text=$SEARCH_TEXT_URI&focus.point.lat=$START_LAT&focus.point.lon=$START_LNG" \
    '.features' \
    '.features | length > 0' \
    '.features[0].properties.name' \
    ".features[0].properties.name | test(\"$SEARCH_RESULT\"; \"i\")" \
    '.features[0].geometry.coordinates'

# Test 2: Reverse geocoding
run_jq_test "reverse geocoding" \
    "$HEADWAY_PUBLIC_URL/pelias/v1/reverse?point.lat=$START_LAT&point.lon=$START_LNG" \
    '.features' \
    '.features | length > 0' \
    '.features[0].geometry.coordinates' \
    '.features[0].properties.name'

print_test_summary "Geosearch"
