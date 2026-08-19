apiVersion: apps/v1
kind: Deployment
metadata:
  name: ${OTP_ENDPOINT_NAME}
spec:
  selector:
    matchLabels:
      app: ${OTP_ENDPOINT_NAME}
  replicas: 1
  strategy:
    type: RollingUpdate
    rollingUpdate:
      # Surge a new pod before retiring the old one. On a version bump the two
      # pods claim *different* PVCs, so the new one downloads its artifact while
      # the old one keeps serving. On a same-version restart the two pods share
      # one ReadWriteOnce claim, which is fine: they only read it, and the
      # scheduler will only place the new pod where that volume can attach.
      maxSurge: 1
      maxUnavailable: 0
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
                  name: otp-${TRANSIT_ZONE}-config
                  key: graph-url
            - name: OTP_ROUTER_CONFIG_JSON
              valueFrom:
                configMapKeyRef:
                  name: otp-${TRANSIT_ZONE}-config
                  key: router-config-json
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
        - name: opentripplanner-volume
          persistentVolumeClaim:
            claimName: opentripplanner-${TRANSIT_ZONE}-${HEADWAY_AREA_TAG_SAFE}-${HEADWAY_DATA_TAG_SAFE}-${OTP_VOLUME_VERSION}
