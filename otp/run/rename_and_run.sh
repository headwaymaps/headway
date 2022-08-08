#!/bin/bash

if [[ -s /data/${HEADWAY_AREA}.graph.obj ]]; then
    cp /data/${HEADWAY_AREA}.graph.obj /otp/graph.obj
    java ${JAVA_MEM_ARGS} -jar otp-shaded.jar --load /otp
fi
