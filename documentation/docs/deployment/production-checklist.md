---
title: Production Checklist
description: Security, reliability, observability, and operational requirements for production MPL deployments
---

# Production Checklist

Use this checklist to verify your MPL proxy deployment is production-ready. Each category covers a critical dimension of operational excellence. Complete all items before routing production traffic through the proxy.

!!! tip "How to Use This Checklist"
    Work through each section sequentially. Items marked as critical are hard requirements. Items marked as recommended are best practices that significantly reduce operational risk.

---

## Security

Ensure the proxy and its environment are hardened against unauthorized access and data exposure.

- [ ] **TLS termination** configured at ingress or load balancer level
- [ ] **Internal TLS** enabled between proxy and upstream if crossing network boundaries
- [ ] **Network policies** deployed to restrict ingress/egress to known peers only
- [ ] **RBAC** configured with least-privilege ServiceAccount (no cluster-admin)
- [ ] **Pod security context** enforces non-root, read-only filesystem, dropped capabilities
- [ ] **Secrets management** used for sensitive config (not plain ConfigMaps)
- [ ] **Image scanning** completed -- no critical CVEs in the runtime image
- [ ] **Image provenance** verified -- signed images from trusted registry
- [ ] **Private registry** used for image pulls (not pulling from public in production)
- [ ] **Ingress authentication** configured (API keys, mTLS, or OAuth2 proxy)
- [ ] **Rate limiting** enabled at ingress to prevent abuse
- [ ] **Audit logging** enabled for all administrative actions

??? example "Security Context Example"
    ```yaml
    securityContext:
      capabilities:
        drop:
          - ALL
      readOnlyRootFilesystem: true
      runAsNonRoot: true
      runAsUser: 1000
      allowPrivilegeEscalation: false
      seccompProfile:
        type: RuntimeDefault
    ```

---

## Reliability

Ensure the proxy can withstand failures, traffic spikes, and infrastructure changes.

- [ ] **Resource limits** set for CPU and memory (prevent noisy-neighbor issues)
- [ ] **Resource requests** set to guarantee scheduling capacity
- [ ] **Horizontal Pod Autoscaler (HPA)** enabled with appropriate min/max replicas
- [ ] **Pod Disruption Budget (PDB)** configured (`minAvailable: 1` or `maxUnavailable: 1`)
- [ ] **Liveness probe** enabled -- restarts pods that become unresponsive
- [ ] **Readiness probe** enabled -- removes pods from service during startup/failures
- [ ] **Startup probe** configured for slow-starting pods (if applicable)
- [ ] **Graceful shutdown** configured with appropriate `terminationGracePeriodSeconds`
- [ ] **Pre-stop hook** added to drain connections before termination
- [ ] **Anti-affinity rules** spread replicas across nodes/zones
- [ ] **Multiple replicas** running (minimum 2 for high availability)
- [ ] **Rolling update strategy** configured with `maxSurge` and `maxUnavailable`
- [ ] **Priority class** assigned to prevent preemption of critical workloads

??? example "Reliability Configuration"
    ```yaml
    # HPA
    autoscaling:
      enabled: true
      minReplicas: 2
      maxReplicas: 10
      targetCPUUtilizationPercentage: 70

    # PDB
    apiVersion: policy/v1
    kind: PodDisruptionBudget
    spec:
      minAvailable: 1

    # Graceful shutdown
    spec:
      terminationGracePeriodSeconds: 30
      containers:
        - lifecycle:
            preStop:
              exec:
                command: ["/bin/sh", "-c", "sleep 5"]
    ```

---

## Observability

Ensure you can monitor, debug, and alert on proxy behavior in production.

- [ ] **Prometheus metrics** exposed on port 9100 and scraped
- [ ] **ServiceMonitor** (or PodMonitor) created for Prometheus Operator discovery
- [ ] **Structured logging** enabled (JSON format) for log aggregation
- [ ] **Log level** set to `info` (not `debug` or `trace` in production)
- [ ] **Dashboards** created for key metrics (request rate, error rate, latency, QoM scores)
- [ ] **Alerting rules** configured for critical conditions:
    - [ ] High error rate (> 5% of requests returning errors)
    - [ ] High latency (p99 > SLA threshold)
    - [ ] Pod restarts (> 3 in 10 minutes)
    - [ ] QoM score degradation (below profile threshold)
    - [ ] Schema validation failure rate spike
- [ ] **Distributed tracing** headers propagated (if using Jaeger/Zipkin/OTEL)
- [ ] **Log retention** configured per compliance requirements
- [ ] **Metric cardinality** reviewed -- no unbounded label values

??? example "Alerting Rules"
    ```yaml
    apiVersion: monitoring.coreos.com/v1
    kind: PrometheusRule
    metadata:
      name: mpl-proxy-alerts
    spec:
      groups:
        - name: mpl-proxy
          rules:
            - alert: MPLHighErrorRate
              expr: |
                rate(mpl_requests_total{status="error"}[5m])
                / rate(mpl_requests_total[5m]) > 0.05
              for: 5m
              labels:
                severity: critical
              annotations:
                summary: "MPL proxy error rate above 5%"

            - alert: MPLHighLatency
              expr: |
                histogram_quantile(0.99, rate(mpl_request_duration_seconds_bucket[5m])) > 1.0
              for: 5m
              labels:
                severity: warning
              annotations:
                summary: "MPL proxy p99 latency above 1s"

            - alert: MPLQoMDegradation
              expr: |
                avg(mpl_qom_score) < 0.7
              for: 10m
              labels:
                severity: warning
              annotations:
                summary: "Average QoM score below threshold"
    ```

