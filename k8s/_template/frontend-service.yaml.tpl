apiVersion: v1
kind: Service
metadata:
  name: frontend
spec:
  type: NodePort
  selector:
    app: frontend
  ports:
    - name: http
      port: 8080
      targetPort: 8080
      nodePort: ${HEADWAY_SERVICE_PORT}
