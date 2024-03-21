apiVersion: apps/v1
kind: Deployment
metadata:
  name: pelias-elasticsearch
spec:
  replicas: 1
  minReadySeconds: 10
  selector:
    matchLabels:
      app: pelias-elasticsearch
  template:
    metadata:
      labels:
        app: pelias-elasticsearch
    spec:
      volumes:
        - name: elasticsearch-volume
          ephemeral:
            volumeClaimTemplate:
              spec:
                accessModes: [ "ReadWriteOnce" ]
                resources:
                  requests:
                    storage: 90Gi
        - name: config-volume
          ephemeral:
            volumeClaimTemplate:
              spec:
                accessModes: [ "ReadWriteOnce" ]
                resources:
                  requests:
                    storage: 1Mi
      initContainers:
        - name: init
          image: ghcr.io/headwaymaps/pelias-init:${HEADWAY_CONTAINER_TAG}
          imagePullPolicy: Always
          volumeMounts:
            - name: elasticsearch-volume
              mountPath: /usr/share/elasticsearch/data
            - name: config-volume
              mountPath: /config
          env:
            - name: PELIAS_CONFIG_JSON
              valueFrom:
                configMapKeyRef:
                  name: deployment-config
                  key: pelias-config-json
            - name: ELASTICSEARCH_ARTIFACT_URL
              valueFrom:
                configMapKeyRef:
                  name: deployment-config
                  key: elasticsearch-artifact-url
            - name: HEADWAY_AREA
              valueFrom:
                configMapKeyRef:
                  name: deployment-config
                  key: area
          command: ["/bin/bash", "-c", "/app/init_config.sh && /app/init_elastic.sh" ]
          resources:
            limits:
              memory: 100Mi
            requests:
              memory: 100Mi
      containers:
        - name: main
          image: pelias/elasticsearch:8.12.2-beta
          volumeMounts:
            - name: elasticsearch-volume
              mountPath: /usr/share/elasticsearch/data
            - name: config-volume
              mountPath: /config
          ports:
            - containerPort: 9200
          env:
            - name: ES_JAVA_OPTS
              value: "-Xmx8g"
          resources:
            limits:
              memory: 16Gi
            requests:
              memory: ${PELIAS_ELASTICSEARCH_MEMORY_REQUEST}
          livenessProbe:
            httpGet:
              path: /
              port: 9200
            initialDelaySeconds: 15
            periodSeconds: 15
            failureThreshold: 50
          readinessProbe:
            httpGet:
              path: /_cluster/health?wait_for_status=yellow
              port: 9200
            initialDelaySeconds: 15
            periodSeconds: 15
            failureThreshold: 50
