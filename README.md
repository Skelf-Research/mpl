# MPL: Meaning Protocol Layer

MPL is a semantic overlay that runs alongside MCP and A2A sessions to make the *meaning* of every exchange explicit, verifiable, and portable. It gives agents and orchestrators a shared contract for what a message represents, how capabilities are negotiated, and how semantic quality is enforced—without changing the underlying transport.

MPL adds:
- **Semantic Types (STypes):** globally versioned identifiers and schemas that declare the intent of each payload.
- **AI-ALPN Handshake:** a negotiation layer for models, tools, vocabularies, and policy/QoM profiles before work begins.
- **Quality of Meaning (QoM):** measurable SLOs such as schema fidelity, instruction compliance, groundedness, and determinism.
- **Semantic Integrity:** canonical hashes, provenance, and typed errors that keep meaning aligned across retries and hops.

The core technical specification is distributed across `docs/protocol-architecture.md` (architecture), `docs/implementation-guide.md` (implementation), and `docs/qom-evaluation-engine.md` (quality enforcement). The documents below clarify motivation, positioning, protocol details, and the staged rollout from internal incubation to industry publication.

## Documentation Map

### Core Concepts
- `docs/challenges.md` — why MCP and A2A alone are not enough, and the cross-industry problems MPL solves.
- `docs/conceptual-model.md` — mental models, overlays, and message flows that situate MPL alongside MCP/A2A.
- `docs/mpl-vs-schema-registries.md` — **how MPL differs from schema registries** (Confluent, AWS Glue, Buf) and when to use each.
- `GLOSSARY.md` — comprehensive definitions of all technical terms, abbreviations, and concepts.

### Technical Specification
- `docs/protocol-architecture.md` — the functional pillars of MPL (SType registry, AI-ALPN, QoM, envelopes, errors).
- `docs/implementation-guide.md` — practical adoption steps, handshake details, envelopes, and operational hooks.
- `docs/integration-modes.md` — **decision guide for integration paths** (sidecar proxy, SDK, native integration, migration strategies).
- `docs/qom-evaluation-engine.md` — architecture, workflow, and APIs for enforcing QoM metrics.
- `docs/policy-engine.md` — policy enforcement, consent management, and data governance.
- `docs/registry-architecture.md` — registry design, APIs, caching, and governance.
- `docs/security.md` — threat model, security controls, and compliance alignment.
- `docs/adversarial-robustness.md` — **Defense against adversarial attacks** (prompt injection, jailbreaking, data exfiltration)

### Integration Guides
- `docs/mpl-with-mcp.md` — detailed integration guide for layering MPL on MCP (handshake, envelopes, tooling, operations).
- `docs/mpl-with-a2a.md` — detailed integration guide for layering MPL on A2A (peer negotiation, typed messaging, governance).
- `docs/mpl-vs-evolving-protocols.md` — why MPL remains a distinct semantic layer even as MCP/A2A evolve, with autonomy and regulatory focus.

### Examples
- `docs/examples/calendar-workflow/` — complete reference implementation with STypes, tools, profiles, and requests/responses.

### Stakeholder Value Propositions
- `docs/regulated-enterprise-value.md` — **How MVP delivers compliance value** (SOX, GDPR, HIPAA, EU AI Act, UK FCA/PRA mappings, adversarial defenses)
- `docs/ai-safety-risk-alignment.md` — **NEW: AI Safety & Risk team alignment** (constraint enforcement, risk quantification, incident management)

### Strategy & Planning
- `docs/market-assessment.md` — **STRATEGIC: Honest market viability assessment** (7/10 value, 12-18 months early, execution dependencies)
- `docs/industry-paper-outline.md` — narrative arc, evidence, and solution framing for the forthcoming industry paper.
- `docs/gtm.md` — market landscape, value prop, ICPs, and go-to-market experiments.
- `docs/roadmap.md` — phased delivery plan from prototype to open specification and ecosystem adoption.

## Implementation Status

**⚠️ MPL is in early specification phase.** The protocol is architecturally sound but requires scoping and refinement for production use.

**Key documents for implementers:**
- `docs/implementation-feasibility-analysis.md` — comprehensive feasibility review identifying risks, gaps, and realistic timelines
- `docs/mvp-scope.md` — pragmatic MVP scope with clear include/exclude decisions and 3-6 month roadmap

**Summary:** Full MPL spec is ambitious (12-24 months to production-ready). Recommended approach is phased rollout starting with Schema Fidelity + Instruction Compliance, deferring advanced QoM metrics (Groundedness, Determinism) and full policy engine to later phases.

## Getting Involved

### For Implementers
1. **Start here:** `docs/integration-modes.md` — **MPL offers 3 integration modes** (priority order):
   - **Sidecar Proxy** (#1) — Zero-code, <30 min, works everywhere (recommended for 90% of adopters)
   - **Native Integration** (#2) — For MCP/A2A vendors (ecosystem play, differentiation)
   - **SDK** (#3) — Power users only (stateful assertions, custom telemetry)
2. Review `docs/implementation-guide.md` — technical implementation details
3. Walk through `docs/protocol-architecture.md` — understand the core protocol
4. Check `docs/mvp-scope.md` — recommended MVP feature set and 3-6 month timeline

### For Decision Makers
1. **START HERE:** `docs/market-assessment.md` — **Honest market viability assessment** (Is this valuable given current market conditions?)
2. Review `docs/challenges.md` — understand the problem MPL solves
3. Read stakeholder-specific value:
   - `docs/regulated-enterprise-value.md` — Compliance teams (SOX, GDPR, HIPAA, EU AI Act, UK FCA/PRA)
   - `docs/ai-safety-risk-alignment.md` — AI Safety and Risk teams
4. Review `docs/implementation-feasibility-analysis.md` — realistic technical scope and risks
5. Use `docs/roadmap.md` and `docs/gtm.md` — for strategic planning and go-to-market

Open questions, comments, or contributions? File issues or reach out to the MPL working group.
