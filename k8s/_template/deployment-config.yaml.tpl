apiVersion: v1
kind: ConfigMap
metadata:
  name: deployment-config
data:
  area: ${HEADWAY_VERSION}
  public-url: https://maps.endworld.org
  font-source-url: ${S3_ROOT}/${HEADWAY_VERSION}/fonts.tar
  sprite-source-url: ${S3_ROOT}/${HEADWAY_VERSION}/sprite.tar
  natural-earth-source-url: ${S3_ROOT}/${HEADWAY_VERSION}/natural_earth.mbtiles
  mbtiles-source-url: ${S3_ROOT}/${HEADWAY_VERSION}/${HEADWAY_AREA_TAG}/${HEADWAY_AREA}.mbtiles
  valhalla-artifact-url: ${S3_ROOT}/${HEADWAY_VERSION}/${HEADWAY_AREA_TAG}/${HEADWAY_AREA}.valhalla.tar
  pelias-config-artifact-url: ${S3_ROOT}/${HEADWAY_VERSION}/${HEADWAY_AREA_TAG}/${HEADWAY_AREA}.pelias.json
  placeholder-artifact-url: ${S3_ROOT}/${HEADWAY_VERSION}/${HEADWAY_AREA_TAG}/${HEADWAY_AREA}.placeholder.tar
  elasticsearch-artifact-url: ${S3_ROOT}/${HEADWAY_VERSION}/${HEADWAY_AREA_TAG}/${HEADWAY_AREA}.elasticsearch.tar
  otp-graph-url: ${S3_ROOT}/${HEADWAY_VERSION}/${HEADWAY_AREA_TAG}/${HEADWAY_AREA}.graph.obj
