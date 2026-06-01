---
title: Troubleshooting
description: Common issues, diagnosis steps, and solutions for MPL proxy, SDK, and schema validation problems
---

# Troubleshooting

This guide covers common issues encountered when operating MPL, organized by symptom. Each problem includes diagnosis steps, solutions, and prevention measures.

---

## Diagnostic Commands

Before diving into specific issues, use these commands to assess the current state of your MPL deployment:

```bash
# Check proxy health
curl http://localhost:9443/health

# Check metrics endpoint
curl http://localhost:9100/metrics | grep mpl_

# List registered schemas
mpl schemas list

# Check proxy logs (increase verbosity)
RUST_LOG=debug mpl proxy http://mcp-server:8080

# Test upstream connectivity
curl http://mcp-server:8080/health
```

---

## Proxy Won't Start

### Port Conflict

!!! failure "Symptoms"
    ```
    Error: Address already in use (os error 98)
    Error: Failed to bind to 0.0.0.0:9443
    ```

**Diagnosis:**

```bash
# Check what is using port 9443
lsof -i :9443
# or
ss -tlnp | grep 9443
```

**Solution:**

```bash
# Option 1: Stop the conflicting process
kill <PID>

# Option 2: Use a different port
mpl proxy http://mcp-server:8080 --listen 0.0.0.0:9444
```

**Prevention:** Use a dedicated port range for MPL services in your infrastructure. Document port assignments:

| Port | Service |
|------|---------|
| 9443 | MPL proxy |
| 9100 | Prometheus metrics |
| 9080 | Built-in dashboard |

---

### Missing or Invalid Configuration

!!! failure "Symptoms"
    ```
    Error: Config file not found: mpl-config.yaml
    Error: Invalid configuration: missing required field 'upstream'
    Error: Failed to parse config: expected string, found integer at line 5
    ```

**Diagnosis:**

```bash
# Verify config file exists
ls -la mpl-config.yaml

# Validate YAML syntax
python3 -c "import yaml; yaml.safe_load(open('mpl-config.yaml'))"

# Check required fields
grep -E "^(upstream|listen):" mpl-config.yaml
```

**Solution:**

Ensure your configuration has all required fields:

```yaml
# Minimum viable mpl-config.yaml
upstream: "http://mcp-server:8080"    # Required
listen: "0.0.0.0:9443"               # Required
mode: learning                        # Optional, defaults to transparent
```

**Prevention:** Use the `--config` flag explicitly and keep a validated template in version control.

---

### Upstream Unreachable

!!! failure "Symptoms"
    ```
    Error: Failed to connect to upstream: Connection refused
    Error: Upstream health check failed: http://mcp-server:8080
    WARN: Upstream not reachable, starting in degraded mode
    ```

**Diagnosis:**

```bash
# Test upstream connectivity
curl -v http://mcp-server:8080/health

# Check DNS resolution
nslookup mcp-server
# or
dig mcp-server

# Check network path
traceroute mcp-server
```

**Solution:**

```bash
# Verify the upstream URL is correct
mpl proxy http://correct-host:correct-port

# If using Docker, ensure services are on the same network
docker network ls
docker network inspect <network-name>
```

**Prevention:** Add the upstream health check to your deployment pipeline. The proxy will start in degraded mode if the upstream is temporarily unavailable, and reconnect when it becomes healthy.

---

## Schema Validation Errors

### Unknown SType

!!! failure "Symptoms"
    ```
    WARN: No SType mapping for tool: "custom.tool.name"
    Metric: mpl_unknown_stype_total increasing
    ```

**Diagnosis:**

```bash
# List registered SType mappings
mpl schemas list

# Check if the tool is mapped in config
grep "custom.tool.name" mpl-config.yaml

# Check metrics for unknown SType rate
curl -s http://localhost:9100/metrics | grep mpl_unknown_stype_total
```

**Solution:**

Add the missing mapping to your configuration:

```yaml
# mpl-config.yaml
stype_mappings:
  - tool: "custom.tool.name"
    stype: "com.yourorg.domain.Intent.v1"
```

