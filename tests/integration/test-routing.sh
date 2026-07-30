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

# Portal Norte (north end of the city) - far enough that transit beats walking
TRANSIT_END_LAT=4.7085
TRANSIT_END_LNG=-74.0425

# Depart at noon rather than "now", so the test doesn't depend on there being service at
# whatever time it happens to run. Times without an offset are in the graph's local timezone, so
# pair it with today's date in Bogota.
TRANSIT_DATE=$(TZ=America/Bogota date +%Y-%m-%d)
TRANSIT_TIME="12:00"

echo "Testing Routing..."
echo ""

# Helper function to build routing URL
build_route_url() {
    local version=$1
    local mode=$2
    local from_lat=${3:-$START_LAT}
    local from_lng=${4:-$START_LNG}
    local to_lat=${5:-$END_LAT}
    local to_lng=${6:-$END_LNG}
    local num_itineraries=${7:-3}

    echo "${FRONTEND_URL}/travelmux/${version}/plan?fromPlace=${from_lat}%2C${from_lng}&toPlace=${to_lat}%2C${to_lng}&numItineraries=${num_itineraries}&mode=${mode}&preferredDistanceUnits=kilometers"
}

# === v6 ===

# Test 1: Walking route
run_jq_test "v6 walking route" \
    "$(build_route_url v6 WALK)" \
    '.plan' \
    '.plan.itineraries' \
    '.plan.itineraries | length > 0' \
    '.plan.itineraries[0].duration' \
    '.plan.itineraries[0].legs'

# Test 2: Car route
run_jq_test "v6 car route" \
    "$(build_route_url v6 CAR)" \
    '.plan' \
    '.plan.itineraries' \
    '.plan.itineraries | length > 0'

# Test 3: Transit route
# Only for builds with transit routing enabled - it's the one mode served by OTP rather than
# Valhalla, so it also covers travelmux finding the OTP instance covering these coordinates.
if [ "${HEADWAY_ENABLE_TRANSIT_ROUTING:-0}" != 0 ]; then
    transit_url="$(build_route_url v6 TRANSIT "$START_LAT" "$START_LNG" "$TRANSIT_END_LAT" "$TRANSIT_END_LNG")"
    transit_url="${transit_url}&date=${TRANSIT_DATE}&time=${TRANSIT_TIME}"

    # The last validation is the one that proves transit data was actually loaded and used:
    # a leg riding a vehicle, rather than an all-walking itinerary.
    run_jq_test "v6 transit route" \
        "$transit_url" \
        '.plan' \
        '.plan.itineraries' \
        '.plan.itineraries | length > 0' \
        '[.plan.itineraries[].legs[] | select(.mode == "TRANSIT")] | length > 0' \
        '[.plan.itineraries[].legs[] | select(.mode == "TRANSIT") | .transitLeg.mode | select(test("^(BUS|TRAM|SUBWAY|RAIL|FERRY|CABLE_CAR|GONDOLA|FUNICULAR)$"))] | length > 0'
else
    echo "  Skipping v6 transit route (HEADWAY_ENABLE_TRANSIT_ROUTING is not set)"
fi

# === v7 ===
# Itineraries are no longer nested under `plan`, times are RFC 3339, and distances are meters.

run_jq_test "v7 walking route" \
    "$(build_route_url v7 WALK)" \
    '.itineraries' \
    '.itineraries | length > 0' \
    '.itineraries[0].durationSeconds' \
    '.itineraries[0].distanceMeters' \
    '.itineraries[0].startTime | test("^[0-9]{4}-[0-9]{2}-[0-9]{2}T")' \
    '.itineraries[0].legs'

run_jq_test "v7 car route" \
    "$(build_route_url v7 CAR)" \
    '.itineraries' \
    '.itineraries | length > 0'

if [ "${HEADWAY_ENABLE_TRANSIT_ROUTING:-0}" != 0 ]; then
    transit_url="$(build_route_url v7 TRANSIT "$START_LAT" "$START_LNG" "$TRANSIT_END_LAT" "$TRANSIT_END_LNG")"
    transit_url="${transit_url}&dateTime=${TRANSIT_DATE}T${TRANSIT_TIME}"

    run_jq_test "v7 transit route" \
        "$transit_url" \
        '.itineraries' \
        '.itineraries | length > 0' \
        '[.itineraries[].legs[] | select(.mode == "TRANSIT")] | length > 0' \
        '[.itineraries[].legs[] | select(.mode == "TRANSIT") | .transitLeg.vehicleMode | select(test("^(BUS|TRAM|SUBWAY|RAIL|FERRY|CABLE_CAR|GONDOLA|FUNICULAR)$"))] | length > 0'
else
    echo "  Skipping v7 transit route (HEADWAY_ENABLE_TRANSIT_ROUTING is not set)"
fi

print_test_summary "Routing"
