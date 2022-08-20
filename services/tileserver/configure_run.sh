#!/bin/bash

set -xe

export ESC="\$"

envsubst < /app/config.json.template > /app/config.json
envsubst < /styles/style.json.template > /app/styles/style.json

cat /app/config.json

tileserver-gl-light --config /app/config.json
