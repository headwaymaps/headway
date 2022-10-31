apiVersion: v1
kind: Service
metadata:
  name: headway
spec:
  type: NodePort
  selector:
    app: headway
  ports:
    - name: http
      port: 8080
      targetPort: 8080
      nodePort: ${HEADWAY_SERVICE_PORT}
