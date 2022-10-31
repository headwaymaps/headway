apiVersion: apps/v1
kind: Deployment
metadata:
  name: headway-deployment
spec:
  selector:
    matchLabels:
      app: headway
  replicas: 1
  template:
    metadata:
      labels:
        app: headway
    spec:
      volumes:
        - name: headway-volume
          ephemeral:
            volumeClaimTemplate:
              spec:
                accessModes: [ "ReadWriteOnce" ]
                resources:
                  requests:
                    storage: 50Mi
      initContainers:
        - name: headway-init
          image: ghcr.io/michaelkirk/headway-init:${HEADWAY_VERSION}
          imagePullPolicy: Always
          volumeMounts:
            - name: headway-volume
              mountPath: /data
          env:
            - name: FONT_ARTIFACT_DEST_PATH
              value: /data/fonts/fonts.tar
            - name: SPRITE_ARTIFACT_DEST_PATH
              value: /data/sprite/sprite.tar
            - name: FONT_ARTIFACT_URL
              valueFrom:
                configMapKeyRef:
                  name: deployment-config
                  key: font-source-url
            - name: SPRITE_ARTIFACT_URL
              valueFrom:
                configMapKeyRef:
                  name: deployment-config
                  key: sprite-source-url
          resources:
            limits:
              memory: 100Mi
            requests:
              memory: 100Mi
      containers:
        - name: headway
          image: ghcr.io/michaelkirk/headway:${HEADWAY_VERSION}
          imagePullPolicy: Always
          volumeMounts:
            - name: headway-volume
              mountPath: /data
          ports:
            - containerPort: 8080
          env:
            - name: HEADWAY_AREA
              valueFrom:
                configMapKeyRef:
                  name: deployment-config
                  key: area
            - name: HEADWAY_TILESERVER_URL
              value: http://mbtileserver:8000
            - name: HEADWAY_PELIAS_URL
              value: http://peliasapi:4000
            - name: HEADWAY_VALHALLA_URL
              value: http://valhalla:8002
            - name: HEADWAY_PUBLIC_URL
              valueFrom:
                configMapKeyRef:
                  name: deployment-config
                  key: public-url
          resources:
            limits:
              memory: 300Mi
            requests:
              memory: 200Mi
          livenessProbe:
            httpGet:
              path: /
              port: 8080
            initialDelaySeconds: 5
            periodSeconds: 5
            failureThreshold: 10
          readinessProbe:
            httpGet:
              path: /
              port: 8080
            initialDelaySeconds: 5
            periodSeconds: 5
            failureThreshold: 10
