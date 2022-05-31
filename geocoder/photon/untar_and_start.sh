#!/bin/bash

cd / && tar xvf /data/${HEADWAY_AREA}.photon.tgz && cd /photon && sudo -E -u photon java -jar /photon/photon.jar