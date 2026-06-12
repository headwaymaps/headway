apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: elasticsearch-${HEADWAY_AREA_TAG_SAFE}-${HEADWAY_DATA_TAG_SAFE}
spec:
  accessModes: [ "ReadWriteOnce" ]
  resources:
    requests:
      storage: 90Gi
