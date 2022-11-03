apiVersion: v1
kind: Service
metadata:
  name: otp
spec:
  selector:
    app: opentripplanner
  ports:
    - protocol: TCP
      name: service
      port: 8080
      targetPort: 8080
    - protocol: TCP
      name: havegraph
      port: 9999
      targetPort: 9999