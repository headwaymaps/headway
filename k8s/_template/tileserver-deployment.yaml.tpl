apiVersion: apps/v1
kind: Deployment
metadata:
  name: tileserver
spec:
  selector:
    matchLabels:
      app: tileserver
  replicas: 1
  strategy:
    type: RollingUpdate
    rollingUpdate:
      # Surge a new pod before retiring the old one. On a version bump the two
      # pods claim *different* PVCs, so the new one downloads its artifact while
      # the old one keeps serving. On a same-version restart they share one
      # ReadWriteOnce claim, which is allowed as long as they're on the same
      # node - hence the podAffinity below.
      maxSurge: 1
      maxUnavailable: 0
  template:
    metadata:
      labels:
        app: tileserver
    spec:
      affinity:
        podAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
            - weight: 100
              podAffinityTerm:
                labelSelector:
                  matchLabels:
                    app: tileserver
                topologyKey: kubernetes.io/hostname
      initContainers:
        - name: init
          image: ghcr.io/headwaymaps/tileserver-init:${HEADWAY_CONTAINER_TAG}
          imagePullPolicy: Always
          volumeMounts:
            - name: tileserver-volume
              mountPath: /data
          env:
            - name: TERRAIN_ARTIFACT_SOURCE
              valueFrom:
                configMapKeyRef:
                  name: deployment-config
                  key: terrain-source-url
            - name: TERRAIN_ARTIFACT_DEST
              value: /data/tiles/terrain.mbtiles
            - name: LANDCOVER_ARTIFACT_SOURCE
              valueFrom:
                configMapKeyRef:
                  name: deployment-config
                  key: landcover-source-url
            - name: LANDCOVER_ARTIFACT_DEST
              value: /data/tiles/landcover.mbtiles
            - name: AREAMAP_ARTIFACT_SOURCE
              valueFrom:
                configMapKeyRef:
                  name: deployment-config
                  key: areamap-source-url
            - name: AREAMAP_ARTIFACT_DEST
              value: /data/tiles/areamap.pmtiles
          resources:
            limits:
              memory: 200Mi
            requests:
              memory: 100Mi
      containers:
        - name: tileserver
          image: ghcr.io/headwaymaps/tileserver:${HEADWAY_CONTAINER_TAG}
          imagePullPolicy: Always
          ports:
            - containerPort: 8000
          volumeMounts:
            - name: tileserver-volume
              mountPath: /data
          env:
            - name: PORT
              value: "8000"
          resources:
            limits:
              memory: 1500Mi
            requests:
              memory: 250Mi
          livenessProbe:
            httpGet:
              path: /
              port: 8000
            initialDelaySeconds: 15
            periodSeconds: 15
            failureThreshold: 10
          readinessProbe:
            httpGet:
              path: /
              port: 8000
            initialDelaySeconds: 15
            periodSeconds: 15
            failureThreshold: 10
      volumes:
        - name: tileserver-volume
          persistentVolumeClaim:
            claimName: tileserver-${HEADWAY_AREA_TAG_SAFE}-${HEADWAY_DATA_TAG_SAFE}
