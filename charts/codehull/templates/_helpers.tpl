{{- define "codehull.storeseat" -}}
{{- if .Values.postgres.urlSecret.name -}}
{{ .Values.postgres.urlSecret.name }}
{{- else -}}
{{ .Release.Name }}-store
{{- end -}}
{{- end -}}

{{- define "codehull.storekey" -}}
{{- if .Values.postgres.urlSecret.name -}}
{{ .Values.postgres.urlSecret.key }}
{{- else -}}
CODEHULL_STORE_URL
{{- end -}}
{{- end -}}

{{- define "codehull.pgpass" -}}
{{- $held := lookup "v1" "Secret" .Release.Namespace (printf "%s-store" .Release.Name) -}}
{{- if and $held $held.data.PGPASSWORD -}}
{{ $held.data.PGPASSWORD | b64dec }}
{{- else if .Values.postgres.password -}}
{{ .Values.postgres.password }}
{{- else -}}
{{ randAlphaNum 32 }}
{{- end -}}
{{- end -}}

{{- define "codehull.seam" -}}
- name: CODEHULL_OIDC_ISSUER
  value: {{ .Values.oidc.issuer | quote }}
- name: CODEHULL_OIDC_AUDIENCE
  value: {{ .Values.oidc.audience | quote }}
{{- end -}}

{{- define "codehull.seat" -}}
{{- if .Values.api.repo.enabled }}
- name: CODEHULL_REPO_PATH
  value: {{ printf "%s/store" .Values.api.repo.mount | quote }}
{{- end }}
{{- end -}}

{{- define "codehull.store" -}}
- name: CODEHULL_STORE_KIND
  value: "pg"
- name: CODEHULL_STORE_URL
  valueFrom:
    secretKeyRef:
      name: {{ include "codehull.storeseat" . }}
      key: {{ include "codehull.storekey" . }}
{{- if .Values.blob.endpoint }}
- name: CODEHULL_BLOB_ENDPOINT
  value: {{ .Values.blob.endpoint | quote }}
- name: CODEHULL_BLOB_KEY
  value: {{ .Values.blob.key | quote }}
- name: CODEHULL_BLOB_SECRET
  valueFrom:
    secretKeyRef:
      name: {{ .Values.blob.secretName }}
      key: {{ .Values.blob.secretKey }}
{{- end }}
{{- end -}}
