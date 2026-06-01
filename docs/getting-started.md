# Getting Started with MPL

This guide walks you through deploying MPL in under 10 minutes. You'll:
1. Start the MPL proxy
2. Define a semantic type (SType)
3. Make a typed API call
4. See schema validation in action

## Prerequisites

- Rust toolchain (for building from source) OR Docker
- An existing MCP/A2A server (or use our demo server)

## Quick Start with Docker

The fastest way to try MPL:

```bash
# Clone the repo
git clone https://github.com/Skelf-Research/mpl.git
cd mpl

# Start everything with Docker Compose
docker compose up -d

# This starts:
# - MPL proxy on port 9443
# - Demo MCP server on port 8080
# - Prometheus metrics on port 9100
```

Once running, test the setup:

```bash
# Check health
curl http://localhost:9443/health

# View capabilities
curl http://localhost:9443/capabilities
```

## Building from Source

```bash
# Clone and build
git clone https://github.com/Skelf-Research/mpl.git
cd mpl
cargo build --release

# Install CLI tools
cargo install --path crates/mplx
cargo install --path crates/mpl-proxy
```

## Step 1: Configure the Proxy

Create `mpl-config.yaml`:

```yaml
transport:
  listen: "0.0.0.0:9443"
  upstream: "localhost:8080"  # Your MCP/A2A server
  protocol: http

mpl:
  registry: "./registry"      # Local registry path
  mode: transparent           # transparent or strict
  required_profile: qom-basic
  enforce_schema: true
  enforce_assertions: true

observability:
  metrics_port: 9100
  log_format: json
  log_level: info
```

Start the proxy:

```bash
mpl-proxy --config mpl-config.yaml
```

## Step 2: Define Your First SType

Use the CLI to create a semantic type:

```bash
# Initialize a namespace (one-time setup)
mpl init --namespace org --registry ./registry

# Add a new SType
mpl add-stype \
  --namespace org \
  --domain calendar \
  --name Event \
  --version 1 \
  --registry ./registry
```

This creates:
```
registry/stypes/org/calendar/Event/v1/
├── schema.json      # JSON Schema
├── examples/        # Valid payload examples
├── negative/        # Invalid payloads (should fail)
├── README.md        # Documentation
└── CHANGELOG.md     # Version history
```

Edit `schema.json` to define your type:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "urn:stype:org.calendar.Event.v1",
  "title": "Calendar Event",
  "description": "A calendar event with required title and time",
  "type": "object",
  "properties": {
    "eventId": {
      "type": "string",
      "description": "Unique event identifier"
    },
    "title": {
      "type": "string",
      "minLength": 1,
      "description": "Event title"
    },
    "start": {
      "type": "string",
      "format": "date-time",
      "description": "Start time in ISO 8601 format"
    },
    "end": {
      "type": "string",
      "format": "date-time",
      "description": "End time in ISO 8601 format"
    },
    "description": {
      "type": "string",
      "description": "Optional event description"
    },
    "location": {
      "type": "string",
      "description": "Event location"
    },
    "attendees": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "email": { "type": "string", "format": "email" },
          "name": { "type": "string" },
          "status": { "enum": ["pending", "accepted", "declined"] }
        },
        "required": ["email"]
      }
    }
  },
  "required": ["title", "start", "end"]
}
```

Add an example in `examples/team_meeting.json`:

```json
{
  "title": "Weekly Team Standup",
  "start": "2024-01-15T09:00:00Z",
  "end": "2024-01-15T09:30:00Z",
  "description": "Regular team sync",
  "attendees": [
    {"email": "alice@example.com", "name": "Alice", "status": "accepted"}
  ]
}
```

## Step 3: Validate Your Schema

```bash
# Lint the registry structure
mpl lint --registry ./registry

# Run conformance tests
mpl conformance --registry ./registry --verbose

# Validate a payload
mpl validate \
  --stype org.calendar.Event.v1 \
  --payload '{"title": "Meeting", "start": "2024-01-15T10:00:00Z", "end": "2024-01-15T11:00:00Z"}' \
  --registry ./registry
