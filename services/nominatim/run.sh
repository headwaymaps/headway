#!/bin/bash

set -xe

(cd ${PROJECT_DIR} && tar xvf /nominatim_extra/tokenizer.tar)

/app/start.sh
