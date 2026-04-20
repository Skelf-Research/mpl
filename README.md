# MPL: Meaning Protocol Layer

**Semantic governance for AI agents in regulated environments.**

MPL is a lightweight protocol overlay that brings typed contracts, quality SLOs, and audit trails to AI agent communications—without replacing MCP or A2A.

## Quick Start (5 minutes)

```bash
# Install
cargo install mpl-cli

# Start proxy pointing to your MCP server - that's it!
mpl proxy http://your-mcp-server:8080

# Dashboard: http://localhost:9080
# Metrics:   http://localhost:9100/metrics
```

**What you get immediately:**
- Traffic visibility for all MCP/A2A requests
- Real-time metrics and dashboard
- Schema learning from traffic (use `mpl schemas generate` after observing traffic)

**Next steps:**
```bash
mpl schemas generate        # Generate schemas from recorded traffic
mpl schemas approve --all   # Approve inferred schemas
mpl proxy http://server:8080 --mode production  # Enforce validation
```

See the [Quick Start Guide](docs/quick-start.md) for the full walkthrough.

---

## Executive Summary

| Stakeholder | Key Benefit |
|-------------|-------------|
| **CTO** | Unblock AI agent deployment by providing compliance teams the semantic guarantees they need |
| **CISO** | Audit trails, schema enforcement, and policy controls that map to SOX/GDPR/HIPAA/EU AI Act |
| **Architect** | Drop-in sidecar proxy requiring zero code changes; works with existing MCP/A2A infrastructure |
| **Solution Engineer** | Docker Compose deployment in <10 minutes; SDKs for Python and TypeScript |

### The Problem MPL Solves

Regulated enterprises are blocking AI agent deployments because they cannot answer: *"Can we prove the agent did what we said?"*

Current agent protocols (MCP, A2A) provide transport—not semantics. Teams lack:
- **Schema enforcement**: No contract for what messages mean
- **Quality guarantees**: No SLOs for agent behavior
- **Audit trails**: No provenance or tamper detection
- **Policy controls**: No enforcement of organizational rules

MPL fills this gap as a semantic overlay that runs alongside existing protocols.

---

## For CTOs: Strategic Value

### Unblock AI Deployment

AI agent pilots are stuck in 12-18 week compliance approval cycles. MPL provides the semantic contracts that compliance teams need to approve production deployment.

### Overlay, Not Replacement

MPL is not a new protocol—it's an overlay that augments MCP and A2A with typed semantics. Your existing investments in agent infrastructure remain intact.

### Market Timing

MPL addresses a 2026-2027 problem today. As enterprises move from supervised copilots to autonomous agents, semantic governance becomes critical. Early adoption positions your organization ahead of regulatory requirements.

**Key Documents:**
- [`docs/market-assessment.md`](docs/market-assessment.md) — Honest market viability analysis
- [`docs/challenges.md`](docs/challenges.md) — Industry problems MPL solves
- [`docs/gtm.md`](docs/gtm.md) — Go-to-market strategy

---

## For CISOs: Security & Compliance

### Compliance Mapping

| Regulation | MPL Control |
|------------|-------------|
| **SOX** | Semantic hashes + provenance provide tamper-evident audit trails |
| **GDPR** | Consent references in envelopes; policy engine for data handling rules |
| **HIPAA** | SType patterns restrict PHI access; QoM thresholds enforce accuracy |
| **EU AI Act** | QoM metrics for transparency; provenance for explainability |
| **UK FCA/PRA** | Policy engine for fiduciary duty; instruction compliance checks |

### Adversarial Defenses

MPL provides defense-in-depth against AI-specific attacks:

| Threat | Defense |
|--------|---------|
| **Prompt Injection** | Schema validation rejects unexpected fields |
| **Data Exfiltration** | Policy engine enforces data handling rules |
| **Output Manipulation** | Semantic hashes detect payload tampering |
| **Jailbreaking** | QoM thresholds flag anomalous behavior |

### Audit Capabilities

Every MPL message includes:
- **Semantic hash** (BLAKE3): Tamper-evident content fingerprint
- **Provenance**: Agent ID, intent, inputs, policy references
- **QoM report**: Quality metrics with pass/fail status
- **Typed errors**: Structured failure information for incident response

