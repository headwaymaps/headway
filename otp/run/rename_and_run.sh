#!/bin/bash

cp /data/*graph.obj /otp/graph.obj

java ${JAVA_MEM_ARGS} -jar otp-shaded.jar --load /otp
