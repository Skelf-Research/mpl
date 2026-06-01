# MPL Configuration Reference

This document provides a complete reference for configuring MPL components and understanding how the registry integrates with the system.

## Table of Contents

1. [Configuration Overview](#configuration-overview)
2. [Proxy Configuration](#proxy-configuration)
3. [Registry Configuration](#registry-configuration)
4. [QoM Configuration](#qom-configuration)
5. [File Reference](#file-reference)
6. [Environment Variables](#environment-variables)
7. [Complete Examples](#complete-examples)

---

## Configuration Overview

MPL uses a layered configuration system:

```
┌─────────────────────────────────────────────────────────────┐
│                    Configuration Layers                      │
├─────────────────────────────────────────────────────────────┤
│  CLI Arguments (highest priority)                            │
│    --listen, --upstream, --profile, --registry              │
├─────────────────────────────────────────────────────────────┤
│  Environment Variables                                       │
│    MPL_LISTEN, MPL_UPSTREAM, MPL_PROFILE, MPL_REGISTRY      │
├─────────────────────────────────────────────────────────────┤
│  Configuration File (mpl-config.yaml)                        │
│    transport, mpl, observability, routing, limits           │
├─────────────────────────────────────────────────────────────┤
│  Defaults (lowest priority)                                  │
│    Built-in sensible defaults for all options               │
└─────────────────────────────────────────────────────────────┘
```

### Configuration Files

| File | Purpose | Location |
|------|---------|----------|
| `mpl-config.yaml` | Proxy configuration | Project root or `--config` path |
| `registry/` | SType definitions, assertions, ontologies | `mpl.registry` path |
| `.mpl/` | Runtime data (traffic, QoM events) | `~/.mpl` or configured path |

---

## Proxy Configuration

The proxy configuration file (`mpl-config.yaml`) controls all aspects of the MPL sidecar proxy.

### Complete Configuration Schema

```yaml
# mpl-config.yaml - Complete configuration reference

#==============================================================================
# TRANSPORT CONFIGURATION
# Controls network settings for the proxy
#==============================================================================
transport:
  # Address to listen on (format: "host:port")
  listen: "0.0.0.0:9443"

  # Upstream server address (format: "host:port")
  upstream: "localhost:8080"

  # Protocol type: http, websocket, grpc
  protocol: http

  # Connection timeout in milliseconds (default: 5000)
  connect_timeout_ms: 5000

  # Request timeout in milliseconds (default: 30000)
  request_timeout_ms: 30000

  # Idle connection timeout in milliseconds (default: 60000)
  idle_timeout_ms: 60000

  # Maximum retries for transient failures (default: 3)
  max_retries: 3

  # Maximum request body size in bytes (default: 10485760 = 10MB)
  max_body_size: 10485760

#==============================================================================
# MPL CONFIGURATION
# Controls validation, QoM profiles, and registry settings
#==============================================================================
mpl:
  # Path to the registry (local path or URL)
  # Local: "./registry" or "/etc/mpl/registry"
  # Remote: "https://registry.mpl.dev"
  registry: "./registry"

  # Proxy mode:
  #   transparent - Log validation failures but forward all requests
  #   strict      - Block requests that fail validation
  mode: transparent

  # Required QoM profile (null = no profile enforcement)
  # Built-in: qom-basic, qom-strict-argcheck, qom-outcome, qom-comprehensive
  required_profile: qom-basic

  # Enable JSON Schema validation (default: true)
  enforce_schema: true

  # Enable CEL assertion checks (default: true)
  enforce_assertions: true

  # Enable policy engine (default: false)
  policy_engine: false

#==============================================================================
# OBSERVABILITY CONFIGURATION
# Controls metrics, logging, and tracing
#==============================================================================
observability:
  # Prometheus metrics port (null = disabled)
  metrics_port: 9100

  # Metrics format: prometheus, opentelemetry
  metrics_format: prometheus

  # Log output: stdout, stderr, file
  logs: stdout

  # Log format: json, text
  log_format: json

  # Log level: trace, debug, info, warn, error
  log_level: info

#==============================================================================
# ROUTING CONFIGURATION
# SType-based request routing to different upstreams
#==============================================================================
routing:
  # Route calendar STypes to calendar service
  - stype_pattern: "org.calendar.*"
    upstream: "calendar-service:8080"

  # Route finance STypes to finance service
  - stype_pattern: "org.finance.*"
    upstream: "finance-service:8080"

  # Route healthcare STypes to HIPAA-compliant service
  - stype_pattern: "org.healthcare.*"
    upstream: "healthcare-service:8443"

#==============================================================================
# RESOURCE LIMITS
# Controls rate limiting and circuit breaker settings
#==============================================================================
limits:
  # Maximum concurrent connections (default: 10000)
  max_connections: 10000

  # Rate limit per second per client IP (default: 100)
  rate_limit_per_second: 100

  # Burst size for rate limiting (default: 50)
  burst_size: 50

  # Maximum pending requests in queue (default: 1000)
  max_pending_requests: 1000

  # Circuit breaker: failures before opening (default: 5)
  failure_threshold: 5

  # Circuit breaker: recovery time in ms (default: 30000)
  recovery_time_ms: 30000
```

### Mode Comparison

| Feature | Transparent Mode | Strict Mode |
|---------|------------------|-------------|
| Invalid requests | Forwarded with error headers | Rejected with 400 response |
| QoM failures | Logged only | Request blocked |
| Unknown STypes | Forwarded | Rejected |
| Response headers | Always added | Always added |
| Use case | Development, migration | Production |

---

## Registry Configuration

The registry is a directory structure containing SType definitions, assertions, and ontologies.

### Registry Structure

```
registry/
├── stypes/                          # Semantic Type definitions
│   ├── {namespace}/                 # e.g., org, eval, data, ai
│   │   └── {domain}/                # e.g., calendar, finance, agent
│   │       └── {TypeName}/          # e.g., Event, Rating, ToolInvocation
│   │           └── v{N}/            # e.g., v1, v2
│   │               ├── schema.json       # JSON Schema (REQUIRED)
│   │               ├── assertions.json   # CEL assertions (optional)
│   │               ├── ontology.json     # Domain constraints (optional)
│   │               ├── examples/         # Valid payload examples
│   │               │   └── *.json
│   │               ├── negative/         # Invalid payloads (should fail)
│   │               │   └── *.json
│   │               ├── README.md         # Documentation
│   │               └── CHANGELOG.md      # Version history
├── tools/                           # Tool descriptors
│   └── tool.{name}.v{N}.json
├── profiles/                        # QoM profile definitions
│   └── *.json
├── policies/                        # Policy rules (Rego)
│   └── {policy-name}/v{N}/
│       └── policy.rego
└── CODEOWNERS                       # Namespace ownership
```

### SType Naming Convention

```
{namespace}.{domain}.{TypeName}.v{version}
```

Examples:
- `org.calendar.Event.v1`
- `org.finance.InvestmentRecommendation.v1`
- `eval.rag.RAGResponse.v1`
- `ai.completion.Response.v1`

### Required Files

| File | Required | Purpose |
|------|----------|---------|
| `schema.json` | Yes | JSON Schema definition |
| `assertions.json` | No | CEL business rules |
| `ontology.json` | No | Domain constraints |
| `examples/*.json` | Recommended | Positive test cases |
| `negative/*.json` | Recommended | Negative test cases |

### schema.json Format

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "urn:stype:org.calendar.Event.v1",
  "title": "Calendar Event",
  "description": "A calendar event with start and end times",
  "type": "object",
  "properties": {
    "eventId": { "type": "string" },
    "title": { "type": "string", "minLength": 1 },
    "start": { "type": "string", "format": "date-time" },
    "end": { "type": "string", "format": "date-time" }
  },
  "required": ["title", "start", "end"]
}
```

### assertions.json Format

```json
{
  "$schema": "https://mpl.dev/schemas/assertions.json",
  "stype": "org.calendar.Event.v1",
  "name": "event_validations",
  "description": "Business logic assertions for calendar events",
  "assertions": [
    {
      "id": "end_after_start",
      "expression": "payload.end > payload.start",
      "message": "Event end time must be after start time",
      "severity": "error",
      "tags": ["required", "temporal"]
    },
    {
      "id": "title_meaningful",
      "expression": "size(payload.title) >= 3",
      "message": "Event title should be at least 3 characters",
      "severity": "warning",
      "tags": ["quality"]
    }
  ],
  "fail_fast": false
}
```

### ontology.json Format

```json
{
  "$schema": "https://mpl.dev/schemas/ontology.json",
  "name": "event_ontology",
  "description": "Domain constraints for calendar events",
  "allowed_values": {
    "status": ["draft", "confirmed", "cancelled"]
  },
  "relationships": [
    {
      "id": "confirmed_has_attendees",
      "from": "status",
      "to": "attendees",
      "relation_type": "implies",
      "condition": "status == 'confirmed'",
      "message": "Confirmed events should have attendees"
    }
  ],
  "type_constraints": {
    "organizer_email": {
      "semantic_type": "email",
      "message": "Organizer must be a valid email"
    }
  }
}
```

---

## QoM Configuration

### Built-in Profiles

| Profile | SF | IC | TOC | G | DJ | OA | Use Case |
|---------|----|----|-----|---|----|----|----------|
| `qom-basic` | 1.0 | - | - | - | - | - | Development |
| `qom-strict-argcheck` | 1.0 | 0.97 | - | - | - | - | Production |
| `qom-outcome` | 1.0 | - | 0.9 | - | - | - | Tool-using agents |
| `qom-comprehensive` | 1.0 | 0.95 | 0.9 | 0.8 | 0.9 | 0.95 | High-stakes |

### Profile Degradation Chain

When a profile fails, the proxy can automatically degrade to a less strict profile:

```
qom-comprehensive → qom-outcome → qom-strict-argcheck → qom-basic
```

### Custom Profile Definition

```json
{
  "name": "my-custom-profile",
  "description": "Custom profile for my application",
  "metrics": {
    "schema_fidelity": { "min": 1.0 },
    "instruction_compliance": { "min": 0.90 },
    "tool_outcome_correctness": { "min": 0.85 },
    "groundedness": { "min": 0.70 },
    "determinism_jitter": { "min": 0.80 },
    "ontology_adherence": { "min": 0.90 }
  },
  "retry_policy": {
    "max_retries": 2,
    "degrade_to": "qom-basic",
    "on_failure": "log"
  }
}
```

---

## File Reference

### Files Created by MPL

| Path | Purpose | Created By |
|------|---------|------------|
| `~/.mpl/traffic/` | Recorded traffic samples | Proxy (learning mode) |
| `~/.mpl/schemas/` | Inferred schemas | Proxy (learning mode) |
| `~/.mpl/qom/qom_events.jsonl` | QoM evaluation events | Proxy |
| `~/.mpl/qom/qom_history.json` | QoM trend history | Proxy |

### Files Loaded by MPL

| Path Pattern | Purpose | Loader |
|--------------|---------|--------|
| `registry/stypes/**/schema.json` | JSON Schemas | Proxy, CLI |
| `registry/stypes/**/assertions.json` | CEL assertions | Proxy |
| `registry/stypes/**/ontology.json` | Domain constraints | Proxy |
| `registry/profiles/*.json` | QoM profiles | Proxy |

---

## Environment Variables

All configuration options can be set via environment variables:

| Variable | Config Path | Example |
|----------|-------------|---------|
| `MPL_LISTEN` | `transport.listen` | `0.0.0.0:9443` |
| `MPL_UPSTREAM` | `transport.upstream` | `localhost:8080` |
| `MPL_REGISTRY` | `mpl.registry` | `./registry` |
| `MPL_MODE` | `mpl.mode` | `strict` |
| `MPL_PROFILE` | `mpl.required_profile` | `qom-strict-argcheck` |
| `MPL_ENFORCE_SCHEMA` | `mpl.enforce_schema` | `true` |
| `MPL_ENFORCE_ASSERTIONS` | `mpl.enforce_assertions` | `true` |
| `MPL_LOG_LEVEL` | `observability.log_level` | `debug` |
| `MPL_LOG_FORMAT` | `observability.log_format` | `json` |
| `MPL_METRICS_PORT` | `observability.metrics_port` | `9100` |

---

## Complete Examples

### Development Configuration

```yaml
# mpl-config.dev.yaml
transport:
  listen: "127.0.0.1:9443"
  upstream: "localhost:8080"
  protocol: http

mpl:
  registry: "./registry"
  mode: transparent           # Don't block, just log
  required_profile: qom-basic # Minimal validation
  enforce_schema: true
  enforce_assertions: false   # Skip during dev

observability:
  metrics_port: null          # Disable metrics
  log_format: text            # Human-readable logs
  log_level: debug            # Verbose logging
```

### Production Configuration

```yaml
# mpl-config.prod.yaml
transport:
  listen: "0.0.0.0:9443"
  upstream: "app-service:8080"
  protocol: http
  connect_timeout_ms: 3000
  request_timeout_ms: 10000
  max_retries: 2

mpl:
  registry: "https://registry.mycompany.com"
  mode: strict                        # Block invalid requests
  required_profile: qom-strict-argcheck
  enforce_schema: true
  enforce_assertions: true
  policy_engine: true

observability:
  metrics_port: 9100
  metrics_format: prometheus
  log_format: json
  log_level: info

limits:
  max_connections: 50000
  rate_limit_per_second: 1000
  burst_size: 200
  failure_threshold: 10
  recovery_time_ms: 60000
```

### Healthcare Configuration (HIPAA)

```yaml
# mpl-config.healthcare.yaml
transport:
  listen: "0.0.0.0:9443"
  upstream: "healthcare-backend:8443"
  protocol: http
  max_body_size: 52428800     # 50MB for medical images

mpl:
  registry: "/etc/mpl/registry"
  mode: strict
  required_profile: qom-comprehensive
  enforce_schema: true
  enforce_assertions: true
  policy_engine: true

observability:
  metrics_port: 9100
  log_format: json
  log_level: info

routing:
  - stype_pattern: "org.healthcare.*"
    upstream: "hipaa-service:8443"

limits:
  rate_limit_per_second: 50   # Lower for safety
  failure_threshold: 3        # Quick circuit break
```

### Multi-Service Configuration

```yaml
# mpl-config.multi.yaml
transport:
  listen: "0.0.0.0:9443"
  upstream: "default-service:8080"
  protocol: http

mpl:
  registry: "./registry"
  mode: strict
  required_profile: qom-strict-argcheck

routing:
  # Calendar team's service
  - stype_pattern: "org.calendar.*"
    upstream: "calendar-service:8080"

  # Finance team's service (higher security)
  - stype_pattern: "org.finance.*"
    upstream: "finance-service:8443"

  # AI/ML team's service
  - stype_pattern: "ai.*"
    upstream: "ml-service:8080"

  # Evaluation pipeline
  - stype_pattern: "eval.*"
    upstream: "eval-service:8080"
```

---

## CLI Reference

### Proxy Commands

```bash
# Start with config file
mpl-proxy --config mpl-config.yaml

# Override with CLI args
mpl-proxy --config mpl-config.yaml \
  --listen 0.0.0.0:9443 \
  --upstream myserver:8080 \
  --registry ./my-registry

# Enable verbose logging
mpl-proxy --config mpl-config.yaml --verbose
```

### CLI Commands

```bash
# Initialize registry namespace
mpl init --namespace mycompany --registry ./registry

# Add new SType
mpl add-stype \
  --namespace mycompany \
  --domain calendar \
  --name Event \
  --version 1 \
  --registry ./registry

# Validate payload
mpl validate \
  --stype mycompany.calendar.Event.v1 \
  --payload '{"title": "Meeting", "start": "2025-01-15T10:00:00Z", "end": "2025-01-15T11:00:00Z"}' \
  --registry ./registry

# Lint registry structure
mpl lint --registry ./registry

# Run conformance tests
mpl conformance --registry ./registry --verbose

# Compute semantic hash
mpl hash --payload '{"title": "Meeting"}'

# Start web UI
mpl ui --port 8080 --data-dir ~/.mpl
```

---

## Troubleshooting

### Common Issues

| Issue | Cause | Solution |
|-------|-------|----------|
| "Unknown SType" | SType not in registry | Check registry path, verify SType exists |
| Schema validation fails | Payload doesn't match schema | Use `mpl validate` to debug |
| Assertions not loading | Invalid assertions.json | Check JSON syntax, verify CEL expressions |
| QoM always passes | Profile not configured | Set `required_profile` in config |
| Metrics not exposed | Port conflict or disabled | Check `metrics_port` setting |

### Debug Commands

```bash
# Check proxy health
curl http://localhost:9443/health

# View loaded capabilities
curl http://localhost:9443/capabilities

# Check QoM metrics
curl http://localhost:9443/_mpl/qom

# View recent events
curl http://localhost:9443/_mpl/qom/events?limit=10

# Test specific SType validation
mpl validate \
  --stype org.calendar.Event.v1 \
  --payload @event.json \
  --registry ./registry \
  --verbose
```

---

## See Also

- [Getting Started](getting-started.md) - Quick start guide
- [QoM Guide](qom-guide.md) - Quality of Meaning metrics
- [Registry Architecture](registry-architecture.md) - Registry design
- [Integration Modes](integration-modes.md) - Deployment patterns
