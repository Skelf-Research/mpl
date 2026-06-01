---
title: Kubernetes & Helm
description: Deploy the MPL proxy to Kubernetes using the official Helm chart with production-grade configuration
---

# Kubernetes & Helm

This guide covers deploying the MPL proxy to Kubernetes using the official Helm chart located at `helm/mpl-proxy/`. The chart supports standalone deployment, sidecar injection, autoscaling, observability, and network policies.

---

## Prerequisites

- Kubernetes 1.24+
- Helm 3.10+
- `kubectl` configured for your cluster
- (Optional) Prometheus Operator for ServiceMonitor support

---

## Helm Chart Overview

```
helm/mpl-proxy/
├── Chart.yaml              # Chart metadata (v0.1.0)
├── values.yaml             # Default configuration values
└── templates/
    ├── _helpers.tpl        # Template helpers
    ├── configmap.yaml      # MPL config and registry
    ├── deployment.yaml     # Proxy deployment
    ├── hpa.yaml            # Horizontal Pod Autoscaler
    ├── service.yaml        # ClusterIP service
    ├── serviceaccount.yaml # RBAC service account
    └── servicemonitor.yaml # Prometheus ServiceMonitor
```

---

## Installation

### Basic Install

```bash
helm install mpl-proxy ./helm/mpl-proxy \
  --set transport.upstream=http://mcp-server:8080
```

### Production Install

```bash
helm install mpl-proxy ./helm/mpl-proxy \
  --set mpl.upstream=http://mcp-server:8080 \
  --set mpl.mode=strict \
  --set mpl.requiredProfile=qom-strict-argcheck \
  --set autoscaling.enabled=true \
  --set serviceMonitor.enabled=true \
  --set networkPolicy.enabled=true
```

### Install with Custom Values File

```bash
helm install mpl-proxy ./helm/mpl-proxy \
  -f my-values.yaml
```

### Upgrade

```bash
helm upgrade mpl-proxy ./helm/mpl-proxy \
  -f my-values.yaml
```

### Uninstall

```bash
helm uninstall mpl-proxy
```

---

## Values Reference

### Image Configuration

```yaml
image:
  repository: ghcr.io/anthropics/mpl-proxy
  tag: ""           # Defaults to Chart.appVersion (0.1.0)
  pullPolicy: IfNotPresent

imagePullSecrets: []
```

### Replicas

```yaml
replicaCount: 1
```

### MPL Configuration

```yaml
mpl:
  # Proxy mode: transparent (log only) or strict (block invalid)
  mode: transparent

  # Registry URL or path for SType schemas
  registry: "https://github.com/anthropics/mpl/raw/main/registry"

  # Required QoM profile for validation
  requiredProfile: qom-basic

  # Enable schema validation
  enforceSchema: true

  # Enable assertion checks
  enforceAssertions: true

  # Enable the policy engine
  policyEngine: false
```

### Transport

```yaml
transport:
  # Listen port for proxy traffic
  port: 9443

  # Upstream MCP/A2A server address
  upstream: ""

  # Protocol: http, websocket, grpc
  protocol: http
```

### Observability

```yaml
observability:
  # Prometheus metrics port
  metricsPort: 9100

  # Log level: trace, debug, info, warn, error
  logLevel: info

  # Log format: json, text
  logFormat: json
```

### Service

```yaml
service:
  type: ClusterIP
  port: 9443
  metricsPort: 9100
```

### Ingress

```yaml
ingress:
  enabled: false
  className: ""
  annotations: {}
  hosts:
    - host: mpl-proxy.local
      paths:
        - path: /
          pathType: ImplementationSpecific
  tls: []
```

??? example "Ingress with TLS"
    ```yaml
    ingress:
      enabled: true
      className: nginx
      annotations:
        cert-manager.io/cluster-issuer: letsencrypt-prod
      hosts:
        - host: mpl.example.com
          paths:
            - path: /
              pathType: Prefix
      tls:
        - secretName: mpl-tls
          hosts:
            - mpl.example.com
    ```

### Resources

```yaml
resources:
  limits:
    cpu: 500m
    memory: 256Mi
  requests:
    cpu: 100m
    memory: 128Mi
```

### Autoscaling (HPA)

```yaml
autoscaling:
  enabled: false
  minReplicas: 1
  maxReplicas: 10
  targetCPUUtilizationPercentage: 80
  targetMemoryUtilizationPercentage: 80
```

### Health Probes

```yaml
probes:
  liveness:
    enabled: true
    initialDelaySeconds: 10
    periodSeconds: 10
    timeoutSeconds: 3
    failureThreshold: 3
  readiness:
    enabled: true
    initialDelaySeconds: 5
    periodSeconds: 5
    timeoutSeconds: 3
    failureThreshold: 3
```

