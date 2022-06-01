#!/bin/bash
if [ ! -z ${HEADWAY_AREA} ]
then
  [[ -e ${HEADWAY_BBOX_PATH} ]] && echo "WARN: overwriting existing ${HEADWAY_BBOX_PATH} with bbox"
  grep "^${HEADWAY_AREA}:" /frontend/bboxes.csv | cut -d':' -f2 > ${HEADWAY_BBOX_PATH}
elif [ ! -e ${HEADWAY_BBOX_PATH} ]
then
  echo "ERR: ${HEADWAY_BBOX_PATH} does not exist and \$HEADWAY_AREA not set"
  exit 1
fi
/docker-entrypoint.sh nginx -g 'daemon off;'
