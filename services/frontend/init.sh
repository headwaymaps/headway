#!/bin/bash

set -xe
set -o pipefail

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

if [ -z "${HEADWAY_SHARED_VOL}" ]; then
    echo "Expecting HEADWAY_SHARED_VOL to be set."
    exit 1
fi
"${SCRIPT_DIR}/generate_config.sh" > $HEADWAY_SHARED_VOL/headway-config.json
