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
      annotations:
        # Roll the pod if the zone file changes
        headway/zone-checksum: "${OTP_ZONE_CHECKSUM}"
    spec:
      initContainers:
        - name: init
          image: ghcr.io/headwaymaps/opentripplanner-init:${HEADWAY_CONTAINER_TAG}
          imagePullPolicy: Always
          volumeMounts:
            - name: opentripplanner-volume
              mountPath: /data
            - name: zone
              mountPath: /run/config
              readOnly: true
            - name: gtfs-secrets
              mountPath: /run/secrets
              readOnly: true
          env:
            - name: OTP_ARTIFACT_URL
              value: "${OTP_GRAPH_URL}"
          resources:
            limits:
              memory: 128Mi
            requests:
              memory: 128Mi
      containers:
        - name: main
          image: ghcr.io/headwaymaps/opentripplanner:${HEADWAY_CONTAINER_TAG}
          env:
            - name: "JAVA_OPTS"
              # keep this in sync to be just under the resources.limits.memory
              value: "-Xmx5G"
          imagePullPolicy: Always
          ports:
            - containerPort: 8000
          volumeMounts:
            - name: opentripplanner-volume
              mountPath: /var/opentripplanner
          resources:
            limits:
              # keep this in sync to be just above env.JAVA_OPTS.-Xmx
              memory: 5.25Gi
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
        - name: zone
          configMap:
            name: otp-${TRANSIT_ZONE}-zone
        - name: gtfs-secrets
          secret:
            secretName: otp-${TRANSIT_ZONE}-gtfs-secrets
            optional: ${OTP_GTFS_SECRET_OPTIONAL}
        - name: opentripplanner-volume
          persistentVolumeClaim:
            claimName: opentripplanner-${TRANSIT_ZONE}-${HEADWAY_AREA_TAG_SAFE}-${HEADWAY_DATA_TAG_SAFE}-${OTP_VOLUME_VERSION}
