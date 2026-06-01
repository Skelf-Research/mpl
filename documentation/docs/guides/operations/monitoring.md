---
title: Monitoring & Metrics
description: Production monitoring guide for MPL proxy with Prometheus metrics, Grafana dashboards, and alerting
---

# Monitoring & Metrics

MPL exposes comprehensive Prometheus metrics for monitoring proxy health, validation rates, QoM scores, and latency. This guide covers metric collection, alerting, and dashboard setup for production deployments.

---

## Metrics Endpoint

The MPL proxy exposes Prometheus-format metrics on port **9100** by default:

```bash
# Verify metrics are available
curl http://localhost:9100/metrics
```

Configure the metrics endpoint in `mpl-config.yaml`:

```yaml
# mpl-config.yaml
metrics:
  enabled: true
  listen: "0.0.0.0:9100"
  path: "/metrics"
```

---

## Available Metrics

### Request Metrics

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `mpl_requests_total` | Counter | `stype`, `method`, `status` | Total requests processed by the proxy |
| `mpl_unknown_stype_total` | Counter | -- | Requests with no SType mapping |
| `mpl_downgrade_total` | Counter | `reason` | Negotiation downgrades during AI-ALPN |

### Validation Metrics

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `mpl_validation_errors_total` | Counter | `stype`, `error_code` | Schema validation failures |

### QoM Metrics

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `mpl_qom_score` | Histogram | `stype`, `metric` | QoM score distribution per metric |
| `mpl_qom_breaches_total` | Counter | `stype`, `profile`, `metric` | QoM profile threshold violations |

### Latency Metrics

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `mpl_handshake_duration_seconds` | Histogram | -- | AI-ALPN handshake duration |
| `mpl_proxy_latency_seconds` | Histogram | `stype` | Total proxy overhead (validation + QoM + hashing) |

### Label Values

| Label | Possible Values | Example |
|-------|----------------|---------|
| `stype` | Any registered SType | `org.calendar.Event.v1` |
| `method` | `tools/call`, `tools/list`, `a2a/task` | `tools/call` |
| `status` | `success`, `validation_error`, `qom_breach`, `upstream_error` | `success` |
| `error_code` | `E-SCHEMA-FIDELITY`, `E-MISSING-FIELD`, `E-ADDITIONAL-PROP`, `E-TYPE-MISMATCH` | `E-MISSING-FIELD` |
| `metric` | `schema_fidelity`, `instruction_compliance`, `context_grounding`, `semantic_coherence`, `provenance_completeness`, `assertion_pass_rate` | `schema_fidelity` |
| `profile` | `qom-basic`, `qom-strict-argcheck`, `qom-comprehensive` | `qom-strict-argcheck` |
| `reason` | `stype_unsupported`, `profile_unavailable`, `feature_missing` | `stype_unsupported` |

---

## Prometheus Configuration

### Basic Scrape Config

```yaml
# prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: "mpl-proxy"
    static_configs:
      - targets: ["mpl-proxy:9100"]
        labels:
          environment: "production"
          service: "mpl"
    metrics_path: "/metrics"
    scrape_interval: 10s
```

### Multi-Instance Scrape Config

For environments with multiple MPL proxy instances:

```yaml
# prometheus.yml
scrape_configs:
  - job_name: "mpl-proxy"
    static_configs:
      - targets:
          - "mpl-proxy-1:9100"
          - "mpl-proxy-2:9100"
          - "mpl-proxy-3:9100"
        labels:
          environment: "production"

  - job_name: "mpl-proxy-staging"
    static_configs:
      - targets: ["mpl-proxy-staging:9100"]
        labels:
          environment: "staging"
```

### Kubernetes ServiceMonitor

For Kubernetes deployments using the Prometheus Operator:

```yaml
# servicemonitor.yaml
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: mpl-proxy
  namespace: monitoring
  labels:
    app: mpl-proxy
    release: prometheus
spec:
  selector:
    matchLabels:
      app: mpl-proxy
  namespaceSelector:
    matchNames:
      - mpl
  endpoints:
    - port: metrics
      path: /metrics
      interval: 10s
      scrapeTimeout: 5s
      honorLabels: true
```

With the corresponding Kubernetes Service:

```yaml
# service.yaml
apiVersion: v1
kind: Service
metadata:
  name: mpl-proxy
  namespace: mpl
  labels:
    app: mpl-proxy
spec:
  selector:
    app: mpl-proxy
  ports:
    - name: proxy
      port: 9443
      targetPort: 9443
    - name: metrics
      port: 9100
      targetPort: 9100
    - name: dashboard
      port: 9080
      targetPort: 9080
```

---

## Alerting

### Key Metrics to Alert On