---

## Configuration

Ensure the MPL proxy is configured for production-grade semantic governance.

- [ ] **Strict mode** enabled (`mpl.mode: strict`) -- block invalid requests
- [ ] **QoM profile** set to production profile (`qom-strict-argcheck` or stricter)
- [ ] **Schema enforcement** enabled (`mpl.enforceSchema: true`)
- [ ] **Assertion checks** enabled (`mpl.enforceAssertions: true`)
- [ ] **Policy engine** enabled for organizational rules (if applicable)
- [ ] **Registry source** pointed to versioned, immutable registry (not mutable branch)
- [ ] **Config immutability** ensured -- config changes require redeployment
- [ ] **Config checksum annotation** enabled for automatic rollout on config change
- [ ] **Environment-specific overrides** managed via Helm values (not inline edits)
- [ ] **Upstream URL** uses fully-qualified service DNS (`.svc.cluster.local`)

??? example "Production MPL Configuration"
    ```yaml
    mpl:
      mode: strict
      registry: "https://github.com/anthropics/mpl/raw/v0.1.0/registry"
      required_profile: qom-strict-argcheck
      enforce_schema: true
      enforce_assertions: true
      policy_engine: true
    ```

---

## Operations

Ensure your team is prepared to operate, maintain, and recover the deployment.

- [ ] **Runbook** written covering common failure scenarios and resolution steps
- [ ] **Rollback plan** documented -- ability to `helm rollback` within minutes
- [ ] **Incident response** playbook includes MPL proxy failure as a scenario
- [ ] **On-call rotation** covers MPL proxy alerts
- [ ] **Backup strategy** for registry schemas and configuration (version-controlled)
- [ ] **Change management** process defined for config and registry updates
- [ ] **Canary deployments** or blue-green strategy for proxy upgrades
- [ ] **Load testing** completed at expected peak traffic (validate HPA behavior)
- [ ] **Chaos testing** performed (pod kill, network partition, resource exhaustion)
- [ ] **Upgrade path** documented for patch, minor, and major version bumps
- [ ] **Dependency inventory** maintained (base image, Rust version, chart version)

??? example "Rollback Procedure"
    ```bash
    # View release history
    helm history mpl-proxy

    # Rollback to previous revision
    helm rollback mpl-proxy 1

    # Verify rollback
    kubectl get pods -l app.kubernetes.io/name=mpl-proxy
    kubectl logs -l app.kubernetes.io/name=mpl-proxy --tail=50
    ```

---

## Compliance

Ensure the deployment meets audit, regulatory, and governance requirements.

- [ ] **Audit log retention** configured per organizational policy (30/90/365 days)
- [ ] **Provenance verification** enabled -- semantic hashes validated on all messages
- [ ] **Policy documentation** published -- what rules are enforced and why
- [ ] **SType registry versioning** -- all schema changes are tracked and auditable
- [ ] **Access control documentation** -- who can modify config, registry, and policies
- [ ] **Data residency** verified -- proxy and logs remain in approved regions
- [ ] **Compliance mapping** reviewed against applicable frameworks (SOC 2, ISO 27001, etc.)
- [ ] **Penetration testing** completed on exposed endpoints
- [ ] **Privacy impact assessment** completed if processing PII
- [ ] **Retention policies** automated -- old audit logs purged per schedule

??? example "Audit Log Configuration"
    ```yaml
    observability:
      log_format: json
      log_level: info
      # Structured fields for audit trail
      # Each request logs: timestamp, stype, qom_score,
      # semantic_hash, source, decision (allow/deny)
    ```

---

## Summary

| Category | Items | Priority |
|----------|:-----:|----------|
| **Security** | 12 | Critical -- must complete before production traffic |
| **Reliability** | 13 | Critical -- ensures availability SLA |
| **Observability** | 9 | High -- required for operational awareness |
| **Configuration** | 10 | High -- ensures semantic governance is active |
| **Operations** | 11 | Medium -- reduces MTTR and operational risk |
| **Compliance** | 10 | Varies -- depends on regulatory environment |

---

## Quick Validation Commands

Run these commands to verify key aspects of your deployment:

```bash
# Verify pods are running and ready
kubectl get pods -l app.kubernetes.io/name=mpl-proxy -o wide

# Check resource limits are set
kubectl describe pod -l app.kubernetes.io/name=mpl-proxy | grep -A 4 "Limits:"

# Verify health endpoints
kubectl exec deploy/mpl-proxy -- curl -s http://localhost:9443/health

# Check metrics are being scraped
kubectl exec deploy/mpl-proxy -- curl -s http://localhost:9100/metrics | head -20

# Verify network policy exists
kubectl get networkpolicy -l app.kubernetes.io/name=mpl-proxy

# Check HPA status
kubectl get hpa mpl-proxy

# Verify PDB exists
kubectl get pdb mpl-proxy-pdb

# Check ServiceMonitor
kubectl get servicemonitor mpl-proxy -n monitoring
```

---

## Next Steps

- [Kubernetes & Helm](kubernetes.md) -- Detailed Helm chart configuration
- [Monitoring & Metrics](../guides/operations/monitoring.md) -- Set up dashboards and alerts
- [Troubleshooting](../guides/operations/troubleshooting.md) -- Debug production issues
- [Security: Threat Model](../security/threat-model.md) -- Understand the threat landscape
