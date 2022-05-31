#!/bin/bash

set -xe

mkdir -p /data/valhalla/

cp /data_mount/${HEADWAY_AREA}.valhalla.tar /data/valhalla/tiles.tar

cd /valhalla_data && tar -zxvf /data/valhalla/tiles.tar timezones.sqlite admins.sqlite

valhalla_build_config --mjolnir-timezone /valhalla_data/timezones.sqlite --mjolnir-admin /valhalla_data/admins.sqlite > valhalla.json

valhalla_service valhalla.json
