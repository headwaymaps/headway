apiVersion: v1
kind: ConfigMap
metadata:
  name: deployment-config
data:
  area: ${HEADWAY_AREA}
  public-url: ${HEADWAY_PUBLIC_URL}
  bbox: "${HEADWAY_BBOX}"
  enable-transit-routing: "${HEADWAY_ENABLE_TRANSIT_ROUTING}"
  font-source-url: ${HEADWAY_K8S_ARTIFACT_ROOT}/${ASSET_VERSION}/fonts.tar
  sprite-source-url: ${HEADWAY_K8S_ARTIFACT_ROOT}/${ASSET_VERSION}/sprite.tar
  natural-earth-source-url: ${HEADWAY_K8S_ARTIFACT_ROOT}/${ASSET_VERSION}/natural_earth.mbtiles
  mbtiles-source-url: ${HEADWAY_K8S_ARTIFACT_ROOT}/${ASSET_VERSION}/${HEADWAY_AREA_TAG}/${HEADWAY_AREA}.mbtiles
  valhalla-artifact-url: ${HEADWAY_K8S_ARTIFACT_ROOT}/${ASSET_VERSION}/${HEADWAY_AREA_TAG}/${HEADWAY_AREA}.valhalla.tar.xz
  placeholder-artifact-url: ${HEADWAY_K8S_ARTIFACT_ROOT}/${ASSET_VERSION}/${HEADWAY_AREA_TAG}/${HEADWAY_AREA}.placeholder.tar.xz
  elasticsearch-artifact-url: ${HEADWAY_K8S_ARTIFACT_ROOT}/${ASSET_VERSION}/${HEADWAY_AREA_TAG}/${HEADWAY_AREA}.elasticsearch.tar.xz
  otp-graph-url: ${HEADWAY_K8S_ARTIFACT_ROOT}/${ASSET_VERSION}/${HEADWAY_AREA_TAG}/${HEADWAY_AREA}.graph.obj.xz
  pelias-config-json: ${PELIAS_CONFIG_JSON_YAML}
