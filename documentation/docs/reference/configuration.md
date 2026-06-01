---
title: Configuration
---

# Configuration Reference

Complete reference for the MPL proxy configuration file `mpl-config.yaml`. This file controls transport settings, enforcement behavior, and observability options.

---

## Configuration File Location

The MPL proxy searches for configuration in the following order:

1. Path specified by `--config` flag
2. `MPL_CONFIG` environment variable
3. `./mpl-config.yaml` (current directory)
4. `~/.mpl/mpl-config.yaml` (user home)
5. `/etc/mpl/mpl-config.yaml` (system-wide)

---

## Full Configuration Schema

```yaml
transport:
  listen: "0.0.0.0:9443"       # Proxy listen address
  upstream: "localhost:8080"    # MCP/A2A server
  protocol: http               # http | websocket

mpl:
  registry: "./registry"       # Path to SType registry
  mode: transparent            # transparent | development | strict
  required_profile: qom-basic  # Default QoM profile
  enforce_schema: true         # Validate payloads
  enforce_assertions: true     # Run IC assertions
  policy_engine: true          # Enable policy enforcement

observability:
  metrics_port: 9100
  metrics_format: prometheus
  logs: stdout                 # stdout | file
  log_format: json             # json | pretty
  log_level: info              # debug | info | warn | error
```

---

## Section Reference

### `transport`

Controls network transport behavior for the proxy.

| Field | Type | Default | Description |
|---|---|---|---|
| `listen` | `string` | `"0.0.0.0:9443"` | Address and port the proxy listens on. Format: `host:port`. |
| `upstream` | `string` | `"localhost:8080"` | Address of the upstream MCP/A2A server to proxy requests to. |
| `protocol` | `string` | `"http"` | Transport protocol. Options: `http`, `websocket`. |

#### Details

- **`listen`** - The bind address for incoming connections. Use `0.0.0.0` to accept connections from all interfaces, or `127.0.0.1` for localhost-only.
- **`upstream`** - The target backend server. Supports host:port format. TLS is negotiated automatically when the upstream uses HTTPS.
- **`protocol`** - Determines the transport layer. Use `http` for standard HTTP JSON-RPC proxying, or `websocket` for persistent WebSocket connections (required for streaming MCP servers).

---

### `mpl`

Controls MPL enforcement behavior including schema validation, QoM profiling, and policy enforcement.

| Field | Type | Default | Description |
|---|---|---|---|
| `registry` | `string` | `"./registry"` | Path to the SType registry directory containing schemas, profiles, and policies. |
| `mode` | `string` | `"transparent"` | Enforcement mode. Options: `transparent`, `development`, `strict`. |
| `required_profile` | `string` | `"qom-basic"` | Default QoM profile applied to all messages. Set to empty string to disable. |
| `enforce_schema` | `boolean` | `true` | When enabled, validates all payloads against registered SType schemas. |
| `enforce_assertions` | `boolean` | `true` | When enabled, runs Interaction Contract (IC) assertions on message flows. |
| `policy_engine` | `boolean` | `true` | When enabled, evaluates policy rules before forwarding requests. |

#### Enforcement Modes

| Mode | Schema Violations | QoM Failures | Policy Violations |
|---|---|---|---|
| `transparent` | Log only | Log only | Log only |
| `development` | Warn + pass-through | Warn + pass-through | Warn + pass-through |
| `strict` | Reject (HTTP 422) | Reject (HTTP 422) | Reject (HTTP 403) |

---

### `observability`

Controls metrics, logging, and monitoring configuration.

| Field | Type | Default | Description |
|---|---|---|---|
| `metrics_port` | `integer` | `9100` | Port for the Prometheus-compatible metrics endpoint. |
| `metrics_format` | `string` | `"prometheus"` | Metrics exposition format. Options: `prometheus`. |
| `logs` | `string` | `"stdout"` | Log output destination. Options: `stdout`, `file`. |
| `log_format` | `string` | `"json"` | Log output format. Options: `json`, `pretty`. |
| `log_level` | `string` | `"info"` | Minimum log level. Options: `debug`, `info`, `warn`, `error`. |

#### Metrics Endpoint

When the proxy is running, metrics are available at:

```
http://localhost:9100/metrics
```

Key metrics exposed:

| Metric | Type | Description |
|---|---|---|
| `mpl_requests_total` | Counter | Total requests processed |
| `mpl_validation_failures_total` | Counter | Schema validation failures |
| `mpl_qom_score` | Histogram | QoM score distribution |
| `mpl_proxy_latency_seconds` | Histogram | End-to-end proxy latency |
| `mpl_upstream_latency_seconds` | Histogram | Upstream response time |
| `mpl_policy_rejections_total` | Counter | Policy-based rejections |

---

## Environment Variable Overrides

All configuration fields can be overridden using environment variables. Environment variables take precedence over file-based configuration.

