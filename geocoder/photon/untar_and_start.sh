#!/bin/bash

set -xe

echo "Extracting photon index"

cd / && time cat /data/${HEADWAY_AREA}.photon.tar.bz2 | pbzip2 -d | tar x

echo "Starting photon"

cd /photon && sudo -E -u photon java -jar /photon/photon.jar
