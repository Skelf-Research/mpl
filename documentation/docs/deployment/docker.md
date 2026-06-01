---
title: Docker Deployment
description: Build, run, and orchestrate the MPL proxy using Docker and Docker Compose
---

# Docker Deployment

This guide covers building the MPL proxy Docker image, running it as a standalone container, and orchestrating a full stack with Docker Compose.

---

## Dockerfile Overview

MPL uses a multi-stage build to produce a minimal runtime image:

```dockerfile title="Dockerfile"
# Stage 1: Build
FROM rust:1.75-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --release --package mpl-proxy --package mpl-cli

# Stage 2: Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/mpl-proxy /usr/local/bin/
COPY --from=builder /app/target/release/mpl-cli /usr/local/bin/
COPY registry /app/registry
COPY mpl-config.yaml /app/mpl-config.yaml

EXPOSE 9443 9100
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:9443/health || exit 1

ENTRYPOINT ["mpl-proxy"]
CMD ["--config", "/app/mpl-config.yaml"]
```

!!! info "Image Layers"
    - **Builder stage**: Uses the official Rust image with full build toolchain (~2 GB, discarded)
    - **Runtime stage**: Based on `debian:bookworm-slim` (~80 MB), contains only the compiled binaries, registry schemas, and config

---

## Building the Image

```bash
docker build -t mpl-proxy .
```

To tag with a version:

```bash
docker build -t mpl-proxy:0.1.0 -t mpl-proxy:latest .
```

!!! tip "Build Cache"
    The Dockerfile copies `Cargo.toml` and `Cargo.lock` before source code, so dependency builds are cached across source changes. Rebuilds that only modify application code are significantly faster.

---

## Running the Container

### Basic Run

```bash
docker run -d \
  --name mpl-proxy \
  -p 9443:9443 \
  -p 9100:9100 \
  -p 9080:9080 \
  mpl-proxy:latest
```

### With Volume Mounts

Mount your local registry and config to override defaults:

```bash
docker run -d \
  --name mpl-proxy \
  -p 9443:9443 \
  -p 9100:9100 \
  -p 9080:9080 \
  -v $(pwd)/registry:/app/registry:ro \
  -v $(pwd)/mpl-config.yaml:/app/mpl-config.yaml:ro \
  mpl-proxy:latest
```

### With Environment Variables

```bash
docker run -d \
  --name mpl-proxy \
  -p 9443:9443 \
  -p 9100:9100 \
  -p 9080:9080 \
  -e RUST_LOG=debug \
  -e MPL_MODE=strict \
  -e MPL_UPSTREAM=http://host.docker.internal:8080 \
  mpl-proxy:latest
```

---

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_LOG` | `info` | Log level filter (`trace`, `debug`, `info`, `warn`, `error`) |
| `MPL_MODE` | `transparent` | Proxy mode: `transparent` (log only) or `strict` (block invalid) |
| `MPL_UPSTREAM` | from config | Upstream server URL |
| `MPL_REGISTRY` | `./registry` | Path to the SType registry directory |
| `MPL_METRICS_PORT` | `9100` | Prometheus metrics endpoint port |
| `MPL_PROFILE` | `qom-basic` | Required QoM profile for validation |
| `MPL_LISTEN` | `0.0.0.0:9443` | Proxy listen address and port |

!!! note "Precedence"
    Environment variables override values in `mpl-config.yaml`. This allows the same image to be used across environments with config injected at runtime.

---

## Volume Mounts

| Mount Path | Purpose | Mode |
|------------|---------|------|
| `/app/registry` | SType schemas, QoM profiles, assertion libraries | Read-only (`:ro`) |
| `/app/mpl-config.yaml` | Proxy configuration file | Read-only (`:ro`) |
| `/tmp` | Temporary scratch space (if needed) | Read-write |

!!! warning "Read-Only Root Filesystem"
    When running with `--read-only`, ensure `/tmp` is mounted as a writable volume or tmpfs:
    ```bash
    docker run --read-only --tmpfs /tmp mpl-proxy:latest
    ```

---

## Health Checks

The proxy exposes a health endpoint for liveness and readiness checks:

```bash
curl http://localhost:9443/health
```

Expected response:

```json
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime_seconds": 142
}
```

The built-in Docker `HEALTHCHECK` polls this endpoint every 30 seconds with a 5-second startup grace period.

---

## Docker Compose

For local development and staging, use Docker Compose to run the full stack:

```yaml title="docker-compose.yaml"
services:
  mpl-proxy:
    build: .
    ports:
      - "9443:9443"
      - "9100:9100"
      - "9080:9080"
    volumes:
      - ./registry:/app/registry:ro
      - ./mpl-config.yaml:/app/mpl-config.yaml:ro
    environment:
      RUST_LOG: info
      MPL_MODE: transparent
    depends_on:
      demo-server:
        condition: service_healthy
    networks:
      - mpl-network
    deploy:
      resources:
        limits:
          cpus: "0.5"
          memory: 256M
        reservations:
          cpus: "0.1"
          memory: 128M

  demo-server:
    build:
      context: ./demo
    ports:
      - "8080:8080"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 10s
      timeout: 3s
      retries: 3
    networks:
      - mpl-network