### ServiceMonitor (Prometheus Operator)

```yaml
serviceMonitor:
  enabled: false
  namespace: ""
  interval: 30s
  scrapeTimeout: 10s
  labels: {}
```

### Network Policy

```yaml
networkPolicy:
  enabled: false
  ingress: []
  egress: []
```

---

## Sidecar Pattern

The most common production pattern is injecting the MPL proxy as a sidecar container alongside your existing MCP/A2A server pod. This ensures all traffic is validated without changing your application code.

### Sidecar Deployment Example

```yaml title="sidecar-deployment.yaml"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: mcp-server-with-mpl
spec:
  replicas: 3
  selector:
    matchLabels:
      app: mcp-server
  template:
    metadata:
      labels:
        app: mcp-server
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "9100"
    spec:
      containers:
        # Your existing MCP server
        - name: mcp-server
          image: your-org/mcp-server:latest
          ports:
            - containerPort: 8080

        # MPL sidecar proxy
        - name: mpl-proxy
          image: ghcr.io/anthropics/mpl-proxy:0.1.0
          args: ["--config", "/config/mpl-config.yaml"]
          ports:
            - name: proxy
              containerPort: 9443
            - name: metrics
              containerPort: 9100
          volumeMounts:
            - name: mpl-config
              mountPath: /config
              readOnly: true
          resources:
            limits:
              cpu: 500m
              memory: 256Mi
            requests:
              cpu: 100m
              memory: 128Mi
          livenessProbe:
            httpGet:
              path: /health
              port: proxy
            initialDelaySeconds: 10
          readinessProbe:
            httpGet:
              path: /health
              port: proxy
            initialDelaySeconds: 5

      volumes:
        - name: mpl-config
          configMap:
            name: mpl-sidecar-config
```

### Sidecar ConfigMap

```yaml title="sidecar-configmap.yaml"
apiVersion: v1
kind: ConfigMap
metadata:
  name: mpl-sidecar-config
data:
  mpl-config.yaml: |
    transport:
      listen: "0.0.0.0:9443"
      upstream: "localhost:8080"   # Same pod, via localhost
      protocol: http
    mpl:
      registry: "https://github.com/anthropics/mpl/raw/main/registry"
      mode: strict
      required_profile: qom-strict-argcheck
      enforce_schema: true
      enforce_assertions: true
    observability:
      metrics_port: 9100
      log_format: json
      log_level: info
```

!!! tip "Sidecar Traffic Flow"
    External traffic hits the MPL sidecar on port 9443. After validation, the proxy forwards to the MCP server on `localhost:8080` within the same pod. This pattern requires updating your Service to target port 9443 instead of 8080.

---

## ServiceMonitor for Prometheus Operator

When using the Prometheus Operator, enable the ServiceMonitor to auto-discover MPL metrics:

```yaml title="values.yaml (excerpt)"
serviceMonitor:
  enabled: true
  namespace: monitoring     # Namespace where Prometheus runs
  interval: 30s
  scrapeTimeout: 10s
  labels:
    release: prometheus     # Match your Prometheus selector
```

This creates a ServiceMonitor resource:

```yaml
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: mpl-proxy
  labels:
    release: prometheus
spec:
  endpoints:
    - port: metrics
      interval: 30s
      scrapeTimeout: 10s
      path: /metrics
  selector:
    matchLabels:
      app.kubernetes.io/name: mpl-proxy
```

---

## Network Policies

Restrict traffic to only expected sources and destinations:

```yaml title="values.yaml (excerpt)"
networkPolicy:
  enabled: true
  ingress:
    - from:
        - podSelector:
            matchLabels:
              app: ai-agent
      ports:
        - port: 9443
          protocol: TCP
    - from:
        - namespaceSelector:
            matchLabels:
              name: monitoring
      ports:
        - port: 9100
          protocol: TCP
  egress:
    - to:
        - podSelector:
            matchLabels:
              app: mcp-server
      ports:
        - port: 8080
          protocol: TCP
```

!!! warning "DNS Access"
    If network policies are enabled, ensure egress to `kube-dns` (port 53) is allowed, or the proxy will fail to resolve upstream hostnames.

---

## Pod Disruption Budget

For high-availability deployments, add a PodDisruptionBudget to prevent all replicas from being evicted simultaneously:

```yaml title="pdb.yaml"
apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: mpl-proxy-pdb
spec:
  minAvailable: 1
  selector:
    matchLabels:
      app.kubernetes.io/name: mpl-proxy
```

