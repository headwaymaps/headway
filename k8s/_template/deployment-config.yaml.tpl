apiVersion: v1
kind: ConfigMap
metadata:
  name: deployment-config
data:
  area: ${HEADWAY_AREA}
  public-url: ${HEADWAY_PUBLIC_URL}
  bbox: "${HEADWAY_BBOX}"
  enable-transit-routing: "${HEADWAY_ENABLE_TRANSIT_ROUTING}"
  natural-earth-source-url: ${HEADWAY_K8S_ARTIFACT_ROOT}/${HEADWAY_SCHEMA_VERSION}/natural_earth.mbtiles
  mbtiles-source-url: ${HEADWAY_K8S_ARTIFACT_ROOT}/${HEADWAY_SCHEMA_VERSION}/${HEADWAY_AREA_TAG}/${HEADWAY_AREA}.mbtiles
  valhalla-artifact-url: ${HEADWAY_K8S_ARTIFACT_ROOT}/${HEADWAY_SCHEMA_VERSION}/${HEADWAY_AREA_TAG}/${HEADWAY_AREA}.valhalla.tar.zst
  placeholder-artifact-url: ${HEADWAY_K8S_ARTIFACT_ROOT}/${HEADWAY_SCHEMA_VERSION}/${HEADWAY_AREA_TAG}/${HEADWAY_AREA}.placeholder.tar.zst
  elasticsearch-artifact-url: ${HEADWAY_K8S_ARTIFACT_ROOT}/${HEADWAY_SCHEMA_VERSION}/${HEADWAY_AREA_TAG}/${HEADWAY_AREA}.elasticsearch.tar.zst${OTP_GRAPHS_YAML}
  pelias-config-json: ${PELIAS_CONFIG_JSON_YAML}
