apiVersion: v1
kind: ConfigMap
metadata:
  name: deployment-config
data:
  area: ${HEADWAY_VERSION}
  public-url: ${HEADWAY_PUBLIC_URL}
  font-source-url: ${HEADWAY_S3_ROOT}/${HEADWAY_VERSION}/fonts.tar
  sprite-source-url: ${HEADWAY_S3_ROOT}/${HEADWAY_VERSION}/sprite.tar
  natural-earth-source-url: ${HEADWAY_S3_ROOT}/${HEADWAY_VERSION}/natural_earth.mbtiles
  mbtiles-source-url: ${HEADWAY_S3_ROOT}/${HEADWAY_VERSION}/${HEADWAY_AREA_TAG}/${HEADWAY_AREA}.mbtiles
  valhalla-artifact-url: ${HEADWAY_S3_ROOT}/${HEADWAY_VERSION}/${HEADWAY_AREA_TAG}/${HEADWAY_AREA}.valhalla.tar.xz
  pelias-config-artifact-url: ${HEADWAY_S3_ROOT}/${HEADWAY_VERSION}/${HEADWAY_AREA_TAG}/${HEADWAY_AREA}.pelias.json
  placeholder-artifact-url: ${HEADWAY_S3_ROOT}/${HEADWAY_VERSION}/${HEADWAY_AREA_TAG}/${HEADWAY_AREA}.placeholder.tar.xz
  elasticsearch-artifact-url: ${HEADWAY_S3_ROOT}/${HEADWAY_VERSION}/${HEADWAY_AREA_TAG}/${HEADWAY_AREA}.elasticsearch.tar.xz
  otp-graph-url: ${HEADWAY_S3_ROOT}/${HEADWAY_VERSION}/${HEADWAY_AREA_TAG}/${HEADWAY_AREA}.graph.obj
