apiVersion: v1
kind: ConfigMap
metadata:
  name: otp-${TRANSIT_AREA}-config
data:
  graph-url: ${OTP_GRAPH_URL}
  otp-config-json: ${OTP_CONFIG_JSON_YAML}
  router-config-json: ${OTP_ROUTER_CONFIG_JSON_YAML}

