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
              value: "${OTP_GRAPH_URL}"
            - name: OTP_ROUTER_CONFIG_JSON
              value: ${OTP_ROUTER_CONFIG_JSON_ENV}
          resources:
            limits:
              memory: 128Mi
            requests:
              memory: 128Mi
      containers:
        - name: main
          image: ghcr.io/headwaymaps/opentripplanner:${HEADWAY_CONTAINER_TAG}
          # API keys for authenticated GTFS-RT feeds. router-config.json refers
          # to them as ${HEADWAY_GTFS_API_KEY_*}, which OTP substitutes from its
          # environment before parsing - so the config stays committable and
          # the credential lives only here.
          #
          # Optional: a zone with no authenticated realtime feeds needs no
          # secret, and shouldn't fail to start for want of one. Create it with
          # a key per variable named in the query-gtfs-index output, e.g.
          #   kubectl create secret generic otp-${TRANSIT_AREA}-gtfs-api-keys \
          #     --from-literal=HEADWAY_GTFS_API_KEY_F_SF_BAY_AREA_RG_RT=...
          envFrom:
            - secretRef:
                name: otp-${TRANSIT_AREA}-gtfs-api-keys
                optional: true
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
        - name: opentripplanner-volume
          persistentVolumeClaim:
            claimName: opentripplanner-${TRANSIT_ZONE}-${HEADWAY_AREA_TAG_SAFE}-${HEADWAY_DATA_TAG_SAFE}-${OTP_VOLUME_VERSION}
