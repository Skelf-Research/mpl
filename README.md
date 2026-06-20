# MPL: Meaning Protocol Layer

[![CI](https://github.com/Skelf-Research/mpl/actions/workflows/ci.yaml/badge.svg)](https://github.com/Skelf-Research/mpl/actions/workflows/ci.yaml)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Docs](https://img.shields.io/badge/docs-mkdocs-blue.svg)](https://skelf-research.github.io/mpl)
[![Security audit](https://img.shields.io/badge/security-cargo--audit-blue.svg)](.cargo/audit.toml)

### Contracts, quality measurement, and audit trails for AI agent communication.

MPL sits between your agents and their tools. It makes every interaction **typed**, **measurable**, and **provable** — so you can move from prototype to production without rewriting your stack.

```
┌─────────────────────────────────────────────────────────┐
│                    Your Agent Logic                      │
├─────────────────────────────────────────────────────────┤
│                    MPL (this layer)                      │
│   Contracts  ·  Quality Metrics  ·  Policies  ·  Proofs │
├─────────────────────────────────────────────────────────┤
│            MCP (client-server)  |  A2A (peer-to-peer)   │
└─────────────────────────────────────────────────────────┘
```

---

## Get Running in 2 Minutes

```bash
cargo install mplx
mpl proxy http://your-mcp-server:8080
```

Your agents keep working exactly as before — MPL observes, validates, and records everything passing through. Open http://localhost:9080 for the dashboard.

When you're ready to enforce:

```bash
mpl schemas generate        # Learn contracts from live traffic
mpl schemas approve --all   # Lock them in
mpl proxy http://your-mcp-server:8080 --mode production
```

Now malformed requests get blocked before they reach your server.

---

## What This Does For You

| Your problem | How MPL helps |
|-------------|---------------|
| Agent outputs are unpredictable | Define contracts for every message type — invalid payloads are caught immediately |
| "It worked in testing" doesn't satisfy compliance | Every interaction gets a tamper-proof hash and quality score you can point to |
| Breaking changes surface in production | Schema validation catches structural errors at the protocol layer |
| No visibility into what agents actually did | Full provenance chain — which agent, what intent, which inputs, what quality score |
| Compliance review blocks your deployment | Audit trails map directly to SOX, GDPR, HIPAA, and EU AI Act requirements |

---

## This Is Not a Guardrail

Guardrails are reactive safety nets — they block bad outputs after generation. MPL is different:

| | Guardrails | MPL |
|-|-----------|-----|
| **When** | After the agent responds | Before, during, and after |
| **What** | Filters harmful content | Defines what correct looks like, measures quality, records proof |
| **Scope** | Single agent output | Entire communication chain across agents |
| **Output** | Pass/block decision | Typed contracts, quality scores, provenance records, audit trails |
| **Adapts to** | Safety policies | Your domain schemas, your quality thresholds, your compliance requirements |

MPL doesn't ask "is this safe?" — it asks **"did this meet the contract, and can you prove it?"**

---

## Adapt It To Your Domain

### Define Your Own Contracts

Create a schema for any message type your agents handle:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "Support Ticket",
  "type": "object",
  "required": ["title", "priority", "description"],
  "properties": {
    "title": { "type": "string", "minLength": 1 },
    "priority": { "enum": ["low", "medium", "high", "critical"] },
    "description": { "type": "string", "minLength": 10 },
    "assignee": { "type": "string" }
  }
}
```

Save it as `registry/stypes/org/support/Ticket/v1/schema.json` — done. Your agents now validate against it.

### Set Your Own Quality Thresholds

Pick which metrics matter for your use case:

```yaml
# qom-my-profile.json
name: "my-production-profile"
metrics:
  schema_fidelity: { min: 1.0 }          # Must match contract perfectly
  instruction_compliance: { min: 0.95 }   # Must follow constraints
  groundedness: { min: 0.90 }             # Claims must cite sources
```

### Write Your Own Policies

Enforce organizational rules at the protocol layer:

```yaml
policies:
  - name: "require-provenance"
    match: { stypes: ["org.finance.*"] }
    rules:
      - deny_if_missing: ["provenance.agent_id"]
      - require_profile: "my-production-profile"
```

### 25+ Contracts Ship Out of the Box

Get started without defining anything — use the pre-built types and extend from there:

| Namespace | Examples |
|-----------|----------|
| `org.*` | Calendar events, task plans, tool calls, workflows |
| `data.*` | Tables, records, queries, file metadata |
| `eval.*` | RAG queries, search results, feedback |
| `ai.*` | Prompt templates, completions, reasoning traces |

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

    result.valid       # True  — matched the contract
    result.qom_passed  # True  — met quality thresholds
    result.data        # The response payload
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

result.valid;     // true  — matched the contract
result.qomPassed; // true  — met quality thresholds
```

### Docker

```bash
git clone https://github.com/Skelf-Research/mpl.git && cd mpl
docker compose up -d
curl http://localhost:9443/health  # {"status": "healthy"}
```

---

## How It Works

**1. Contracts**

Every message declares what it is — a versioned identifier backed by a JSON Schema:

```json
{
  "stype": "org.calendar.Event.v1",
  "payload": { "title": "Meeting", "start": "2025-01-15T10:00:00Z", "end": "..." },
  "sem_hash": "blake3:7f2a...",
  "provenance": { "agent_id": "scheduler", "intent": "create-event" }
}
```

If the payload doesn't match the contract, it's rejected before reaching the server.

**2. Quality Measurement**

Six metrics you can mix and match into profiles:

| Metric | What it answers |
|--------|----------------|
| Schema Fidelity | Does the payload match the contract? |
| Instruction Compliance | Were the constraints followed? |
| Groundedness | Are claims backed by sources? |
| Determinism | Is the output stable across runs? |
| Ontology Adherence | Are domain rules respected? |
| Tool Outcome | Did the side effects match expectations? |

Use `qom-basic` while developing. Switch to `qom-strict-argcheck` for production. Or define your own.

**3. No Code Changes Required**

MPL runs as a proxy alongside your existing setup:

```
Agent  ──▶  MPL Proxy  ──▶  MCP/A2A Server
              │
              ├── Validates contracts
              ├── Measures quality
              ├── Applies policies
              └── Records everything
```

Start in transparent mode (observe only), graduate to strict (enforce) when you're ready.

**4. Compliance You Can Point To**

Every message through MPL carries a BLAKE3 hash (tamper detection), provenance metadata (who did what), and a quality report (did it meet thresholds). These map to:

| Requirement | What MPL provides |
|-------------|-------------------|
| Tamper-evident records (SOX) | Cryptographic hash on every payload |
| Data handling proof (GDPR) | Consent references + policy enforcement |
| Accuracy controls (HIPAA) | Quality thresholds on clinical outputs |
| Transparency (EU AI Act) | Quality scores + full provenance chain |

---

## Documentation

Full docs at **[skelf-research.github.io/mpl](https://skelf-research.github.io/mpl)**

| I want to... | Start here |
|--------------|-----------|
| Get running quickly | [Quick Start](https://skelf-research.github.io/mpl/getting-started/quick-start/) |
| Understand the architecture | [How It Works](https://skelf-research.github.io/mpl/overview/how-it-works/) |
| Follow a tutorial end-to-end | [Guides](https://skelf-research.github.io/mpl/guides/) |
| Create my own contracts | [Custom SType Tutorial](https://skelf-research.github.io/mpl/guides/tutorials/custom-stype/) |
| Look up SDK methods | [Python](https://skelf-research.github.io/mpl/reference/python/) / [TypeScript](https://skelf-research.github.io/mpl/reference/typescript/) |
| Deploy to Kubernetes | [Deployment](https://skelf-research.github.io/mpl/deployment/) |
| Understand the compliance story | [Security & Compliance](https://skelf-research.github.io/mpl/security/) |

---

## Status

| Phase | What's in it |
|-------|-------------|
| **Phase 1** — Complete | Core protocol, Python SDK, Sidecar Proxy, Schema Registry |
| **Phase 2** — Complete | TypeScript SDK, Registry API, Helm Chart, Policy Engine |
| **Phase 3** — In Progress | Conformance suite, A2A hardening, Production readiness |

---

## Repository

```
crates/mpl-protocol/    Core protocol (Rust)
crates/mpl-proxy/       Sidecar proxy
crates/mplx/            CLI tooling
python/                 Python SDK
typescript/             TypeScript SDK
registry/stypes/        Pre-built contract definitions
helm/mpl-proxy/         Kubernetes Helm chart
examples/               Tutorials and demos
documentation/          Documentation site (MkDocs)
```

## Contributing

See the [Contributing Guide](https://skelf-research.github.io/mpl/community/contributing/) for development setup and workflow.

## License

MIT — [Open an issue](https://github.com/Skelf-Research/mpl/issues) if you have questions.
