apiVersion: v1
kind: Service
metadata:
  name: ${OTP_ENDPOINT_NAME}
spec:
  selector:
    app: ${OTP_ENDPOINT_NAME}
  ports:
    - protocol: TCP
      name: service
      port: 8000
