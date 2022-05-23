#!/bin/bash

mkdir -p /tmp_vol/valhalla_tiles

cd /tmp_vol/valhalla_tiles

valhalla_build_config --mjolnir-tile-dir /tmp_vol/valhalla_tiles --mjolnir-tile-extract /tmp_vol/valhalla_tiles.tar --mjolnir-timezone /tmp_vol/valhalla_tiles/timezones.sqlite --mjolnir-admin /tmp_vol/valhalla_tiles/admins.sqlite > valhalla.json
valhalla_build_timezones > /tmp_vol/valhalla_tiles/timezones.sqlite
valhalla_build_tiles -c valhalla.json /data_vol/data.osm.pbf

cd /tmp_vol/valhalla_tiles && find | sort -n | tar -cf /tmp_vol/valhalla_tiles.tar --no-recursion -T -