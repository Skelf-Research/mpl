# MPL: Meaning Protocol Layer

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Docs](https://img.shields.io/badge/docs-mkdocs-blue.svg)](https://skelf-research.github.io/mpl)
[![Tests](https://img.shields.io/badge/tests-144%20passing-brightgreen.svg)](#status)

**Your AI agents are stuck in compliance purgatory.** MCP and A2A give them transport—but compliance teams are asking: *"Can you prove the agent did what you said?"*

MPL is the answer. A lightweight overlay that adds **typed contracts**, **quality SLOs**, and **tamper-evident audit trails** to your existing agent infrastructure. No rewrites. No new transport. Just the semantic governance layer that gets your agents to production.

```
┌─────────────────────────────────────────────────────────┐
│                    Your Agent Logic                      │
├─────────────────────────────────────────────────────────┤
│                   MPL (this layer)                       │
│   Typed Schemas  ·  Quality Metrics  ·  Policy Engine   │
│   Semantic Hashing  ·  Provenance  ·  Audit Trails      │
├─────────────────────────────────────────────────────────┤
│            MCP (client-server)  |  A2A (peer-to-peer)   │
└─────────────────────────────────────────────────────────┘
```

---

## Get Running in 2 Minutes

```bash
cargo install mpl-cli
mpl proxy http://your-mcp-server:8080
```

That's it. Your agents now talk through MPL. Open http://localhost:9080 for the dashboard.

**What you immediately get:**
- Every request validated against typed schemas
- Quality metrics computed per interaction
- Tamper-evident hashes on every payload
- Full provenance trail (who did what, with what intent)

**When you're ready to enforce:**

```bash
mpl schemas generate        # Learn schemas from live traffic
mpl schemas approve --all   # Lock them in
mpl proxy http://your-mcp-server:8080 --mode production  # Now invalid requests are blocked
```

---

## Why Teams Choose MPL

| Without MPL | With MPL |
|------------|----------|
| Agents send untyped JSON—nobody knows if it's correct | Every message has a versioned schema contract (`org.calendar.Event.v1`) |
| Quality is "works on my prompt" | Six measurable metrics with enforceable thresholds |
| Compliance review takes 12-18 weeks | Tamper-evident audit trails satisfy SOX/GDPR/HIPAA/EU AI Act |
| Breaking changes discovered in production | Schema validation catches errors before they reach servers |
| No way to prove what the agent did | Provenance + semantic hashes = verifiable execution records |

---

## SDK Examples

### Python

```bash
pip install mpl-sdk
```

```python
from mpl_sdk import Client, Mode

async with Client("http://localhost:9443", mode=Mode.PRODUCTION) as client:
    result = await client.call("calendar.create", {
        "title": "Quarterly Review",
        "start": "2025-03-15T14:00:00Z",
        "end": "2025-03-15T15:00:00Z"
    })

    print(result.valid)       # True  — schema contract met
    print(result.qom_passed)  # True  — quality thresholds met
    print(result.data)        # The response payload
```

### TypeScript

```bash
npm install @mpl/sdk
```

```typescript
import { MplClient, Mode } from '@mpl/sdk';

const client = new MplClient('http://localhost:9443', { mode: Mode.Production });

const result = await client.call('calendar.create', {
  title: 'Quarterly Review',
  start: '2025-03-15T14:00:00Z',
  end: '2025-03-15T15:00:00Z',
});

console.log(result.valid);     // true  — schema contract met
console.log(result.qomPassed); // true  — quality thresholds met
```

### Docker

```bash
git clone https://github.com/Skelf-Research/mpl.git && cd mpl
docker compose up -d
curl http://localhost:9443/health  # {"status": "healthy"}
```

---

## How It Works

**1. Typed Contracts (STypes)**

Every message declares its semantic type—a versioned, schema-backed identifier:

```json
{
  "stype": "org.calendar.Event.v1",
  "payload": { "title": "Meeting", "start": "2025-01-15T10:00:00Z", "end": "..." },
  "sem_hash": "blake3:7f2a...",
  "provenance": { "agent_id": "scheduler", "intent": "create-event" }
}
```

25+ STypes ship pre-seeded (`org.*`, `data.*`, `eval.*`, `ai.*`). Create your own in minutes.

**2. Quality of Meaning (QoM)**

Six metrics that measure whether your agent is doing its job:

| Metric | What It Measures | Example Threshold |
|--------|-----------------|-------------------|
| Schema Fidelity | Payload matches the contract | 1.0 (mandatory) |
| Instruction Compliance | Assertions and constraints met | >= 0.97 |
| Groundedness | Claims supported by sources | >= 0.90 |
| Determinism | Output stable under perturbation | >= 0.85 |
| Ontology Adherence | Domain rules followed | >= 0.95 |
| Tool Outcome | Side effects match expectations | >= 0.90 |

Combine into profiles: `qom-basic` for dev, `qom-strict-argcheck` for production, or define your own.

**3. Zero-Code Integration**

MPL deploys as a sidecar proxy. No code changes to your agents or servers:

```
Agent  ──▶  MPL Proxy  ──▶  MCP/A2A Server
              │
              ├── Validates schemas
              ├── Computes QoM metrics
              ├── Enforces policies
              └── Records audit trail
```

**4. Compliance Built In**

| Regulation | MPL Control |
|------------|-------------|
| **SOX** | Semantic hashes + provenance = tamper-evident audit trails |
| **GDPR** | Consent refs in envelopes; policy engine for data handling |
| **HIPAA** | SType patterns restrict PHI; QoM enforces accuracy |
| **EU AI Act** | QoM for transparency; provenance for explainability |

---

## Documentation

Full docs at **[skelf-research.github.io/mpl](https://skelf-research.github.io/mpl)**

| I want to... | Go here |
|--------------|---------|
| Understand the value proposition | [Why MPL](https://skelf-research.github.io/mpl/overview/why-mpl/) |
| Get running quickly | [Quick Start](https://skelf-research.github.io/mpl/getting-started/quick-start/) |
| Understand the architecture | [Concepts](https://skelf-research.github.io/mpl/concepts/architecture/) |
| Follow a hands-on tutorial | [Guides](https://skelf-research.github.io/mpl/guides/) |
| Look up SDK methods | [Python SDK](https://skelf-research.github.io/mpl/reference/python/) / [TypeScript SDK](https://skelf-research.github.io/mpl/reference/typescript/) |
| Deploy to production | [Deployment](https://skelf-research.github.io/mpl/deployment/) |
| Show my CISO | [Security & Compliance](https://skelf-research.github.io/mpl/security/) |

---

## Status

| Phase | Status | What Shipped |
|-------|--------|-------------|
| **Phase 1** | Complete | Core protocol, Python SDK, Sidecar Proxy, Schema Registry |
| **Phase 2** | Complete | TypeScript SDK, Registry API, Helm Chart, Policy Engine |
| **Phase 3** | In Progress | Conformance suite, A2A hardening, Production readiness |

**144 tests** across the workspace. **25+ pre-seeded STypes.** SDKs for Python and TypeScript.

---

## Repository

```
crates/mpl-core/        Core protocol implementation (Rust)
crates/mpl-proxy/       Sidecar proxy
crates/mpl-cli/         CLI tooling
python/                 Python SDK
typescript/             TypeScript SDK
registry/stypes/        Pre-seeded SType definitions
helm/mpl-proxy/         Kubernetes Helm chart
examples/               Tutorials and demos
documentation/          MkDocs documentation site
```

## Contributing

We welcome contributions. See the [Contributing Guide](https://skelf-research.github.io/mpl/community/contributing/) for setup instructions.

## License

Apache 2.0 — [Open an issue](https://github.com/Skelf-Research/mpl/issues) if you have questions.
