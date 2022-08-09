#!/bin/bash

set -xe

mkdir -p /data/valhalla/

cd /valhalla_data && time cat /data_mount/${HEADWAY_AREA}.valhalla.tar.bz2 | pbzip2 -d | tar x

valhalla_build_config --mjolnir-timezone /valhalla_data/timezones.sqlite --mjolnir-admin /valhalla_data/admins.sqlite > valhalla.json

valhalla_service valhalla.json