networks:
  mpl-network:
    driver: bridge
```

Start the stack:

```bash
docker compose up -d
```

View logs:

```bash
docker compose logs -f mpl-proxy
```

Stop and clean up:

```bash
docker compose down -v
```

---

## Resource Recommendations

| Resource | Development | Staging | Production |
|----------|:-----------:|:-------:|:----------:|
| **CPU limit** | 250m | 500m | 500m--1000m |
| **Memory limit** | 128Mi | 256Mi | 256Mi--512Mi |
| **CPU request** | 50m | 100m | 100m--250m |
| **Memory request** | 64Mi | 128Mi | 128Mi--256Mi |

!!! tip "Right-Sizing"
    The MPL proxy is lightweight. Under typical loads (< 1000 req/s), 500m CPU and 256Mi memory are sufficient. Monitor metrics at `:9100/metrics` to adjust based on actual usage.

---

## Docker Networking

### Bridge Network (Default)

Containers on the same Docker network can reach each other by service name:

```yaml
# mpl-config.yaml for Docker Compose
transport:
  listen: "0.0.0.0:9443"
  upstream: "demo-server:8080"  # resolved via Docker DNS
```

### Host Network

For development, you can use host networking to access services running directly on the host:

```bash
docker run --network host \
  -e MPL_UPSTREAM=http://localhost:8080 \
  mpl-proxy:latest
```

### Accessing Host Services

When the upstream server runs on the host (not in Docker), use the special DNS name:

```bash
docker run -p 9443:9443 \
  -e MPL_UPSTREAM=http://host.docker.internal:8080 \
  mpl-proxy:latest
```

!!! note "Platform Support"
    `host.docker.internal` is supported on Docker Desktop (macOS, Windows) and Docker Engine 20.10+ on Linux with `--add-host=host.docker.internal:host-gateway`.

---

## Troubleshooting

??? question "Container exits immediately"
    Check the logs for configuration errors:
    ```bash
    docker logs mpl-proxy
    ```
    Common causes: missing config file, invalid upstream URL, port conflicts.

??? question "Cannot reach upstream service"
    Verify network connectivity:
    ```bash
    docker exec mpl-proxy curl -s http://upstream-host:8080/health
    ```
    Ensure both containers are on the same Docker network.

??? question "Health check failing"
    Allow sufficient startup time. The default `start-period` is 5 seconds. For slower systems:
    ```bash
    docker run --health-start-period=15s mpl-proxy:latest
    ```

---

## Next Steps

- [Kubernetes & Helm](kubernetes.md) -- Deploy to production with the Helm chart
- [Production Checklist](production-checklist.md) -- Verify readiness before going live
- [Monitoring & Metrics](../guides/operations/monitoring.md) -- Set up Prometheus and Grafana dashboards
