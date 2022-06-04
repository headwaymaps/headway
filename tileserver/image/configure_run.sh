#!/bin/bash

set -xe

envsubst < /app/config.json.template > /app/config.json
envsubst < /app/style_templates/bright.json.template > /app/styles/bright.json

cat /app/config.json

tileserver-gl-light --config /app/config.json
