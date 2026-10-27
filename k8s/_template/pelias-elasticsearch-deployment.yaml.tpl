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
          persistentVolumeClaim:
            claimName: elasticsearch-${HEADWAY_AREA_TAG_SAFE}-${HEADWAY_DATA_TAG_SAFE}-${ELASTICSEARCH_VOLUME_VERSION}
        # Elasticsearch's writes, layered over the read-only index image.
        - name: elasticsearch-scratch
          emptyDir: {}
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
              mountPath: /index
            - name: config-volume
              mountPath: /config
          env:
            - name: PELIAS_CONFIG_JSON
              value: ${PELIAS_CONFIG_JSON_ENV}
            - name: ELASTICSEARCH_ARTIFACT_URL
              value: "${ELASTICSEARCH_ARTIFACT_URL}"
          command: ["/bin/bash", "-c", "/app/init_config.sh && /app/init_elastic.sh" ]
          resources:
            limits:
              memory: 1Gi
            requests:
              memory: 200Mi
      containers:
        - name: main
          image: ghcr.io/headwaymaps/pelias-elasticsearch:${HEADWAY_CONTAINER_TAG}
          imagePullPolicy: Always
          # to mount the index image
          securityContext:
            privileged: true
          volumeMounts:
            - name: elasticsearch-volume
              mountPath: /index
              readOnly: true
            - name: elasticsearch-scratch
              mountPath: /scratch
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