Or enable learning mode to auto-detect:

```bash
mpl proxy http://mcp-server:8080 --learn
# Wait for traffic, then:
mpl schemas generate
```

**Prevention:** Maintain a comprehensive tool-to-SType mapping. Run in learning mode periodically to detect new tools.

---

### Schema Mismatch

!!! failure "Symptoms"
    ```json
    {
      "error": "MPL schema validation failed",
      "errors": [
        {"path": "/amount", "message": "expected number, found string"},
        {"path": "/metadata", "message": "additional property not allowed"}
      ]
    }
    ```

**Diagnosis:**

```bash
# View the current schema
mpl schemas show org.calendar.Event.v1 --format json

# Compare with actual payload (from logs)
RUST_LOG=debug mpl proxy http://mcp-server:8080
# Look for "payload" field in log entries

# Validate a specific payload against a schema
echo '{"title": "Test", "amount": "100"}' | mpl validate --stype com.acme.finance.Transaction.v3
```

**Solution:**

1. If the **schema is wrong** (too strict or outdated):

    ```bash
    # Regenerate from current traffic
    mpl schemas generate --stype com.acme.finance.Transaction.v3

    # Or manually edit the schema
    vim registry/stypes/com.acme/finance/Transaction/v3/schema.json

    # Reload the proxy (no restart needed if using file watcher)
    ```

2. If the **payload is wrong** (client sending bad data):

    - Fix the client to send conforming payloads
    - Switch the SType to `warn` mode while fixing: `enforcement.overrides[].mode: warn`

**Prevention:** Run schema validation in CI/CD pipelines. Keep schemas in version control and review changes.

---

### additionalProperties Violations

!!! failure "Symptoms"
    ```json
    {"path": "/extraField", "message": "additional property 'extraField' is not allowed"}
    ```

**Diagnosis:**

This occurs when a payload contains fields not declared in the schema. All MPL schemas require `additionalProperties: false`.

```bash
# See what fields the schema expects
mpl schemas show org.calendar.Event.v1 --format json | jq '.properties | keys'
```

**Solution:**

1. If the extra field is **legitimate**, add it to the schema as an optional property:

    ```json
    {
      "properties": {
        "existingField": { "type": "string" },
        "newField": { "type": "string", "description": "Newly added optional field" }
      }
    }
    ```

    !!! warning "Version Impact"
        Adding a new optional field is a **minor** version change. Update `metadata.json` accordingly.

2. If the extra field is **unwanted**, fix the client to stop sending it.

---

## QoM Breaches

### Threshold Too Strict

!!! failure "Symptoms"
    ```
    WARN: QoM breach for org.calendar.Event.v1: instruction_compliance=0.87 < threshold=0.95
    Metric: mpl_qom_breaches_total increasing for specific SType
    ```

**Diagnosis:**

```bash
# Check current profile thresholds
mpl profiles show qom-strict-argcheck

# Check QoM score distribution
curl -s http://localhost:9100/metrics | grep 'mpl_qom_score{' | grep instruction_compliance

# Review recent QoM reports in logs
RUST_LOG=debug mpl proxy http://mcp-server:8080
```

**Solution:**

1. **Relax the threshold** for the specific metric:

    ```yaml
    # In registry profile definition
    profiles:
      qom-adjusted:
        thresholds:
          schema_fidelity: 0.95
          instruction_compliance: 0.80    # Lowered from 0.95
          context_grounding: 0.70
    ```

2. **Switch to a less strict profile:**

    ```yaml
    # mpl-config.yaml
    profile: "qom-basic"    # Instead of qom-strict-argcheck
    ```

3. **Use per-SType profiles:**

    ```yaml
    enforcement:
      overrides:
        - stype: "org.calendar.Event.v1"
          profile: "qom-basic"          # Relaxed for this SType
        - stype: "com.acme.finance.Transaction.v3"
          profile: "qom-comprehensive"  # Strict for financial
    ```

**Prevention:** Start with `qom-basic` and progressively tighten. Monitor score distributions before raising thresholds.

---

### Assertion Failures

