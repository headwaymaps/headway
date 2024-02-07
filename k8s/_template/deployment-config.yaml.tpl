apiVersion: v1
kind: ConfigMap
metadata:
  name: deployment-config
data:
  area: ${HEADWAY_AREA}
  public-url: ${HEADWAY_PUBLIC_URL}
  bbox: "${HEADWAY_BBOX}"
  enable-transit-routing: "${HEADWAY_ENABLE_TRANSIT_ROUTING}"
  www-about-url: "${HEADWAY_ABOUT_URL}"
  www-about-link-text: "${HEADWAY_ABOUT_LINK_TEXT}"
  www-contact-url: "${HEADWAY_CONTACT_URL}"
  www-contact-link-text: "${HEADWAY_CONTACT_LINK_TEXT}"
  terrain-source-url: ${HEADWAY_K8S_ARTIFACT_ROOT}/${HEADWAY_DATA_TAG}/terrain.mbtiles
  landcover-source-url: ${HEADWAY_K8S_ARTIFACT_ROOT}/${HEADWAY_DATA_TAG}/landcover.mbtiles
  areamap-source-url: ${HEADWAY_K8S_ARTIFACT_ROOT}/${HEADWAY_DATA_TAG}/${HEADWAY_AREA_TAG}/${HEADWAY_AREA}.mbtiles
  valhalla-artifact-url: ${HEADWAY_K8S_ARTIFACT_ROOT}/${HEADWAY_DATA_TAG}/${HEADWAY_AREA_TAG}/${HEADWAY_AREA}.valhalla.tar.zst
  placeholder-artifact-url: ${HEADWAY_K8S_ARTIFACT_ROOT}/${HEADWAY_DATA_TAG}/${HEADWAY_AREA_TAG}/${HEADWAY_AREA}.placeholder.tar.zst
  elasticsearch-artifact-url: ${HEADWAY_K8S_ARTIFACT_ROOT}/${HEADWAY_DATA_TAG}/${HEADWAY_AREA_TAG}/${HEADWAY_AREA}.elasticsearch.tar.zst${OTP_GRAPHS_YAML}
  pelias-config-json: ${PELIAS_CONFIG_JSON_YAML}
