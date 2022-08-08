#!/bin/bash

set -xe

if [[ -s /data/${HEADWAY_AREA}.graph.obj ]]; then
    netcat -l -p 9999 # Hack: Signal to clients that we have a transit graph.

    cp /data/${HEADWAY_AREA}.graph.obj /otp/graph.obj
    java ${JAVA_MEM_ARGS} -jar otp-shaded.jar --load /otp
else
    while :
    do
        sleep 1
    done
fi
