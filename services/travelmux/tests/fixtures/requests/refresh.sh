set -ex

# Fetch and format real responses from a locally running server - i.e. when the schema changes
function fetch_valhalla {
    mode=$1
    output_prefix="valhalla_$(echo "$mode" | tr '[:upper:]' '[:lower:]')"

    # from realFine coffee in West Seattle to Zeitgeist downtown Seattle
    json_data="{
        \"locations\": [
            {\"lat\": 47.575837, \"lon\": -122.339414},
            {\"lat\": 47.651048, \"lon\": -122.347234}
        ],
        \"costing\": \"$mode\",
        \"alternates\": 3,
        \"units\": \"miles\"
    }"
    encoded_json=$(echo $json_data | jq -c . | sed 's/ /%20/g; s/{/%7B/g; s/}/%7D/g; s/:/%3A/g; s/,/%2C/g; s/\"/%22/g; s/\[/%5B/g; s/\]/%5D/g')
    curl "http://localhost:9001/route?json=$encoded_json" | jq -S . > "${output_prefix}_route.json"
}

fetch_valhalla pedestrian
fetch_valhalla bicycle
fetch_valhalla auto # car

# realFine coffee in West Seattle
from_lat=47.575837
from_lon=-122.339414
# Zeitgeist downtown Seattle
to_lat=47.651048
to_lon=-122.347234

# OTP removed its REST /plan endpoint in 2.8. We now query the GTFS GraphQL `planConnection` at
# /otp/gtfs/v1. `$modes` is a GraphQL PlanModesInput literal, e.g.
#   '{ directOnly: true, direct: [WALK] }' or '{ transit: { access: [WALK], egress: [WALK] } }'.
#
# Keep the selection below in sync with the query fragments in src/otp/gtfs_graphql.rs - the tests
# deserialize these files with them.
#
# The `*_plan.json` fixtures alongside these are the *old* REST responses, which back the v6 tests.
# There's no endpoint left to re-capture those from; the ones here were derived from them.
function fetch_opentripplanner {
    name=$1
    modes=$2

    query="query {
      planConnection(
        origin: { location: { coordinate: { latitude: ${from_lat}, longitude: ${from_lon} } } }
        destination: { location: { coordinate: { latitude: ${to_lat}, longitude: ${to_lon} } } }
        first: 5
        modes: ${modes}
      ) {
        edges { node {
          start end duration walkDistance
          legs {
            mode transitLeg distance duration realTime headsign
            start { scheduledTime estimated { time } }
            end { scheduledTime estimated { time } }
            from { name lat lon arrival { scheduledTime estimated { time } } departure { scheduledTime estimated { time } } }
            to { name lat lon arrival { scheduledTime estimated { time } } departure { scheduledTime estimated { time } } }
            legGeometry { points length }
            route { shortName longName color }
            agency { name }
            steps { distance relativeDirection absoluteDirection streetName lat lon area bogusName stayOn exit }
            alerts { alertHeaderText alertDescriptionText alertUrl effectiveStartDate effectiveEndDate }
          }
        } }
        routingErrors { code description }
      }
    }"

    jq -n --arg q "$query" '{query: $q}' \
      | curl -s -X POST -H "Content-Type: application/json" -d @- "http://localhost:9002/otp/gtfs/v1" \
      | jq -S . > "opentripplanner_${name}_planconnection.json"
}

fetch_opentripplanner walk '{ directOnly: true, direct: [WALK] }'
fetch_opentripplanner bicycle '{ directOnly: true, direct: [BICYCLE] }'
fetch_opentripplanner transit '{ transit: { access: [WALK], egress: [WALK], transfer: [WALK] } }'
fetch_opentripplanner transit_with_bicycle '{ transit: { access: [BICYCLE], egress: [BICYCLE], transfer: [BICYCLE] } }'

