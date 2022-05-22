#!/bin/bash

# Copyright (C) 2022 Ellen Poe

# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.

# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU Affero General Public License for more details.

# You should have received a copy of the GNU Affero General Public License
# along with this program. If not, see https://www.gnu.org/licenses/.

mkdir -p /tmp_vol/valhalla_tiles

cd /tmp_vol/valhalla_tiles

valhalla_build_config --mjolnir-tile-dir /tmp_vol/valhalla_tiles --mjolnir-tile-extract /tmp_vol/valhalla_tiles.tar --mjolnir-timezone /tmp_vol/valhalla_tiles/timezones.sqlite --mjolnir-admin /tmp_vol/valhalla_tiles/admins.sqlite > valhalla.json
valhalla_build_timezones > /tmp_vol/valhalla_tiles/timezones.sqlite
valhalla_build_tiles -c valhalla.json /data_vol/data.osm.pbf

cd /tmp_vol/valhalla_tiles && find | sort -n | tar -cf /tmp_vol/valhalla_tiles.tar --no-recursion -T -