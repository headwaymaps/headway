apiVersion: v1
kind: Service
metadata:
  name: opentripplanner
spec:
  selector:
    app: opentripplanner
  ports:
    - protocol: TCP
      name: service
      port: 8080
      targetPort: 8080