| Environment Variable | Config Field | Example |
|---|---|---|
| `MPL_LISTEN` | `transport.listen` | `MPL_LISTEN=0.0.0.0:9443` |
| `MPL_UPSTREAM` | `transport.upstream` | `MPL_UPSTREAM=localhost:8080` |
| `MPL_PROTOCOL` | `transport.protocol` | `MPL_PROTOCOL=websocket` |
| `MPL_REGISTRY` | `mpl.registry` | `MPL_REGISTRY=/opt/mpl/registry` |
| `MPL_MODE` | `mpl.mode` | `MPL_MODE=strict` |
| `MPL_REQUIRED_PROFILE` | `mpl.required_profile` | `MPL_REQUIRED_PROFILE=qom-strict` |
| `MPL_ENFORCE_SCHEMA` | `mpl.enforce_schema` | `MPL_ENFORCE_SCHEMA=true` |
| `MPL_ENFORCE_ASSERTIONS` | `mpl.enforce_assertions` | `MPL_ENFORCE_ASSERTIONS=false` |
| `MPL_POLICY_ENGINE` | `mpl.policy_engine` | `MPL_POLICY_ENGINE=true` |
| `MPL_METRICS_PORT` | `observability.metrics_port` | `MPL_METRICS_PORT=9100` |
| `MPL_METRICS_FORMAT` | `observability.metrics_format` | `MPL_METRICS_FORMAT=prometheus` |
| `MPL_LOGS` | `observability.logs` | `MPL_LOGS=file` |
| `MPL_LOG_FORMAT` | `observability.log_format` | `MPL_LOG_FORMAT=pretty` |
| `MPL_LOG_LEVEL` | `observability.log_level` | `MPL_LOG_LEVEL=debug` |

---

## Example Configurations

### Development

Permissive configuration for local development with verbose logging and pretty-printed output.

```yaml title="mpl-config.yaml (development)"
transport:
  listen: "127.0.0.1:9443"
  upstream: "localhost:8080"
  protocol: http

mpl:
  registry: "./registry"
  mode: development
  required_profile: qom-basic
  enforce_schema: true
  enforce_assertions: true
  policy_engine: false

observability:
  metrics_port: 9100
  metrics_format: prometheus
  logs: stdout
  log_format: pretty
  log_level: debug
```

Usage:

```bash
mpl proxy localhost:8080 --config mpl-config.yaml
```

---

### Staging

Moderate enforcement with structured logging, suitable for integration testing and pre-production validation.

```yaml title="mpl-config.yaml (staging)"
transport:
  listen: "0.0.0.0:9443"
  upstream: "mcp-server.staging.internal:8080"
  protocol: http

mpl:
  registry: "/opt/mpl/registry"
  mode: development
  required_profile: qom-basic
  enforce_schema: true
  enforce_assertions: true
  policy_engine: true

observability:
  metrics_port: 9100
  metrics_format: prometheus
  logs: stdout
  log_format: json
  log_level: info
```

---

### Production

Strict enforcement with full policy engine, JSON logging, and minimal log verbosity.

```yaml title="mpl-config.yaml (production)"
transport:
  listen: "0.0.0.0:9443"
  upstream: "mcp-server.prod.internal:8080"
  protocol: http

mpl:
  registry: "/opt/mpl/registry"
  mode: strict
  required_profile: qom-strict
  enforce_schema: true
  enforce_assertions: true
  policy_engine: true

observability:
  metrics_port: 9100
  metrics_format: prometheus
  logs: stdout
  log_format: json
  log_level: warn
```

Deploy with environment variable overrides:

```bash
export MPL_UPSTREAM=mcp-server.prod.internal:8080
export MPL_MODE=strict
export MPL_LOG_LEVEL=warn
mpl proxy $MPL_UPSTREAM --config /etc/mpl/mpl-config.yaml
```

---

### WebSocket Transport

Configuration for streaming MCP servers that use WebSocket connections.

```yaml title="mpl-config.yaml (websocket)"
transport:
  listen: "0.0.0.0:9443"
  upstream: "mcp-ws-server.internal:8080"
  protocol: websocket

mpl:
  registry: "./registry"
  mode: development
  required_profile: qom-basic
  enforce_schema: true
  enforce_assertions: false
  policy_engine: false

observability:
  metrics_port: 9100
  metrics_format: prometheus
  logs: stdout
  log_format: json
  log_level: info
```

---

## Configuration Validation

Validate your configuration file without starting the proxy:

```bash
mpl proxy localhost:8080 --config mpl-config.yaml --verbose 2>&1 | head -5
```

The proxy logs its resolved configuration at startup when `--verbose` is enabled:

```json
{
  "level": "debug",
  "msg": "resolved configuration",
  "transport.listen": "0.0.0.0:9443",
  "transport.upstream": "localhost:8080",
  "mpl.mode": "strict",
  "mpl.enforce_schema": true
}
```

---

## Precedence Order

Configuration values are resolved in the following order (highest priority first):

1. **CLI flags** (`--listen`, `--mode`, etc.)
2. **Environment variables** (`MPL_UPSTREAM`, `MPL_MODE`, etc.)
3. **Configuration file** (`mpl-config.yaml`)
4. **Built-in defaults**