| Metric / Query | Threshold | Severity | Rationale |
|----------------|-----------|----------|-----------|
| `rate(mpl_validation_errors_total[5m]) > 0.1` | > 0.1 errors/sec | Warning | Schema violations increasing |
| `rate(mpl_validation_errors_total[5m]) > 1.0` | > 1.0 errors/sec | Critical | High validation failure rate |
| `rate(mpl_qom_breaches_total[5m]) > 0.05` | > 0.05 breaches/sec | Warning | QoM quality degrading |
| `histogram_quantile(0.99, mpl_proxy_latency_seconds) > 0.05` | p99 > 50ms | Warning | Proxy latency spike |
| `histogram_quantile(0.99, mpl_proxy_latency_seconds) > 0.1` | p99 > 100ms | Critical | Severe latency degradation |
| `rate(mpl_unknown_stype_total[5m]) > 0.5` | > 0.5/sec | Info | New unmapped tools appearing |
| `rate(mpl_downgrade_total[5m]) > 0.1` | > 0.1/sec | Warning | Frequent negotiation downgrades |
| `up{job="mpl-proxy"} == 0` | Instance down | Critical | Proxy unreachable |

### Prometheus Alert Rules

```yaml
# alerts.yml
groups:
  - name: mpl-proxy
    rules:
      - alert: MplHighValidationErrorRate
        expr: rate(mpl_validation_errors_total[5m]) > 0.1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High MPL validation error rate"
          description: "Validation errors are occurring at {{ $value | printf \"%.2f\" }}/sec for {{ $labels.stype }}"

      - alert: MplCriticalValidationErrors
        expr: rate(mpl_validation_errors_total[5m]) > 1.0
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "Critical MPL validation error rate"
          description: "Validation errors exceeding 1/sec. Possible schema mismatch or upstream change."

      - alert: MplQomBreaches
        expr: rate(mpl_qom_breaches_total[5m]) > 0.05
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "QoM breaches detected"
          description: "Quality breaches for {{ $labels.stype }} on metric {{ $labels.metric }}"

      - alert: MplHighLatency
        expr: histogram_quantile(0.99, rate(mpl_proxy_latency_seconds_bucket[5m])) > 0.05
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "MPL proxy p99 latency above 50ms"
          description: "Proxy latency p99 is {{ $value | printf \"%.3f\" }}s for {{ $labels.stype }}"

      - alert: MplProxyDown
        expr: up{job="mpl-proxy"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "MPL proxy instance down"
          description: "MPL proxy at {{ $labels.instance }} is unreachable"
```

---

## Grafana Dashboard

### Recommended Panels

Set up a Grafana dashboard with these panels for comprehensive MPL monitoring:

#### Row 1: Overview

| Panel | Type | Query |
|-------|------|-------|
| Request Rate | Time series | `rate(mpl_requests_total[5m])` |
| Error Rate | Time series | `rate(mpl_validation_errors_total[5m])` |
| Success Rate % | Stat | `1 - (rate(mpl_validation_errors_total[5m]) / rate(mpl_requests_total[5m]))` |
| Active STypes | Stat | `count(count by (stype) (mpl_requests_total))` |

#### Row 2: QoM Quality

| Panel | Type | Query |
|-------|------|-------|
| QoM Score Distribution | Heatmap | `mpl_qom_score_bucket` |
| QoM Breaches by SType | Bar chart | `sum by (stype) (rate(mpl_qom_breaches_total[1h]))` |
| Avg Schema Fidelity | Gauge | `avg(mpl_qom_score{metric="schema_fidelity"})` |
| Breach Rate | Time series | `rate(mpl_qom_breaches_total[5m])` |

#### Row 3: Latency

| Panel | Type | Query |
|-------|------|-------|
| Proxy Latency (p50/p95/p99) | Time series | `histogram_quantile(0.5\|0.95\|0.99, rate(mpl_proxy_latency_seconds_bucket[5m]))` |
| Handshake Duration | Time series | `histogram_quantile(0.95, rate(mpl_handshake_duration_seconds_bucket[5m]))` |
| Latency by SType | Table | `histogram_quantile(0.95, rate(mpl_proxy_latency_seconds_bucket[5m])) by (stype)` |

#### Row 4: Errors and Downgrades

| Panel | Type | Query |
|-------|------|-------|
| Validation Errors by Code | Pie chart | `sum by (error_code) (mpl_validation_errors_total)` |
| Unknown SType Rate | Time series | `rate(mpl_unknown_stype_total[5m])` |
| Downgrades by Reason | Bar chart | `sum by (reason) (mpl_downgrade_total)` |

### Dashboard JSON Import

A pre-built Grafana dashboard is available:

```bash
# Download the MPL Grafana dashboard
curl -o mpl-dashboard.json \
  https://raw.githubusercontent.com/mpl-dev/mpl/main/dashboards/grafana-mpl-proxy.json

# Import via Grafana API
curl -X POST http://admin:admin@localhost:3000/api/dashboards/db \
  -H "Content-Type: application/json" \
  -d @mpl-dashboard.json
```

---

## Built-in Dashboard

MPL includes a built-in web dashboard accessible at port **9080** (no Grafana required):

```bash
# Access the dashboard
open http://localhost:9080
```

The built-in dashboard provides:

- Real-time request rate and error rate
- Per-SType validation status
- QoM score summaries
- Recent validation errors with full context
- Active sessions and negotiated capabilities
- Registry schema inventory

