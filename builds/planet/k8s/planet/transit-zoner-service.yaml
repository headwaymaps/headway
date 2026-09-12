apiVersion: v1
kind: Service
metadata:
  name: transit-zoner
spec:
  selector:
    app: transit-zoner
  ports:
    - protocol: TCP
      name: http
      port: 8420
      targetPort: 8420
