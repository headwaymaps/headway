#!/bin/bash

cd /build_tiles

mkdir -p /vol/valhalla_tiles

valhalla_build_config --mjolnir-tile-dir /vol/valhalla_tiles --mjolnir-tile-extract /vol/valhalla_tiles.tar --mjolnir-timezone /vol/valhalla_tiles/timezones.sqlite --mjolnir-admin /vol/valhalla_tiles/admins.sqlite > valhalla.json
valhalla_build_timezones > /vol/valhalla_tiles/timezones.sqlite
valhalla_build_tiles -c valhalla.json /vol/data.osm.pbf

cd /vol/valhalla_tiles && find | sort -n | tar -cf /vol/valhalla_tiles.tar --no-recursion -T -