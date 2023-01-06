apiVersion: apps/v1
kind: Deployment
metadata:
  name: transitmux
spec:
  selector:
    matchLabels:
      app: transitmux
  replicas: 1
  template:
    metadata:
      labels:
        app: transitmux
    spec:
      containers:
        - name: main
          image: ghcr.io/headwaymaps/transitmux:${HEADWAY_CONTAINER_VERSION}
          imagePullPolicy: Always
          args: ${OTP_ENDPOINTS_JSON}
          ports:
            - containerPort: 8000
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
