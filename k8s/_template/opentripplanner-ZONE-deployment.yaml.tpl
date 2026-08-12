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
      # the old one keeps serving. On a same-version restart they share one
      # ReadWriteOnce claim, which is allowed as long as they're on the same
      # node - hence the podAffinity below.
      maxSurge: 1
      maxUnavailable: 0
  template:
    metadata:
      labels:
        app: ${OTP_ENDPOINT_NAME}
    spec:
      affinity:
        podAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
            - weight: 100
              podAffinityTerm:
                labelSelector:
                  matchLabels:
                    app: ${OTP_ENDPOINT_NAME}
                topologyKey: kubernetes.io/hostname
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
            claimName: opentripplanner-${TRANSIT_ZONE}-${HEADWAY_AREA_TAG_SAFE}-${HEADWAY_DATA_TAG_SAFE}-${OTP_GRAPH_DATE}
