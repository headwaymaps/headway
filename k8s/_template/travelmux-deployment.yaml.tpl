apiVersion: apps/v1
kind: Deployment
metadata:
  name: travelmux
spec:
  selector:
    matchLabels:
      app: travelmux
  replicas: 1
  template:
    metadata:
      labels:
        app: travelmux
    spec:
      initContainers:
        - name: init
          image: ghcr.io/headwaymaps/travelmux-init:${HEADWAY_CONTAINER_TAG}
          imagePullPolicy: Always
          volumeMounts:
            - name: travelmux-volume
              mountPath: /data
          env:
            - name: TRAVELMUX_ELEVATION_ARTIFACT_URL
              valueFrom:
                configMapKeyRef:
                  name: deployment-config
                  key: elevation-artifact-url
          resources:
            limits:
              memory: 128Mi
            requests:
              memory: 64Mi
      containers:
        - name: main
          image: ghcr.io/headwaymaps/travelmux:${HEADWAY_CONTAINER_TAG}
          imagePullPolicy: Always
          args: ${OTP_ENDPOINTS_JSON}
          env:
            - name: ELEVATION_TIFS_DIR
              value: /data/elevation-tifs
          ports:
            - containerPort: 8000
          volumeMounts:
            - name: travelmux-volume
              mountPath: /data
          resources:
            limits:
              memory: 256Mi
            requests:
              memory: 128Mi
          livenessProbe:
            httpGet:
              path: /health/alive
              port: 8000
            initialDelaySeconds: 5
            periodSeconds: 10
            failureThreshold: 10
          readinessProbe:
            httpGet:
              path: /health/ready
              port: 8000
            initialDelaySeconds: 5
            periodSeconds: 10
            failureThreshold: 10
      volumes:
        - name: travelmux-volume
          ephemeral:
            volumeClaimTemplate:
              spec:
                accessModes: [ "ReadWriteOnce" ]
                resources:
                  requests:
                    storage: 1Gi