!!! info "Dashboard Configuration"
    ```yaml
    # mpl-config.yaml
    dashboard:
      enabled: true
      listen: "0.0.0.0:9080"
      auth:
        enabled: false          # Enable for production
        username: "admin"
        password_env: "MPL_DASHBOARD_PASSWORD"
    ```

---

## Common PromQL Queries

### Traffic Analysis

```promql
# Total request rate by SType
sum by (stype) (rate(mpl_requests_total[5m]))

# Request rate by status
sum by (status) (rate(mpl_requests_total[5m]))

# Top 5 busiest STypes
topk(5, sum by (stype) (rate(mpl_requests_total[5m])))

# Percentage of requests with unknown SType
rate(mpl_unknown_stype_total[5m]) / rate(mpl_requests_total[5m]) * 100
```

### Validation Health

```promql
# Overall validation success rate
1 - (sum(rate(mpl_validation_errors_total[5m])) / sum(rate(mpl_requests_total[5m])))

# Validation errors by error code
sum by (error_code) (rate(mpl_validation_errors_total[5m]))

# STypes with highest error rate
topk(3, sum by (stype) (rate(mpl_validation_errors_total[5m])))

# Error rate trend (increase over 1 hour)
delta(mpl_validation_errors_total[1h])
```

### QoM Analysis

```promql
# Average QoM score per SType
avg by (stype) (mpl_qom_score)

# QoM breach rate by metric dimension
sum by (metric) (rate(mpl_qom_breaches_total[5m]))

# Percentage of requests breaching QoM profile
sum(rate(mpl_qom_breaches_total[5m])) / sum(rate(mpl_requests_total[5m])) * 100

# Low-quality STypes (avg schema_fidelity below 0.9)
avg by (stype) (mpl_qom_score{metric="schema_fidelity"}) < 0.9
```

### Latency Analysis

```promql
# Proxy overhead percentiles
histogram_quantile(0.50, rate(mpl_proxy_latency_seconds_bucket[5m]))
histogram_quantile(0.95, rate(mpl_proxy_latency_seconds_bucket[5m]))
histogram_quantile(0.99, rate(mpl_proxy_latency_seconds_bucket[5m]))

# Slowest STypes by p95 latency
topk(5, histogram_quantile(0.95, sum by (stype, le) (rate(mpl_proxy_latency_seconds_bucket[5m]))))

# Handshake duration p99
histogram_quantile(0.99, rate(mpl_handshake_duration_seconds_bucket[5m]))
```

---

## Health Checks

The proxy exposes a health endpoint for load balancers and orchestrators:

```bash
# Basic health check
curl http://localhost:9443/health

# Response:
# {
#   "status": "healthy",
#   "mode": "production",
#   "upstream": "http://mcp-server:8080",
#   "upstream_healthy": true,
#   "registry_loaded": true,
#   "schemas_count": 12,
#   "uptime_seconds": 86400
# }
```

### Kubernetes Probes

```yaml
# deployment.yaml
spec:
  containers:
    - name: mpl-proxy
      livenessProbe:
        httpGet:
          path: /health
          port: 9443
        initialDelaySeconds: 5
        periodSeconds: 10
      readinessProbe:
        httpGet:
          path: /health
          port: 9443
        initialDelaySeconds: 3
        periodSeconds: 5
```

---

## Logging

MPL outputs structured JSON logs that complement metrics for debugging:

```bash
# Set log level
RUST_LOG=info mpl proxy http://mcp-server:8080

# Available levels: error, warn, info, debug, trace
RUST_LOG=debug mpl proxy http://mcp-server:8080
```

### Log Fields

| Field | Description | Example |
|-------|-------------|---------|
| `timestamp` | ISO 8601 timestamp | `2025-01-15T10:00:00.123Z` |
| `level` | Log level | `INFO`, `WARN`, `ERROR` |
| `target` | Rust module path | `mpl_proxy::validation` |
| `stype` | Resolved SType (if applicable) | `org.calendar.Event.v1` |
| `request_id` | Unique request identifier | `req-a7b3c9d1` |
| `sem_hash` | Semantic hash of payload | `blake3:f47ac10b...` |
| `validation_result` | Pass/fail with details | `{"valid": false, "errors": [...]}` |
| `qom_pass` | QoM profile result | `true` |
| `latency_ms` | Proxy processing time | `3.2` |

### Example Log Entry

```json
{
  "timestamp": "2025-01-15T10:00:01.234Z",
  "level": "WARN",
  "target": "mpl_proxy::validation",
  "message": "Schema validation failed",
  "request_id": "req-a7b3c9d1",
  "stype": "org.calendar.Event.v1",
  "errors": [
    {"path": "/end", "message": "required property is missing"}
  ],
  "client_addr": "10.0.1.5:48230"
}
```

---

## Next Steps

- **[Troubleshooting](troubleshooting.md)** -- Diagnose common operational issues
- **[Existing Infrastructure](../integration/existing-infrastructure.md)** -- Migration metrics to track
- **[Concepts: QoM](../../concepts/qom.md)** -- Understand the quality metrics being measured
