apiVersion: apps/v1
kind: Deployment
metadata:
  name: frontend
spec:
  selector:
    matchLabels:
      app: frontend
  replicas: 1
  template:
    metadata:
      labels:
        app: frontend
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
        - name: init
          image: ghcr.io/headwaymaps/headway-init:${HEADWAY_CONTAINER_TAG}
          imagePullPolicy: Always
          volumeMounts:
            - name: headway-volume
              mountPath: /data
          env:
            - name: HEADWAY_BBOX
              valueFrom:
                configMapKeyRef:
                  name: deployment-config
                  key: bbox
            - name: HEADWAY_ENABLE_TRANSIT_ROUTING
              valueFrom:
                configMapKeyRef:
                  name: deployment-config
                  key: enable-transit-routing
            - name: HEADWAY_ABOUT_URL
              valueFrom:
                configMapKeyRef:
                  name: deployment-config
                  key: www-about-url
            - name: HEADWAY_ABOUT_LINK_TEXT
              valueFrom:
                configMapKeyRef:
                  name: deployment-config
                  key: www-about-link-text
            - name: HEADWAY_CONTACT_URL
              valueFrom:
                configMapKeyRef:
                  name: deployment-config
                  key: www-contact-url
            - name: HEADWAY_CONTACT_LINK_TEXT
              valueFrom:
                configMapKeyRef:
                  name: deployment-config
                  key: www-contact-link-text
          resources:
            limits:
              memory: 100Mi
            requests:
              memory: 100Mi
      containers:
        - name: main
          image: ghcr.io/headwaymaps/headway:${HEADWAY_CONTAINER_TAG}
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
              value: http://tileserver:8000
            - name: HEADWAY_PELIAS_URL
              value: http://pelias-api:4000
            - name: HEADWAY_PUBLIC_URL
              valueFrom:
                configMapKeyRef:
                  name: deployment-config
                  key: public-url
            - name: HEADWAY_ENABLE_TRANSIT_ROUTING
              valueFrom:
                configMapKeyRef:
                  name: deployment-config
                  key: enable-transit-routing
          resources:
            limits:
              memory: 300Mi
            requests:
              memory: 100Mi
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
