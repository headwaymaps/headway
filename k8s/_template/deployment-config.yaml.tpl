apiVersion: v1
kind: ConfigMap
metadata:
  name: deployment-config
data:
  public-url: ${HEADWAY_PUBLIC_URL}
  bbox: "${HEADWAY_BBOX}"
  enable-transit-routing: "${HEADWAY_ENABLE_TRANSIT_ROUTING}"
  www-about-url: "${HEADWAY_ABOUT_URL}"
  www-about-link-text: "${HEADWAY_ABOUT_LINK_TEXT}"
  www-contact-url: "${HEADWAY_CONTACT_URL}"
  www-contact-link-text: "${HEADWAY_CONTACT_LINK_TEXT}"
  terrain-source-url: ${TERRAIN_ARTIFACT_URL}
  landcover-source-url: ${LANDCOVER_ARTIFACT_URL}
  areamap-source-url: ${AREAMAP_ARTIFACT_URL}
  valhalla-artifact-url: ${VALHALLA_ARTIFACT_URL}
  placeholder-artifact-url: ${PLACEHOLDER_ARTIFACT_URL}
  elasticsearch-artifact-url: ${ELASTICSEARCH_ARTIFACT_URL}
  elevation-artifact-url: "${ELEVATION_ARTIFACT_URL}"
  pelias-config-json: ${PELIAS_CONFIG_JSON_YAML}
