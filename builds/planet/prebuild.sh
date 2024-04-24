#!/bin/bash -e

if [ -z "$HEADWAY_PLANET_VERSION" ]; then
    echo "missing HEADWAY_PLANET_VERSION"
    exit 1
fi

OUTPUT_PBF="maps-earth-planet-${HEADWAY_PLANET_VERSION}.osm.pbf"
if [[ -f "${OUTPUT_PBF}" ]]; then
    echo "pbf already exists: $(ls -l $OUTPUT_PBF)"
    exit 0
fi

OUTPUT_ROOT=../../../data
(cd $(dirname "$0") && \
    cd assemble-planet-pbf && \
    cargo run --release -- "$HEADWAY_PLANET_VERSION" "$OUTPUT_ROOT" && \
    mv "${OUTPUT_ROOT}/generated/${HEADWAY_PLANET_VERSION}/final-planet-${HEADWAY_PLANET_VERSION}.osm.pbf" ../../../${OUTPUT_PBF})
