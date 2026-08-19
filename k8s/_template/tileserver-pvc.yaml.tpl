apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: tileserver-${HEADWAY_AREA_TAG_SAFE}-${HEADWAY_DATA_TAG_SAFE}-${TILESERVER_VOLUME_VERSION}
  labels:
    app.kubernetes.io/part-of: headway
spec:
  accessModes: [ "ReadWriteOnce" ]
  resources:
    requests:
      storage: 200Gi