**Key Documents:**
- [`docs/regulated-enterprise-value.md`](docs/regulated-enterprise-value.md) — Detailed compliance mapping
- [`docs/adversarial-robustness.md`](docs/adversarial-robustness.md) — Security controls
- [`docs/ai-safety-risk-alignment.md`](docs/ai-safety-risk-alignment.md) — Risk team integration

---

## For Architects: Technical Architecture

### Protocol Stack

```
┌─────────────────────────────────────────────────────────┐
│                    Application Layer                     │
│              (Agent Logic, Business Rules)               │
├─────────────────────────────────────────────────────────┤
│                  MPL Semantic Layer                      │
│   ┌─────────────┐  ┌──────────┐  ┌────────────────┐     │
│   │   STypes    │  │   QoM    │  │  Policy Engine │     │
│   │  (schemas)  │  │ (metrics)│  │   (rules)      │     │
│   └─────────────┘  └──────────┘  └────────────────┘     │
│   ┌─────────────────────────────────────────────────┐   │
│   │         AI-ALPN Handshake (negotiation)         │   │
│   └─────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────┤
│                  Transport Layer                         │
│              MCP (client-server)  |  A2A (peer-to-peer) │
└─────────────────────────────────────────────────────────┘
```

### Integration Modes

| Mode | Effort | Use Case |
|------|--------|----------|
| **Sidecar Proxy** | Zero code | Recommended for 90% of deployments |
| **SDK Integration** | Low code | Custom telemetry, advanced assertions |
| **Native Integration** | Medium | MCP/A2A vendors adding MPL support |

### Core Components

| Component | Purpose | Implementation |
|-----------|---------|----------------|
| **STypes** | Versioned semantic types with JSON Schema | `crates/mpl-core/src/stype.rs` |
| **Envelope** | Message wrapper with provenance, hash, QoM | `crates/mpl-core/src/envelope.rs` |
| **AI-ALPN** | Capability negotiation before work begins | `crates/mpl-core/src/handshake.rs` |
| **QoM Engine** | Quality metrics evaluation | `crates/mpl-core/src/qom.rs` |
| **Policy Engine** | Rule-based enforcement | `crates/mpl-core/src/policy.rs` |
| **Validation** | JSON Schema validation with caching | `crates/mpl-core/src/validation.rs` |

### QoM Profiles

| Profile | Metrics | Thresholds | Use Case |
|---------|---------|------------|----------|
| `qom-basic` | Schema Fidelity | SF = 1.0 | Development, low-risk |
| `qom-strict-argcheck` | SF + Instruction Compliance | SF = 1.0, IC >= 0.97 | Production, regulated |
| `qom-outcome` | SF + IC + Tool Outcome | All >= 0.95 | High-stakes workflows |
| `qom-comprehensive` | All 6 metrics | Configurable | Mission-critical |

**Key Documents:**
- [`docs/protocol-architecture.md`](docs/protocol-architecture.md) — Full protocol specification
- [`docs/integration-modes.md`](docs/integration-modes.md) — Integration decision guide
- [`docs/qom-evaluation-engine.md`](docs/qom-evaluation-engine.md) — QoM implementation details

---

## For Solution Engineers: Deployment

### Quick Start (10 minutes)

```bash
# Clone and deploy
git clone https://github.com/anthropics/mpl.git
cd mpl
docker compose up -d

# Verify deployment
curl http://localhost:9443/health
curl http://localhost:9443/capabilities

# Test validation
curl -X POST http://localhost:9443/validate \
  -H "Content-Type: application/json" \
  -d '{"stype": "org.calendar.Event.v1", "payload": {"title": "Meeting"}}'
```

### Kubernetes Deployment

```bash
helm repo add mpl https://mpl-charts.example.com
helm install mpl-proxy mpl/mpl-proxy \
  --set registry.endpoint=http://registry:8080 \
  --set qom.defaultProfile=qom-strict-argcheck
```

### SDK Integration

**Python:**
```python
from mpl import MplClient, SType, QomProfile

client = MplClient("http://localhost:9443")
client.negotiate(stypes=["org.calendar.Event.v1"], profile=QomProfile.STRICT)

result = client.validate(
    stype="org.calendar.Event.v1",
    payload={"title": "Meeting", "start": "2025-01-15T10:00:00Z"}
)
assert result.qom_report.meets_profile
```

