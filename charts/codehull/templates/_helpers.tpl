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
API_STORE_URL
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
- name: API_OIDC_ISSUER
  value: {{ .Values.oidc.issuer | quote }}
- name: API_OIDC_AUDIENCE
  value: {{ .Values.oidc.audience | quote }}
{{- end -}}

{{- define "codehull.seat" -}}
{{- if .Values.api.repo.enabled }}
- name: API_REPO_PATH
  value: {{ printf "%s/store" .Values.api.repo.mount | quote }}
{{- end }}
{{- end -}}

{{- define "codehull.store" -}}
- name: API_STORE_KIND
  value: "pg"
- name: API_STORE_URL
  valueFrom:
    secretKeyRef:
      name: {{ include "codehull.storeseat" . }}
      key: {{ include "codehull.storekey" . }}
{{- if .Values.blob.endpoint }}
- name: API_BLOB_ENDPOINT
  value: {{ .Values.blob.endpoint | quote }}
- name: API_BLOB_KEY
  value: {{ .Values.blob.key | quote }}
- name: API_BLOB_SECRET
  valueFrom:
    secretKeyRef:
      name: {{ .Values.blob.secretName }}
      key: {{ .Values.blob.secretKey }}
{{- end }}
{{- end -}}
