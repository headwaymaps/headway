apiVersion: apps/v1
kind: Deployment
metadata:
  name: valhalla
spec:
  selector:
    matchLabels:
      app: valhalla
  replicas: 1
  template:
    metadata:
      labels:
        app: valhalla
    spec:
      initContainers:
        - name: init
          image: ghcr.io/headwaymaps/valhalla-init:${HEADWAY_CONTAINER_VERSION}
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
          image: ghcr.io/headwaymaps/valhalla:${HEADWAY_CONTAINER_VERSION}
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
          ephemeral:
            volumeClaimTemplate:
              spec:
                accessModes: [ "ReadWriteOnce" ]
                resources:
                  requests:
                    storage: 200Gi
