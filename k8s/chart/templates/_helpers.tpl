{{/* A tag made safe for a resource name. */}}
{{- define "headway.safe" -}}
{{- . | lower | replace "." "-" | replace "_" "-" -}}
{{- end -}}

{{/* An area artifact's URL: (list $ "<key in artifacts.json>") */}}
{{- define "headway.artifact" -}}
{{- $root := index . 0 -}}
{{- $file := index $root.Values.artifacts (index . 1) | required (printf "missing %s in artifacts.json; run bin/artifacts --pin on the build machine" (index . 1)) -}}
{{- printf "%s/%s/%s/%s" (required "artifactRoot must be set" $root.Values.artifactRoot) $root.Values.dataTag $root.Values.areaTag $file -}}
{{- end -}}

{{/* The URL of an artifact shared across areas: (list $ "<key in artifacts.json's shared>") */}}
{{- define "headway.sharedArtifact" -}}
{{- $root := index . 0 -}}
{{- $file := index $root.Values.artifacts.shared (index . 1) | required (printf "missing shared.%s in artifacts.json; run bin/artifacts --pin on the build machine" (index . 1)) -}}
{{- printf "%s/%s/%s" (required "artifactRoot must be set" $root.Values.artifactRoot) $root.Values.dataTag $file -}}
{{- end -}}

{{/* A headway image: (list $ "<name>") */}}
{{- define "headway.image" -}}
ghcr.io/headwaymaps/{{ index . 1 }}:{{ (index . 0).Values.containerTag }}
{{- end -}}

{{/*
A data volume's claim name: (list $ "<prefix>" "<artifact url>"...)

It ends with the version of the data the volume holds, as `<date>-<hash>`, from
the names of the artifacts unpacked into it: the newest date, and a hash over
the exact set. A claim named with it changes whenever its contents do -
otherwise the init containers, which short-circuit on a volume that already has
data, would serve the old build forever.
*/}}
{{- define "headway.claim" -}}
{{- $root := index . 0 -}}
{{- $names := list -}}
{{- range slice . 2 }}{{ $names = append $names (base .) }}{{ end -}}
{{- $names = join "\n" (sortAlpha $names) -}}
{{- $date := regexFindAll "[0-9]{4}-[0-9]{2}-[0-9]{2}" $names -1 | sortAlpha | last | required (printf "no <date> in artifact name(s): %s" $names) -}}
{{- $hash := printf "%s\n" $names | sha1sum | trunc 8 -}}
{{- printf "%s-%s-%s-%s-%s" (index . 1) (include "headway.safe" $root.Values.areaTag) (include "headway.safe" $root.Values.dataTag) $date $hash -}}
{{- end -}}

{{/* A data volume's claim, kept after it's superseded so a rollback is instant: (list "<name>" "<size>") */}}
{{- define "headway.pvc" -}}
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: {{ index . 0 }}
  labels:
    app.kubernetes.io/part-of: headway
  annotations:
    helm.sh/resource-policy: keep
spec:
  accessModes: [ "ReadWriteOnce" ]
  resources:
    requests:
      storage: {{ index . 1 }}
{{- end -}}

{{/* A ClusterIP service exposing one port: (list "<app>" <port>) */}}
{{- define "headway.service" -}}
apiVersion: v1
kind: Service
metadata:
  name: {{ index . 0 }}
spec:
  selector:
    app: {{ index . 0 }}
  ports:
    - protocol: TCP
      name: http
      port: {{ index . 1 }}
      targetPort: {{ index . 1 }}
{{- end -}}

{{- define "headway.configVolume" -}}
- name: config-volume
  ephemeral:
    volumeClaimTemplate:
      spec:
        accessModes: [ "ReadWriteOnce" ]
        resources:
          requests:
            storage: 1Mi
{{- end -}}
