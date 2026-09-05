apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: opentripplanner-${TRANSIT_ZONE}-${HEADWAY_AREA_TAG_SAFE}-${HEADWAY_DATA_TAG_SAFE}-${OTP_VOLUME_VERSION}
  labels:
    app.kubernetes.io/part-of: headway
spec:
  accessModes: [ "ReadWriteOnce" ]
  resources:
    requests:
      storage: 1Gi