!!! failure "Symptoms"
    ```
    ERROR: CEL assertion failed for org.calendar.Event.v1:
      assertion "end_after_start" failed: !(payload.end > payload.start)
    ```

**Diagnosis:**

```bash
# View assertions for the SType
cat registry/stypes/org/calendar/Event/v1/assertions.cel

# Test an assertion manually
mpl validate --stype org.calendar.Event.v1 --payload '{"title":"Test","start":"2025-01-15T10:00:00Z","end":"2025-01-15T09:00:00Z"}'
```

**Solution:**

1. If the assertion is **correct**, fix the payload (the data violates a business rule)
2. If the assertion is **too strict**, update the CEL expression:

    ```cel
    // assertions.cel - relaxed version
    // Allow same-time events (zero-duration)
    payload.end >= payload.start
    ```

---

### Missing Context

!!! failure "Symptoms"
    ```
    WARN: QoM metric 'context_grounding' scored 0.0: no context provided
    WARN: QoM metric 'provenance_completeness' scored 0.0: empty provenance chain
    ```

**Diagnosis:**

These metrics require additional context that may not be available in all deployments:

- `context_grounding`: Requires reference context to compare against
- `provenance_completeness`: Requires provenance chain (A2A mode)

**Solution:**

If you are not using features that provide this context, switch to a profile that does not require them:

```yaml
# Use a profile that only measures available metrics
profile: "qom-basic"    # Only schema_fidelity and instruction_compliance
```

Or configure the QoM engine to skip unavailable metrics:

```yaml
qom:
  skip_unavailable_metrics: true    # Score only metrics with available data
```

---

## Connection Issues

### Timeout Configuration

!!! failure "Symptoms"
    ```
    Error: Request timed out after 30s
    Error: Upstream response timeout
    WARN: Slow upstream response: 28.5s for tools/call
    ```

**Diagnosis:**

```bash
# Check current timeout settings
grep timeout mpl-config.yaml

# Test upstream response time directly
time curl http://mcp-server:8080/slow-endpoint
```

**Solution:**

Adjust timeout settings in configuration:

```yaml
# mpl-config.yaml
timeouts:
  connect: 5s              # Time to establish connection to upstream
  request: 60s             # Total time for request/response cycle
  idle: 300s               # Keep-alive idle timeout
  handshake: 10s           # AI-ALPN negotiation timeout
```

For specific slow tools, consider per-route timeouts:

```yaml
timeouts:
  request: 30s             # Default
  overrides:
    - tool: "data.analysis.run"
      request: 300s        # 5 minutes for long-running analysis
```

---

### WebSocket vs HTTP

!!! failure "Symptoms"
    ```
    Error: Unexpected upgrade request
    Error: WebSocket handshake failed
    WARN: Received WebSocket frame on HTTP endpoint
    ```

**Diagnosis:**

```bash
# Check if the MCP server uses WebSocket
curl -v -H "Upgrade: websocket" http://mcp-server:8080
```

**Solution:**

Configure the proxy transport mode to match your MCP server:

```yaml
# mpl-config.yaml
mcp:
  transport: websocket      # Match server transport: http | websocket
```

```bash
# Or via CLI flag
mpl proxy http://mcp-server:8080 --transport websocket
```

!!! info "Transport Detection"
    The proxy attempts to auto-detect the transport mode. If auto-detection fails, set it explicitly in the configuration.

---

### DNS Resolution

!!! failure "Symptoms"
    ```
    Error: Failed to resolve hostname: mcp-server
    Error: DNS lookup failed for upstream
    ```

**Diagnosis:**

```bash
# Test DNS resolution
nslookup mcp-server
dig mcp-server

# Check /etc/resolv.conf
cat /etc/resolv.conf

# Try with IP address directly
curl http://10.0.1.5:8080/health
```

**Solution:**

```bash
# Use IP address instead of hostname
mpl proxy http://10.0.1.5:8080

# Or add to /etc/hosts
echo "10.0.1.5 mcp-server" >> /etc/hosts

# For Docker: ensure services are on the same network
docker network connect mpl-network mcp-server
```

---

## Performance Issues

