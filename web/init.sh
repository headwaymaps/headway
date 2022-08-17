#!/bin/bash

set -xe

if [ ! -z "${HEADWAY_FORCE_BBOX}" ]
then
  echo "${HEADWAY_FORCE_BBOX}" > ${HEADWAY_BBOX_PATH}
elif [ ! -z ${HEADWAY_AREA} ]
then
  [[ -e ${HEADWAY_BBOX_PATH} ]] && echo "WARN: overwriting existing ${HEADWAY_BBOX_PATH} with bbox"
  grep "^${HEADWAY_AREA}:" /frontend/bboxes.csv | cut -d':' -f2 > ${HEADWAY_BBOX_PATH}
elif [ ! -e ${HEADWAY_BBOX_PATH} ]
then
  echo "ERR: ${HEADWAY_BBOX_PATH} does not exist and \$HEADWAY_AREA not set"
  exit 1
fi

sleep 1 # Hack: make sure the OTP image has had a chance to start.

echo "BASEMAP" >> ${HEADWAY_CAPABILITIES_PATH}
echo "VALHALLA" >> ${HEADWAY_CAPABILITIES_PATH}
echo "PELIAS" >> ${HEADWAY_CAPABILITIES_PATH}
# Hack: The OTP image start command listens on 9999 if it has a transit graph.
echo > /dev/tcp/otp/9999 && echo "OTP" >> ${HEADWAY_CAPABILITIES_PATH}

/docker-entrypoint.sh nginx -g 'daemon off;'