**TypeScript:**
```typescript
import { MplClient, QomProfile } from '@mpl/sdk';

const client = new MplClient('http://localhost:9443');
await client.negotiate({
  stypes: ['org.calendar.Event.v1'],
  profile: QomProfile.Strict
});

const result = await client.validate({
  stype: 'org.calendar.Event.v1',
  payload: { title: 'Meeting', start: '2025-01-15T10:00:00Z' }
});
```

### Registry Seeding

25+ STypes pre-seeded across namespaces:

| Namespace | STypes | Examples |
|-----------|--------|----------|
| `eval.*` | 5+ | RAGQuery, SearchResult, Feedback |
| `data.*` | 6+ | Table, Record, Query, FileMetadata |
| `org.*` | 8+ | Step, Pipeline, Profile, Alert, Rating |
| `ai.*` | 6+ | Template, Response, Reasoning |

**Key Documents:**
- [`docs/getting-started.md`](docs/getting-started.md) — Full deployment guide
- [`docs/implementation-guide.md`](docs/implementation-guide.md) — SDK integration details
- [`helm/mpl-proxy/`](helm/mpl-proxy/) — Kubernetes Helm chart

---

## Implementation Status

| Phase | Status | Deliverables |
|-------|--------|--------------|
| **Phase 1 (MVP)** | ✅ Complete | Core protocol, Python SDK, Sidecar Proxy, Registry |
| **Phase 2** | ✅ Complete | TypeScript SDK, Registry API, Helm Chart, Policy Engine |
| **Phase 3** | 🚧 In Progress | Conformance suite (100+), A2A integration, Production hardening |

### Test Coverage

- **144 tests** across workspace
- **20 A2A integration tests** (client-server + MPL envelope)
- **107 core protocol tests** (conformance suite)

### Repository Structure

```
mpl/
├── crates/
│   ├── mpl-core/       # Core protocol (Rust)
│   ├── mpl-proxy/      # Sidecar proxy
│   ├── mpl-cli/        # CLI tooling
│   ├── mpl-python/     # Python SDK (PyO3)
│   └── mpl-registry-api/  # Registry REST API
├── typescript/         # TypeScript SDK
├── registry/stypes/    # SType definitions
├── helm/mpl-proxy/     # Kubernetes chart
├── docs/               # Documentation
└── examples/           # Tutorials and samples
```

---

## Documentation Index

### By Role

| Role | Start Here |
|------|------------|
| **CTO/Executive** | [`docs/market-assessment.md`](docs/market-assessment.md) |
| **CISO/Compliance** | [`docs/regulated-enterprise-value.md`](docs/regulated-enterprise-value.md) |
| **Architect** | [`docs/protocol-architecture.md`](docs/protocol-architecture.md) |
| **Solution Engineer** | [`docs/getting-started.md`](docs/getting-started.md) |
| **Developer** | [`docs/implementation-guide.md`](docs/implementation-guide.md) |

### Full Index

**Core Concepts:**
- [`docs/challenges.md`](docs/challenges.md) — Problems MPL solves
- [`docs/conceptual-model.md`](docs/conceptual-model.md) — Mental models
- [`GLOSSARY.md`](GLOSSARY.md) — Term definitions

**Technical Specification:**
- [`docs/protocol-architecture.md`](docs/protocol-architecture.md) — Protocol pillars
- [`docs/qom-evaluation-engine.md`](docs/qom-evaluation-engine.md) — Quality metrics
- [`docs/policy-engine.md`](docs/policy-engine.md) — Policy enforcement
- [`docs/registry-architecture.md`](docs/registry-architecture.md) — Registry design

**Integration:**
- [`docs/mpl-with-mcp.md`](docs/mpl-with-mcp.md) — MCP integration
- [`docs/mpl-with-a2a.md`](docs/mpl-with-a2a.md) — A2A integration
- [`docs/integration-modes.md`](docs/integration-modes.md) — Deployment options

**Security & Compliance:**
- [`docs/security.md`](docs/security.md) — Threat model
- [`docs/adversarial-robustness.md`](docs/adversarial-robustness.md) — Attack defenses
- [`docs/regulated-enterprise-value.md`](docs/regulated-enterprise-value.md) — Compliance mapping

**Strategy:**
- [`docs/market-assessment.md`](docs/market-assessment.md) — Market analysis
- [`docs/roadmap.md`](docs/roadmap.md) — Development plan
- [`docs/mvp-scope.md`](docs/mvp-scope.md) — MVP specification

---

## License

Apache 2.0

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

Questions? Open an issue or contact the MPL working group.