!!! info "Recommendation"
    With `autoscaling.minReplicas: 2` or higher, set `minAvailable: 1` to ensure at least one proxy remains running during node drains and cluster upgrades.

---

## ConfigMap for Registry and Config

The Helm chart automatically creates a ConfigMap from your `mpl-config.yaml`. To also bundle the registry schemas:

```yaml title="configmap-with-registry.yaml"
apiVersion: v1
kind: ConfigMap
metadata:
  name: mpl-proxy-config
data:
  mpl-config.yaml: |
    transport:
      listen: "0.0.0.0:9443"
      upstream: "mcp-server:8080"
      protocol: http
    mpl:
      registry: "/config/registry"
      mode: strict
      required_profile: qom-strict-argcheck
      enforce_schema: true
      enforce_assertions: true
    observability:
      metrics_port: 9100
      log_format: json
      log_level: info
---
apiVersion: v1
kind: ConfigMap
metadata:
  name: mpl-proxy-registry
data:
  calendar-event.schema.json: |
    { "$schema": "https://json-schema.org/draft/2020-12/schema", ... }
  finance-transaction.schema.json: |
    { "$schema": "https://json-schema.org/draft/2020-12/schema", ... }
```

Mount both in the deployment:

```yaml
volumeMounts:
  - name: config
    mountPath: /config/mpl-config.yaml
    subPath: mpl-config.yaml
  - name: registry
    mountPath: /config/registry
    readOnly: true
volumes:
  - name: config
    configMap:
      name: mpl-proxy-config
  - name: registry
    configMap:
      name: mpl-proxy-registry
```

---

## Security Context

The chart applies a restrictive security context by default:

```yaml
podSecurityContext:
  fsGroup: 1000

securityContext:
  capabilities:
    drop:
      - ALL
  readOnlyRootFilesystem: true
  runAsNonRoot: true
  runAsUser: 1000
```

!!! success "Security Defaults"
    - Runs as non-root user (UID 1000)
    - Drops all Linux capabilities
    - Read-only root filesystem
    - Writable `/tmp` via emptyDir volume

---

## Example: Full Production Values

```yaml title="production-values.yaml"
image:
  repository: ghcr.io/anthropics/mpl-proxy
  tag: "0.1.0"

replicaCount: 3

mpl:
  mode: strict
  requiredProfile: qom-strict-argcheck
  enforceSchema: true
  enforceAssertions: true
  policyEngine: true

transport:
  port: 9443
  upstream: http://mcp-server.default.svc.cluster.local:8080
  protocol: http

observability:
  metricsPort: 9100
  logLevel: info
  logFormat: json

resources:
  limits:
    cpu: "1"
    memory: 512Mi
  requests:
    cpu: 250m
    memory: 256Mi

autoscaling:
  enabled: true
  minReplicas: 2
  maxReplicas: 10
  targetCPUUtilizationPercentage: 70

probes:
  liveness:
    enabled: true
    initialDelaySeconds: 10
    periodSeconds: 10
  readiness:
    enabled: true
    initialDelaySeconds: 5
    periodSeconds: 5

serviceMonitor:
  enabled: true
  interval: 15s
  labels:
    release: prometheus

networkPolicy:
  enabled: true
  ingress:
    - from:
        - podSelector:
            matchLabels:
              role: ai-agent
      ports:
        - port: 9443
  egress:
    - to:
        - podSelector:
            matchLabels:
              app: mcp-server
      ports:
        - port: 8080

ingress:
  enabled: true
  className: nginx
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
  hosts:
    - host: mpl.example.com
      paths:
        - path: /
          pathType: Prefix
  tls:
    - secretName: mpl-tls
      hosts:
        - mpl.example.com
```

Deploy:

```bash
helm install mpl-proxy ./helm/mpl-proxy -f production-values.yaml
```

---

## Verifying the Deployment

```bash
# Check pods are running
kubectl get pods -l app.kubernetes.io/name=mpl-proxy

# Check the service
kubectl get svc mpl-proxy

# View proxy logs
kubectl logs -l app.kubernetes.io/name=mpl-proxy -f

# Test health endpoint
kubectl port-forward svc/mpl-proxy 9443:9443
curl http://localhost:9443/health

# Check metrics
kubectl port-forward svc/mpl-proxy 9100:9100
curl http://localhost:9100/metrics
```

---

## Next Steps

- [Production Checklist](production-checklist.md) -- Verify all requirements before going live
- [Monitoring & Metrics](../guides/operations/monitoring.md) -- Configure dashboards and alerts
- [Troubleshooting](../guides/operations/troubleshooting.md) -- Debug common Kubernetes issues
