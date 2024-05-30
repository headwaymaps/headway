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

realFine="47.575837,-122.339414"
zeitgeist="47.651048,-122.347234"

function fetch_opentripplanner {
    mode=$1
    output_prefix="opentripplanner_$(echo "$mode" | tr '[:upper:]' '[:lower:]' | sed 's/,/_with_/')"
    request_url="http://localhost:9002/otp/routers/default/plan?fromPlace=${realFine}&toPlace=${zeitgeist}&mode=${mode}"

    # Make the request and save the output to a file
    curl "$request_url" | jq -S . > "${output_prefix}_plan.json"
}

fetch_opentripplanner WALK
fetch_opentripplanner BICYCLE
fetch_opentripplanner TRANSIT
fetch_opentripplanner "TRANSIT,BICYCLE"

