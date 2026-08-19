apiVersion: apps/v1
kind: Deployment
metadata:
  name: valhalla
spec:
  selector:
    matchLabels:
      app: valhalla
  replicas: 1
  strategy:
    type: RollingUpdate
    rollingUpdate:
      # Surge a new pod before retiring the old. On a version bump the two pods
      # claim *different* PVCs, so the new one downloads its artifact while the
      # old keeps serving. On a same-version restart they share one ReadWriteOnce
      # claim: fine, since the scheduler only places them where it can attach.
      maxSurge: 1
      maxUnavailable: 0
  template:
    metadata:
      labels:
        app: valhalla
    spec:
      initContainers:
        - name: init
          image: ghcr.io/headwaymaps/valhalla-init:${HEADWAY_CONTAINER_TAG}
          imagePullPolicy: Always
          volumeMounts:
            - name: valhalla-volume
              mountPath: /data
          env:
            - name: VALHALLA_ARTIFACT_URL
              valueFrom:
                configMapKeyRef:
                  name: deployment-config
                  key: valhalla-artifact-url
          resources:
            limits:
              memory: 200Mi
            requests:
              memory: 100Mi
      containers:
        - name: main
          image: ghcr.io/headwaymaps/valhalla:${HEADWAY_CONTAINER_TAG}
          imagePullPolicy: Always
          ports:
          - containerPort: 8002
          volumeMounts:
          - name: valhalla-volume
            mountPath: /data
          resources:
            limits:
              memory: 8Gi
            requests:
              memory: ${VALHALLA_MEMORY_REQUEST}
      volumes:
        - name: valhalla-volume
          persistentVolumeClaim:
            claimName: valhalla-${HEADWAY_AREA_TAG_SAFE}-${HEADWAY_DATA_TAG_SAFE}-${VALHALLA_VOLUME_VERSION}
