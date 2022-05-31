#!/bin/bash

cp /data/*graph.obj /otp/graph.obj

java -jar otp-shaded.jar --load /otp
