apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: opentripplanner-${TRANSIT_ZONE}-${HEADWAY_AREA_TAG_SAFE}-${HEADWAY_DATA_TAG_SAFE}-${OTP_GRAPH_DATE}
  labels:
    app.kubernetes.io/part-of: headway
    headway/area-tag: ${HEADWAY_AREA_TAG_SAFE}
    headway/data-tag: ${HEADWAY_DATA_TAG_SAFE}
    headway/graph-date: "${OTP_GRAPH_DATE}"
spec:
  accessModes: [ "ReadWriteOnce" ]
  resources:
    requests:
      storage: 1Gi
