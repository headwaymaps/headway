#!/bin/bash

if [[ $(tar -tf gtfs.tar) ]]; then
    java ${JAVA_MEM_ARGS} -jar /otp/otp-shaded.jar --build --save .
else
    echo "Empty GTFS tarball, skipping build"
    touch graph.obj # Create empty graph to signal run image to do nothing.
fi
