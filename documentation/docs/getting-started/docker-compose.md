# Docker Compose

Deploy the full MPL stack locally with Docker Compose. This includes the proxy, a demo MCP server, and optional monitoring.

---

## Quick Deploy

```bash
git clone https://github.com/Skelf-Research/mpl.git
cd mpl

# Start core services (proxy + demo server)
docker compose up -d

# Verify
curl http://localhost:9443/health
```

---

## Services

The `docker-compose.yaml` includes:

| Service | Port | Description |
|---------|------|-------------|
| `mpl-proxy` | 9443, 9100 | MPL sidecar proxy with metrics |
| `demo-server` | 8080 | Demo MCP server (echo server for testing) |
| `prometheus` | 9090 | Metrics collection (optional, `monitoring` profile) |
| `grafana` | 3000 | Dashboards (optional, `monitoring` profile) |

---

## Core Stack

The default `docker compose up` starts the proxy and demo server:

```yaml
services:
  mpl-proxy:
    build: .
    ports:
      - "9443:9443"   # Proxy endpoint
      - "9100:9100"   # Prometheus metrics
    volumes:
      - ./registry:/app/registry:ro
      - ./mpl-config.yaml:/app/mpl-config.yaml:ro
    environment:
      - RUST_LOG=info

  demo-server:
    image: python:3.11-slim
    working_dir: /app
    volumes:
      - ./examples/demo-server:/app
    command: python server.py
    ports:
      - "8080:8080"
```

---

## With Monitoring

To include Prometheus and Grafana:

```bash
docker compose --profile monitoring up -d
```

This adds:

- **Prometheus** at http://localhost:9090 — scrapes MPL metrics every 15s
- **Grafana** at http://localhost:3000 — visualize metrics (login: admin/admin)

---

## Configuration

The proxy reads from `mpl-config.yaml`:

```yaml
transport:
  listen: "0.0.0.0:9443"
  upstream: "demo-server:8080"
  protocol: http

mpl:
  registry: "./registry"
  mode: transparent          # transparent | strict
  required_profile: qom-basic
  enforce_schema: true
  enforce_assertions: true

observability:
  metrics_port: 9100
  metrics_format: prometheus
  logs: stdout
  log_format: json
  log_level: info
```

!!! tip "Switching to Strict Mode"
    Change `mode: transparent` to `mode: strict` and restart to enforce validation. In strict mode, invalid payloads are rejected with typed errors.

---

## Testing the Stack

Once running, test validation through the proxy:

```bash
# Health check
curl http://localhost:9443/health

# Check capabilities
curl http://localhost:9443/capabilities

# Validate a payload
curl -X POST http://localhost:9443/validate \
  -H "Content-Type: application/json" \
  -d '{
    "stype": "org.calendar.Event.v1",
    "payload": {
      "title": "Meeting",
      "start": "2025-01-15T10:00:00Z",
      "end": "2025-01-15T11:00:00Z"
    }
  }'
```

---

## Customizing

### Point to Your Own Server

Replace the demo server with your MCP/A2A server by editing `mpl-config.yaml`:

```yaml
transport:
  upstream: "your-mcp-server:8080"
```

Or override via environment variable:

```bash
MPL_UPSTREAM=http://your-server:8080 docker compose up -d
```

### Add Your Own STypes

Mount your custom registry alongside the default:

```yaml
volumes:
  - ./my-registry:/app/registry:ro
```

### Resource Limits

For production-like testing, add resource constraints:

```yaml
services:
  mpl-proxy:
    deploy:
      resources:
        limits:
          cpus: '0.5'
          memory: 256M
```

---

## Stopping

```bash
# Stop all services
docker compose down

# Stop and remove volumes
docker compose down -v
```

---

## Next Steps

- [First Validation](first-validation.md) — Validate payloads through the proxy
- [Configuration Reference](../reference/configuration.md) — Full config options
- [Monitoring](../guides/operations/monitoring.md) — Production monitoring setup
