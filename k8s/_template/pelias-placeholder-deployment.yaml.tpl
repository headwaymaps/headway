apiVersion: apps/v1
kind: Deployment
metadata:
  name: pelias-placeholder
spec:
  replicas: 1
  minReadySeconds: 10
  selector:
    matchLabels:
      app: pelias-placeholder
  template:
    metadata:
      labels:
        app: pelias-placeholder
      annotations:
    spec:
      initContainers:
        - name: init
          image: ghcr.io/headwaymaps/pelias-init:${HEADWAY_CONTAINER_TAG}
          imagePullPolicy: Always
          volumeMounts:
            - name: placeholder-volume
              mountPath: /data
            - name: config-volume
              mountPath: /config
          env:
            - name: PELIAS_CONFIG_JSON
              valueFrom:
                configMapKeyRef:
                  name: deployment-config
                  key: pelias-config-json
            - name: PLACEHOLDER_ARTIFACT_URL
              valueFrom:
                configMapKeyRef:
                  name: deployment-config
                  key: placeholder-artifact-url
            - name: HEADWAY_AREA
              valueFrom:
                configMapKeyRef:
                  name: deployment-config
                  key: area
          command: ["/bin/bash", "-c", "/app/init_config.sh && /app/init_placeholder.sh" ]
          resources:
            limits:
              memory: 200Mi
            requests:
              memory: 100Mi
      containers:
        - name: main
          image: pelias/placeholder:latest
          ports:
            - containerPort: 4100
          volumeMounts:
            - name: placeholder-volume
              mountPath: /data
            - name: config-volume
              mountPath: /config
          env:
            - name: PLACEHOLDER_DATA
              value: "/data/placeholder/"
            - name: PORT
              value: "4100"
          resources:
            limits:
              memory: 500Mi
            requests:
              memory: 200Mi
          livenessProbe:
            httpGet:
              path: /demo/
              port: 4100
            initialDelaySeconds: 15
            periodSeconds: 15
            failureThreshold: 10
          readinessProbe:
            httpGet:
              path: /demo/
              port: 4100
            initialDelaySeconds: 15
            periodSeconds: 15
            failureThreshold: 10
      volumes:
        - name: placeholder-volume
          ephemeral:
            volumeClaimTemplate:
              spec:
                accessModes: [ "ReadWriteOnce" ]
                resources:
                  requests:
                    storage: 40Gi
        - name: config-volume
          ephemeral:
            volumeClaimTemplate:
              spec:
                accessModes: [ "ReadWriteOnce" ]
                resources:
                  requests:
                    storage: 1Mi
