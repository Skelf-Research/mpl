---
title: SDK Reference
---

# SDK Reference

Complete reference documentation for the Model Context Protocol Language (MPL) ecosystem. This section covers the SDK libraries, CLI tooling, configuration options, and API endpoints.

---

## Available References

### :material-language-python: Python SDK

Full reference for the MPL Python SDK, covering SType definitions, schema validation, QoM profiling, proxy integration, and more.

- **Pages:** 8
- **Install:** `pip install mpl-sdk`
- **Async model:** asyncio / async-await
- **Schema engine:** jsonschema

[:octicons-arrow-right-24: Python SDK Reference](python/index.md)

---

### :material-language-typescript: TypeScript SDK

Full reference for the MPL TypeScript SDK, covering SType definitions, schema validation, QoM profiling, proxy integration, and more.

- **Pages:** 8
- **Install:** `npm install @mpl/sdk`
- **Async model:** Promise / async-await
- **Schema engine:** AJV

[:octicons-arrow-right-24: TypeScript SDK Reference](typescript/index.md)

---

### :material-console: CLI Reference

Complete command reference for the `mpl` binary. Covers proxy management, schema operations, validation, QoM evaluation, and conformance testing.

[:octicons-arrow-right-24: CLI Reference](cli.md)

---

### :material-cog: Configuration Reference

Full reference for `mpl-config.yaml` including transport settings, MPL enforcement options, observability configuration, and environment variable overrides.

[:octicons-arrow-right-24: Configuration Reference](configuration.md)

---

### :material-api: REST API Reference

Documentation for all HTTP endpoints exposed by the MPL proxy (port 9443) and the optional registry API service.

[:octicons-arrow-right-24: REST API Reference](rest-api.md)

---

## Quick Comparison

| Feature | Python SDK | TypeScript SDK |
|---|---|---|
| **Language** | Python 3.9+ | TypeScript 5.0+ / Node 18+ |
| **Install command** | `pip install mpl-sdk` | `npm install @mpl/sdk` |
| **Async model** | asyncio / async-await | Promise / async-await |
| **Schema validation engine** | jsonschema | AJV |
| **SType definition** | Dataclass / Pydantic | Interface / AJV schema |
| **QoM profiling** | Built-in | Built-in |
| **Proxy integration** | `MplProxyClient` | `MplProxyClient` |
| **Registry support** | Read / Write | Read / Write |
| **Streaming** | AsyncIterator | AsyncGenerator |
| **IC assertions** | Decorator-based | Middleware-based |

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────┐
│                  Your Application                    │
│         (Python SDK  or  TypeScript SDK)             │
└───────────────────────┬─────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────┐
│                MPL Proxy (mpl binary)                │
│  Port 9443 (proxy)  │  Port 9100 (metrics)          │
│  ─────────────────────────────────────────────────  │
│  • Schema validation   • QoM enforcement            │
│  • IC assertions       • Policy engine              │
│  • Traffic learning    • Observability              │
└───────────────────────┬─────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────┐
│            MCP / A2A Server (upstream)               │
└─────────────────────────────────────────────────────┘
```

---

## Getting Started

If you are new to MPL, start with:

1. **[Getting Started Guide](../getting-started/index.md)** - Installation and first steps
2. **[Concepts](../concepts/index.md)** - Core concepts (STypes, QoM, IC)
3. **This Reference** - Detailed API and configuration documentation