### High Latency

!!! failure "Symptoms"
    ```
    Metric: mpl_proxy_latency_seconds p99 > 50ms
    Users reporting slow tool responses
    Dashboard showing latency spikes
    ```

**Diagnosis:**

```bash
# Check latency breakdown
curl -s http://localhost:9100/metrics | grep mpl_proxy_latency_seconds

# Check schema cache hit rate
curl -s http://localhost:9100/metrics | grep mpl_cache

# Check if registry is remote
grep registry mpl-config.yaml

# Profile with debug logging
RUST_LOG=mpl_proxy::timing=debug mpl proxy http://mcp-server:8080
```

**Solution:**

| Cause | Fix |
|-------|-----|
| Remote registry | Switch to local file registry: `registry: "file://./registry"` |
| Schema cache miss | Pre-warm cache: `mpl schemas preload` |
| Large schemas | Simplify deeply nested schemas; split into sub-schemas |
| QoM evaluation slow | Use `qom-basic` profile (fewer metrics to compute) |
| CEL assertions complex | Simplify assertion expressions; reduce assertion count |

```yaml
# mpl-config.yaml - performance tuning
registry: "file://./registry"      # Local, not remote

cache:
  max_schemas: 1000                # Increase cache size
  ttl: 3600s                       # Cache for 1 hour

qom:
  timeout: 50ms                    # Cap QoM evaluation time
```

---

### High Memory Usage

!!! failure "Symptoms"
    ```
    Container OOMKilled
    Memory usage growing over time
    Metric: process_resident_memory_bytes climbing
    ```

**Diagnosis:**

```bash
# Check memory usage
ps aux | grep mpl
# or for containers:
docker stats mpl-proxy

# Check registry size
du -sh registry/
find registry/ -name "*.json" | wc -l

# Check number of cached schemas
curl -s http://localhost:9100/metrics | grep mpl_cache_size
```

**Solution:**

```yaml
# mpl-config.yaml - memory tuning
cache:
  max_schemas: 100               # Limit cached schemas (default: unlimited)
  eviction: lru                  # Evict least-recently-used

learning:
  max_samples_per_tool: 1000     # Limit learning buffer
  flush_interval: 60s            # Write to disk more frequently
```

For container deployments, set appropriate resource limits:

```yaml
# Kubernetes deployment
resources:
  requests:
    memory: "128Mi"
  limits:
    memory: "512Mi"
```

---

## SDK Errors

### Connection Refused

!!! failure "Symptoms"
    ```python
    ConnectionError: Connection refused at localhost:9443
    ```
    ```typescript
    Error: connect ECONNREFUSED 127.0.0.1:9443
    ```

**Diagnosis:**

```bash
# Check if proxy is running
curl http://localhost:9443/health

# Check if port is open
nc -zv localhost 9443

# Check proxy process
ps aux | grep mpl
```

**Solution:**

1. Ensure the proxy is running:

    ```bash
    mpl proxy http://mcp-server:8080
    ```

2. Verify the SDK is pointing at the correct address:

    ```python
    # Python
    client = Client("http://localhost:9443")  # Not 8080!
    ```

    ```typescript
    // TypeScript
    const client = new MplClient('http://localhost:9443');
    ```

3. For containerized deployments, use the service name:

    ```python
    client = Client("http://mpl-proxy:9443")
    ```

---

### Negotiation Failures

!!! failure "Symptoms"
    ```python
    NegotiationError: No compatible STypes found
    NegotiationError: Profile 'qom-comprehensive' not supported by server
    ```

**Diagnosis:**

```bash
# Check what the server supports
curl http://localhost:9443/health | jq '.capabilities'

# Check proxy logs for handshake details
RUST_LOG=mpl_proxy::handshake=debug mpl proxy http://mcp-server:8080
```

**Solution:**

Ensure the STypes and profiles you request are registered:

```python
# Request only STypes that are registered
session = await client.negotiate(
    stypes=["org.calendar.Event.v1"],     # Must be in registry
    profile="qom-basic"                    # Must be a known profile
)
```

Check the registry:

