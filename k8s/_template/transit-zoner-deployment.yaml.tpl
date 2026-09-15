apiVersion: apps/v1
kind: Deployment
metadata:
  name: transit-zoner
spec:
  selector:
    matchLabels:
      app: transit-zoner
  replicas: 1
  template:
    metadata:
      labels:
        app: transit-zoner
    spec:
      containers:
        - name: main
          image: ghcr.io/headwaymaps/transit-zoner:${HEADWAY_CONTAINER_TAG}
          imagePullPolicy: Always
          ports:
            - containerPort: 8420
          readinessProbe:
            httpGet:
              path: /health/ready
              port: 8420
            initialDelaySeconds: 3
            periodSeconds: 10
