apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: tileserver-${HEADWAY_AREA_TAG_SAFE}-${HEADWAY_DATA_TAG_SAFE}
  labels:
    app.kubernetes.io/part-of: headway
    headway/area-tag: ${HEADWAY_AREA_TAG_SAFE}
    headway/data-tag: ${HEADWAY_DATA_TAG_SAFE}
spec:
  accessModes: [ "ReadWriteOnce" ]
  resources:
    requests:
      storage: 200Gi