```

Expected output:
```
Validation result: VALID
SType: org.calendar.Event.v1
Schema Fidelity: 1.0
```

## Step 4: Make a Typed API Call

With the proxy running, send requests through it:

```bash
# Send a request with SType header
curl -X POST http://localhost:9443/api/events \
  -H "Content-Type: application/json" \
  -H "X-MPL-SType: org.calendar.Event.v1" \
  -d '{
    "title": "Project Review",
    "start": "2024-01-20T14:00:00Z",
    "end": "2024-01-20T15:00:00Z"
  }'
```

The proxy will:
1. Validate the payload against the schema
2. Compute a semantic hash
3. Add MPL headers to the response
4. Log QoM metrics

Check the response headers:
```
X-MPL-SType: org.calendar.Event.v1
X-MPL-Schema-Fidelity: 1.0
X-MPL-Semantic-Hash: b3:a1b2c3...
```

## Step 5: Try Invalid Data

See validation in action with invalid data:

```bash
# Missing required field
curl -X POST http://localhost:9443/api/events \
  -H "Content-Type: application/json" \
  -H "X-MPL-SType: org.calendar.Event.v1" \
  -d '{"title": "Meeting"}'
```

In **transparent mode**, the request passes through with headers indicating failure:
```
X-MPL-Schema-Fidelity: 0.0
X-MPL-Validation-Error: Missing required property: start
```

In **strict mode**, invalid requests are rejected:
```json
{
  "error": "E-SCHEMA-FIDELITY",
  "message": "Payload does not match schema",
  "details": {
    "missing": ["start", "end"]
  }
}
```

## Step 6: View Metrics

The proxy exports Prometheus metrics:

```bash
curl http://localhost:9100/metrics
```

Key metrics:
- `mpl_requests_total` - Total requests by SType
- `mpl_schema_fidelity_pass_rate` - Schema validation success rate
- `mpl_qom_pass_rate` - QoM profile pass rate
- `mpl_validation_latency_seconds` - Validation latency histogram

## Using the Python SDK

For programmatic access:

```python
from mpl_sdk import Session, SessionConfig

async with Session(SessionConfig(
    endpoint="ws://localhost:9443/ws",
    stypes=["org.calendar.Event.v1"],
    qom_profile="qom-basic",
    registry_path="./registry",
)) as session:
    # Send typed request
    response = await session.send(
        stype="org.calendar.Event.v1",
        payload={
            "title": "Team Sync",
            "start": "2024-01-15T10:00:00Z",
            "end": "2024-01-15T10:30:00Z",
        }
    )
    print(f"Response: {response.payload}")
    print(f"Schema Fidelity: valid")
```

## Next Steps

- **Add more STypes**: See [Registry Architecture](registry-architecture.md)
- **Configure QoM profiles**: See [QoM Evaluation Engine](qom-evaluation-engine.md)
- **Integration patterns**: See [Integration Modes](integration-modes.md)
- **MCP integration**: See [MPL with MCP](mpl-with-mcp.md)
- **A2A integration**: See [MPL with A2A](mpl-with-a2a.md)

## Troubleshooting

### Proxy won't start
- Check that the upstream server is running
- Verify the listen port is available
- Check logs: `mpl-proxy --verbose`

### Schema validation fails unexpectedly
- Run `mpl lint` to check schema syntax
- Verify JSON is valid with `jq`
- Check field types match schema

### Can't find SType
- Verify registry path is correct
- Check SType naming: `namespace.domain.Name.vN`
- Run `mpl conformance` to test registry

## Kubernetes Deployment

For production deployments, use the Helm chart:

```bash
# Add the MPL Helm repository
helm repo add mpl https://mpl-charts.example.com
helm repo update

# Install with custom values
helm install mpl-proxy mpl/mpl-proxy \
  --namespace mpl-system \
  --create-namespace \
  --set registry.endpoint=http://registry:8080 \
  --set qom.defaultProfile=qom-strict-argcheck \
  --set proxy.mode=strict \
  --set monitoring.prometheus.enabled=true
```

### Helm Values

Key configuration options in `values.yaml`:

```yaml
proxy:
  mode: strict               # strict | transparent
  replicas: 3
  resources:
    requests:
      memory: "128Mi"
      cpu: "100m"
    limits:
      memory: "512Mi"
      cpu: "500m"