```bash
mpl schemas list
# Verify the SType exists in the output
```

---

### SDK Timeout

!!! failure "Symptoms"
    ```python
    TimeoutError: Request timed out after 30.0 seconds
    ```

**Diagnosis:**

```bash
# Check if proxy is responding
time curl http://localhost:9443/health

# Check upstream latency
time curl http://mcp-server:8080/health
```

**Solution:**

Configure SDK timeouts:

```python
# Python SDK
client = Client(
    "http://localhost:9443",
    timeout=60.0,              # Increase from default 30s
    connect_timeout=5.0
)
```

```typescript
// TypeScript SDK
const client = new MplClient('http://localhost:9443', {
  timeout: 60000,             // 60 seconds
  connectTimeout: 5000,       // 5 seconds
});
```

---

## Log Analysis

### Understanding Structured Logs

MPL outputs structured JSON logs. Key fields to look for when diagnosing issues:

```bash
# Filter for errors only
RUST_LOG=error mpl proxy http://mcp-server:8080

# Filter for a specific module
RUST_LOG=mpl_proxy::validation=debug mpl proxy http://mcp-server:8080

# Combine levels
RUST_LOG=info,mpl_proxy::validation=debug,mpl_proxy::qom=debug mpl proxy http://mcp-server:8080
```

### Key Log Fields

| Field | When to Check | What It Tells You |
|-------|--------------|-------------------|
| `request_id` | Tracing a specific request | Correlate across log lines |
| `stype` | Validation or QoM issues | Which SType is affected |
| `errors` | Validation failures | Exact schema violations |
| `qom_scores` | QoM breaches | Which metrics failed |
| `latency_ms` | Performance issues | Where time is spent |
| `upstream_status` | Upstream errors | Server response code |
| `sem_hash` | Audit trail | Content fingerprint |
| `provenance` | Multi-hop issues | Agent chain |

### Log Aggregation

For production, pipe structured logs to your log aggregation system:

```bash
# JSON output to stdout (default)
mpl proxy http://mcp-server:8080 2>&1 | jq .

# Pipe to a file for later analysis
mpl proxy http://mcp-server:8080 2>> /var/log/mpl/proxy.jsonl

# Filter specific issues from logs
cat /var/log/mpl/proxy.jsonl | jq 'select(.level == "ERROR")'
cat /var/log/mpl/proxy.jsonl | jq 'select(.stype == "org.calendar.Event.v1")'
cat /var/log/mpl/proxy.jsonl | jq 'select(.latency_ms > 50)'
```

---

## Quick Reference

| Problem | First Check | Quick Fix |
|---------|------------|-----------|
| Proxy won't start | `lsof -i :9443` | Use `--listen 0.0.0.0:9444` |
| Validation errors | `mpl schemas show <stype>` | Switch to `mode: warn` |
| QoM breaches | Check profile thresholds | Switch to `qom-basic` |
| High latency | Registry location | Use `file://./registry` |
| SDK connection | `curl localhost:9443/health` | Verify proxy is running |
| Unknown SType | `mpl schemas list` | Add mapping or enable `--learn` |
| Memory growth | `du -sh registry/` | Set `cache.max_schemas` |
| Timeout | `time curl upstream:8080` | Increase `timeouts.request` |

---

## Getting Help

If you cannot resolve an issue with this guide:

1. **Check metrics**: `curl http://localhost:9100/metrics | grep mpl_` for quantitative state
2. **Enable debug logs**: `RUST_LOG=debug` for detailed request tracing
3. **Check the dashboard**: `http://localhost:9080` for visual overview
4. **Reproduce with minimal config**: Strip down to the simplest configuration that exhibits the issue
5. **Report**: Include the proxy version (`mpl --version`), configuration (redact secrets), relevant logs, and metrics

---

## Next Steps

- **[Monitoring & Metrics](monitoring.md)** -- Set up proactive alerting
- **[Existing Infrastructure](../integration/existing-infrastructure.md)** -- Migration-specific troubleshooting
- **[Concepts: Integration Modes](../../concepts/integration-modes.md)** -- Understanding deployment models
