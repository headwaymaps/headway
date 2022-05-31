#!/bin/bash

cp /data/${HEADWAY_AREA}.valhalla_tiles.tar /custom_files/valhalla_tiles.tar

# This hash is necessary for valhalla to pick up the tiles.
md5sum /custom_files/valhalla_tiles.tar > /custom_files/file_hashes.txt