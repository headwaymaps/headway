#!/bin/bash

set -xe

if [[ -s /data/${HEADWAY_AREA}.graph.obj ]]; then
    netcat -l -p 9999 # Hack: Signal to clients that we have a transit graph.

    mkdir /servedir
    ln -s /data/${HEADWAY_AREA}.graph.obj /servedir/graph.obj

    java ${JAVA_MEM_ARGS} -jar /otp/otp-shaded.jar --load /servedir
else
    while :
    do
        sleep 1
    done
fi
