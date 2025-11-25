#!/bin/bash
# Test routing endpoints via frontend proxy

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/_test-lib.sh"

FRONTEND_URL="${FRONTEND_URL:-http://localhost:8080}"

# Test coordinates for Bogota
# Plaza de Bolivar (downtown)
START_LAT=4.5981
START_LNG=-74.0758

# Nearby location (short route)
END_LAT=4.6029
END_LNG=-74.067

echo "Testing Routing..."
echo ""

# Helper function to build routing URL
build_route_url() {
    local mode=$1
    local from_lat=${2:-$START_LAT}
    local from_lng=${3:-$START_LNG}
    local to_lat=${4:-$END_LAT}
    local to_lng=${5:-$END_LNG}
    local num_itineraries=${6:-3}

    echo "${FRONTEND_URL}/travelmux/v6/plan?fromPlace=${from_lat}%2C${from_lng}&toPlace=${to_lat}%2C${to_lng}&numItineraries=${num_itineraries}&mode=${mode}&preferredDistanceUnits=kilometers"
}

# Test 1: Walking route
run_jq_test "walking route" \
    "$(build_route_url WALK)" \
    '.plan' \
    '.plan.itineraries' \
    '.plan.itineraries | length > 0' \
    '.plan.itineraries[0].duration' \
    '.plan.itineraries[0].legs'

# Test 2: Car route
run_jq_test "car route" \
    "$(build_route_url CAR)" \
    '.plan' \
    '.plan.itineraries' \
    '.plan.itineraries | length > 0'

print_test_summary "Routing"
