#!/bin/bash -x

rm -fr data/download
rm -fr data/generated
rm -fr data/planet-download
rm -fr data/maps-earth-planet-v1.*{.osm.pbf,.elasticsearch.tar.zst,.mbtiles,.pelias.json,.placeholder.tar.zst,.valhalla.tar.zst,.graph.obj.zst,.gtfs.tar.zst}
dagger core engine local-cache prune

