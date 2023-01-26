apiVersion: apps/v1
kind: Deployment
metadata:
  name: ${OTP_ENDPOINT_NAME}
spec:
  selector:
    matchLabels:
      app: ${OTP_ENDPOINT_NAME}
  replicas: 1
  template:
    metadata:
      labels:
        app: ${OTP_ENDPOINT_NAME}
    spec:
      initContainers:
        - name: init
          image: ghcr.io/headwaymaps/opentripplanner-init:${HEADWAY_CONTAINER_TAG}
          imagePullPolicy: Always
          volumeMounts:
            - name: opentripplanner-volume
              mountPath: /data
          env:
            - name: OTP_ARTIFACT_URL
              valueFrom:
                configMapKeyRef:
                  name: deployment-config
                  key: otp-graph-urls.${TRANSIT_AREA}
          resources:
            limits:
              memory: 128Mi
            requests:
              memory: 128Mi
      containers:
        - name: main
          image: ghcr.io/headwaymaps/opentripplanner:${HEADWAY_CONTAINER_TAG}
          imagePullPolicy: Always
          ports:
            - containerPort: 8000
          volumeMounts:
            - name: opentripplanner-volume
              mountPath: /var/opentripplanner
          resources:
            limits:
              memory: 4.5Gi
            requests:
              memory: 500Mi
          livenessProbe:
            httpGet:
              path: /
              port: 8000
            initialDelaySeconds: 15
            periodSeconds: 15
            failureThreshold: 20
          readinessProbe:
            httpGet:
              path: /
              port: 8000
            initialDelaySeconds: 15
            periodSeconds: 15
            failureThreshold: 20
      volumes:
        - name: opentripplanner-volume
          ephemeral:
            volumeClaimTemplate:
              spec:
                accessModes: [ "ReadWriteOnce" ]
                resources:
                  requests:
                    storage: 1Gi
