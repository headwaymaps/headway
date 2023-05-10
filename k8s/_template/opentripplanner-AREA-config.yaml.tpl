apiVersion: v1
kind: ConfigMap
metadata:
  name: otp-${TRANSIT_AREA}-config
data:
  graph-url: ${OTP_GRAPH_URL}
  router-config-json: ${OTP_ROUTER_CONFIG_JSON_YAML}

