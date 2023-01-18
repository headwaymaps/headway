apiVersion: apps/v1
kind: Deployment
metadata:
  name: pelias-libpostal
spec:
  replicas: 1
  minReadySeconds: 10
  selector:
    matchLabels:
      app: pelias-libpostal
  template:
    metadata:
      labels:
        app: pelias-libpostal
    spec:
      containers:
        - name: main
          image: pelias/libpostal-service:latest
          ports:
            - containerPort: 4400
          resources:
            limits:
              memory: 3Gi
            requests:
              memory: 2Gi
          livenessProbe:
            httpGet:
              path: /parse?address=readiness
              port: 4400
            initialDelaySeconds: 15
            periodSeconds: 15
            failureThreshold: 10
          readinessProbe:
            httpGet:
              path: /parse?address=readiness
              port: 4400
            initialDelaySeconds: 15
            periodSeconds: 15
            failureThreshold: 10
