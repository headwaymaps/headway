# See latest releases with: aws s3 ls --no-sign-request s3://osm-planet-us-west-2/planet/pbf/2025/
# And update the HEADWAY_PLANET_VERSION accordingly. The build scripts will fetch the corresponding osm.pbf
export HEADWAY_PLANET_VERSION=v1.250908
export HEADWAY_AREA=maps-earth-planet-${HEADWAY_PLANET_VERSION}
export HEADWAY_AREA_TAG="$HEADWAY_AREA"
export HEADWAY_COUNTRIES="ALL"
export HEADWAY_PUBLIC_URL=https://maps.earth
export HEADWAY_SERVICE_PORT=30400
export HEADWAY_ENABLE_TRANSIT_ROUTING=1
export HEADWAY_TRANSIT_AREAS="Barcelona LosAngeles PugetSound"
export PELIAS_ELASTICSEARCH_MEMORY_REQUEST=8Gi
export VALHALLA_MEMORY_REQUEST=4Gi
export HEADWAY_ABOUT_URL="https://about.maps.earth"
export HEADWAY_ABOUT_LINK_TEXT="About maps.earth"
export HEADWAY_CONTACT_URL="mailto:info@maps.earth?subject=Hello,%20Earth"
export HEADWAY_CONTACT_LINK_TEXT="Contact Us"
