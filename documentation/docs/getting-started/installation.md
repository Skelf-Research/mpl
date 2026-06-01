# Installation

## MPL CLI & Proxy

### From Cargo (Recommended)

```bash
cargo install mplx
```

This installs the `mpl` binary which includes the proxy, schema tools, and all CLI commands.

!!! info "Rust Toolchain"
    Requires Rust 1.75 or later. Install from [rustup.rs](https://rustup.rs/).

### From Docker

```bash
docker pull ghcr.io/skelf-research/mpl-proxy:latest
```

Run the proxy:

```bash
docker run -p 9443:9443 -p 9100:9100 \
  -v ./registry:/app/registry:ro \
  ghcr.io/skelf-research/mpl-proxy:latest \
  --upstream http://host.docker.internal:8080
```

### From Source

```bash
git clone https://github.com/Skelf-Research/mpl.git
cd mpl
cargo build --release

# Binary at target/release/mpl
```

---

## Python SDK

```bash
pip install mpl-sdk
```

Or with development dependencies:

```bash
pip install mpl-sdk[dev]
```

!!! info "Requirements"
    - Python 3.10 or later
    - Dependencies: `aiohttp>=3.9.0`, `websockets>=12.0`

### Verify Installation

```python
import mpl_sdk
print(mpl_sdk.__version__)
```

---

## TypeScript SDK

```bash
npm install @mpl/sdk
```

Or with yarn/pnpm:

```bash
yarn add @mpl/sdk
pnpm add @mpl/sdk
```

!!! info "Requirements"
    - Node.js 18 or later
    - TypeScript 5.3+ (for type definitions)

### Verify Installation

```typescript
import { MplClient } from '@mpl/sdk';
console.log('MPL SDK loaded');
```

---

## Verify the Proxy

After installing, verify the proxy works:

```bash
# Start with a test upstream (or your own MCP server)
mpl proxy http://localhost:8080

# Check health
curl http://localhost:9443/health
# Expected: {"status": "healthy"}

# Check capabilities
curl http://localhost:9443/capabilities
```

---

## What's Installed

| Component | Description | Port |
|-----------|-------------|------|
| `mpl` CLI | Command-line tool for proxy, schemas, validation | — |
| Proxy endpoint | Intercepts and validates MCP/A2A traffic | 9443 |
| Metrics endpoint | Prometheus metrics | 9100 |
| Dashboard | Web UI for monitoring | 9080 |

---

## Next Steps

- [Quick Start](quick-start.md) — Get running in 5 minutes
- [Docker Compose](docker-compose.md) — Full stack with monitoring
- [First Validation](first-validation.md) — Validate your first payload