qom:
  defaultProfile: qom-strict-argcheck
  enforceSchema: true
  enforceAssertions: true

registry:
  endpoint: http://registry:8080
  cacheTTL: 300s

monitoring:
  prometheus:
    enabled: true
    port: 9100
  grafana:
    dashboards: true

security:
  tls:
    enabled: true
    certSecret: mpl-tls-cert
```

## Operational Runbook

### Health Checks

```bash
# Liveness probe
curl http://localhost:9443/health/live

# Readiness probe
curl http://localhost:9443/health/ready

# Full status
curl http://localhost:9443/status
```

### Common Issues

| Issue | Diagnosis | Resolution |
|-------|-----------|------------|
| High latency | Check `mpl_validation_latency_seconds` | Scale proxy replicas |
| Schema failures | Check `mpl_schema_fidelity_pass_rate` | Review payload against schema |
| Registry errors | Check `mpl_registry_errors_total` | Verify registry connectivity |
| QoM breaches | Check `mpl_qom_breach_total` | Review IC assertions |

### Alerting Rules

Example Prometheus alerting rules:

```yaml
groups:
  - name: mpl
    rules:
      - alert: MPLSchemaFidelityLow
        expr: mpl_schema_fidelity_pass_rate < 0.99
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Schema validation pass rate below 99%"

      - alert: MPLQoMBreachHigh
        expr: rate(mpl_qom_breach_total[5m]) > 0.1
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "QoM breach rate exceeds threshold"
```

### Log Analysis

MPL logs are structured JSON. Key fields:

```json
{
  "level": "info",
  "timestamp": "2025-01-15T10:30:00Z",
  "stype": "org.calendar.Event.v1",
  "schema_fidelity": 1.0,
  "qom_profile": "qom-basic",
  "qom_passed": true,
  "latency_ms": 12,
  "semantic_hash": "b3:a1b2c3..."
}
```

Filter logs by SType:
```bash
kubectl logs -l app=mpl-proxy | jq 'select(.stype | startswith("org.finance"))'
```

## Quick Reference

| Command | Description |
|---------|-------------|
| `mpl init` | Initialize registry namespace |
| `mpl add-stype` | Create new semantic type |
| `mpl validate` | Validate payload against schema |
| `mpl lint` | Check registry structure |
| `mpl conformance` | Run all schema tests |
| `mpl hash` | Compute semantic hash |
| `mpl-proxy` | Start the sidecar proxy |

## SDK Quick Reference

### Python

```bash
pip install mpl-sdk
```

```python
from mpl import MplClient, QomProfile

# Initialize client
client = MplClient("http://localhost:9443")

# Negotiate capabilities
await client.negotiate(
    stypes=["org.calendar.Event.v1"],
    profile=QomProfile.STRICT
)

# Validate and send
result = await client.validate(
    stype="org.calendar.Event.v1",
    payload={"title": "Meeting", "start": "2025-01-15T10:00:00Z", "end": "2025-01-15T11:00:00Z"}
)
```

### TypeScript

```bash
npm install @mpl/sdk
```

```typescript
import { MplClient, QomProfile } from '@mpl/sdk';

const client = new MplClient('http://localhost:9443');

await client.negotiate({
  stypes: ['org.calendar.Event.v1'],
  profile: QomProfile.Strict
});

const result = await client.validate({
  stype: 'org.calendar.Event.v1',
  payload: { title: 'Meeting', start: '2025-01-15T10:00:00Z', end: '2025-01-15T11:00:00Z' }
});
```

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         Client Application                       │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                         MPL Proxy                                │
│  ┌───────────┐  ┌────────────┐  ┌──────────────────────────┐   │
│  │ Handshake │  │ Validation │  │      QoM Evaluation      │   │
│  │ (AI-ALPN) │  │  (Schema)  │  │ (SF, IC, TOC, G, DJ, OA) │   │
│  └───────────┘  └────────────┘  └──────────────────────────┘   │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │                    Policy Engine                          │ │
│  └───────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                    MCP Server / A2A Peer                         │
└─────────────────────────────────────────────────────────────────┘
```

## Support

- **Documentation**: [docs/](../)
- **Issues**: GitHub Issues
- **Discussion**: GitHub Discussions
