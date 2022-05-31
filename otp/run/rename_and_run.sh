#!/bin/bash

cp /data/${HEADWAY_AREA}.graph.obj /otp/graph.obj

java ${JAVA_MEM_ARGS} -jar otp-shaded.jar --load /otp
