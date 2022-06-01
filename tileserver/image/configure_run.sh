#!/bin/bash

set -xe

envsubst < config.json.template > /app/config.json

tileserver-gl --config /app/config.json
