# MPL Go-To-Market Playbook

This playbook provides an actionable plan for positioning MPL alongside MCP and A2A. See `docs/protocol-architecture.md` for the technical specification.

**⚠️ CRITICAL: Read `docs/market-assessment.md` first.** The market assessment shows MPL is 12-18 months early to market maturity. This GTM plan must be executed with that timing reality in mind (design partner validation first, scale in 2026-2027).

## 1. Market Landscape

- **Model Context Protocol (MCP):** client–server topology focused on LLMs mounting tool hosts. Strengths include mature transports and vendor momentum; gaps include implicit semantics, limited provenance, and no quality contracts.
- **Agent-to-Agent (A2A):** peer-to-peer coordination for autonomous agents. Provides a discovery fabric yet leaves schema/version governance and QoM guarantees unspecified.
- **Summary:** both protocols solve connectivity but leave meaning implicit. Enterprises layering critical workflows on MCP/A2A struggle with schema drift, unmeasured quality, and audit requirements.

## 2. MPL Differentiators

| Area | MPL advantage |
| ---- | ------------- |
| Semantic contracts | Registry-backed STypes, canonical schemas, semantic hashes. |
| Negotiation | AI-ALPN handshake selects protocols, models, tools, policies, and QoM profiles with downgrade telemetry. |
| Quality | QoM metrics (Schema Fidelity, Instruction Compliance, Groundedness, Determinism, Ontology Adherence, Tool Outcome) with enforceable thresholds. |
| Governance | Policy negotiation, consent receipts, redaction filters, provenance logs. |
| Overlay adoption | Runs on existing MCP/A2A transports; minimal friction through proxy/SDK rollout. |

## 3. Target Segments & Use Cases

- **Regulated enterprises (finance, healthcare, legal):** need auditable agent workflows, policy controls, and QoM evidence before releasing tooling beyond pilots.
- **Risk & compliance teams inside enterprises:** aim to shorten assurance cycles by requiring machine-verifiable QoM reports instead of bespoke manual reviews.
- **Platform and framework builders:** require reusable semantic contracts to support plug-and-play tools and cross-model compatibility.
- **Agent ops teams:** seek observability and downgrade detection for multi-tool pipelines where semantic mismatches cause incidents.
- **Data-intensive domains:** rely on grounded retrieval and typed references to avoid context rot and hallucination-induced failures.

Key use cases: calendar/task orchestration, document review with tool chains, compliance automation, knowledge-graph assisted agents, and multi-agent workflows with delegated tasks.

## 4. Value Proposition & Messaging

- **Tagline:** “The semantic contract layer for AI agents and tools.”
- **Core promise:** “Connect, coordinate, and govern meaning-centric AI workflows—not just messages.”
- **Proof points:** schema versioning, negotiated capabilities, QoM dashboards, signed provenance, accelerated risk sign-off (weeks → days).
- **Call to action:** “Wrap your MCP/A2A stack with MPL, validate meaning in 30 minutes, and unlock audit-ready agents.”

## 5. GTM Motions & Experiments

- **Overlay Fit (T1):** ship sidecar proxy around an existing MCP server; target time-to-first-typed-call <30 minutes and <5% breakage.
- **Developer Ergonomics (T2):** SDK alpha for Python/TS; 10 external developers complete a typed flow in ≤100 LOC with DX NPS ≥ +30.
- **Enterprise Pull (T3):** two vertical pilots (finance + healthcare); package QoM SLOs, provenance, and policy enforcement; close ≥1 paid PoC.
- **Registry Value (T4):** seed 12 STypes + 6 tools; earn ≥5 third-party PRs without heavy hand-holding.
- **QoM Demand (T5):** deploy QoM widget; aim for ≥3 teams incorporating reports into QA gates.
- **Compatibility (T6):** run MPL proxy against Claude MCP and a popular A2A framework; keep downgrade/compat errors <2%.

## 6. GTM Activities

- Launch developer pilot with proxy + SDK + sample registry (calendar workflow).
- Publish thought leadership highlighting the semantic gap in existing protocols.
- Run joint workshops with MCP/A2A maintainers to emphasise complementarity.
- Open the registry and conformance mini-suite to community contributions.
- Equip design partners with QoM dashboards and policy packs to demonstrate enterprise readiness.

## 7. Competitive Risks & Mitigations

- **Protocol convergence:** MCP/A2A might add semantics natively. *Mitigation:* stay ahead on QoM depth, registry tooling, and governance automation.
- **Perceived overhead:** developers may resist another layer. *Mitigation:* emphasise overlay approach, minimal handshake, and proxy rollout path.
- **Ecosystem inertia:** without contributions, registry stagnates. *Mitigation:* publish curated profiles, adapters, and migration guides to reduce toil.
- **"Why not use schema registries?"** Conflation with Confluent/Buf/AWS Glue. *Mitigation:* position MPL as **complementary** (see `docs/mpl-vs-schema-registries.md`). Schema registries = data contracts; MPL = AI semantic contracts + compliance. Use both.

## 8. Pricing & Packaging Tests

- **Seat-based:** $30–$50 per developer/month for QoM dashboards, governance tooling, and support.
- **Usage-based:** charge per 1,000 validated tool calls or per QoM evaluation.
- **Control-plane SaaS:** host registry explorer, conformance suite, policy packs; monetise via enterprise subscriptions.
- **MPL Lite SKU:** schema-only offering for frictionless trials with upsell to full QoM/policy features if enterprise sales cycles stall.

## 9. Momentum Metrics

- Time-to-first-typed-call (p50/p90).
- Schema fidelity rate (target ≥99.5%).
- QoM SLO pass rate per workflow.
- Handshake downgrade frequency (target <5%).
- Unknown SType rate (target <0.1%).
- Registry contribution velocity (PRs per month).
- Proxy adoption (# of services running MPL sidecar).
- Paid PoCs started and closed.

## 10. Field Enablement Checklist

1. Maintain an assumption board summarising overlay fit, DX acceptance, enterprise demand, registry traction, QoM visibility, and compatibility (see `docs/roadmap.md`).
2. Carry demo assets: MPL-wrapped MCP session, QoM failure/resolution walkthrough, policy enforcement example.
3. Prepare case studies showing incident reduction or audit wins from semantic contracts.
4. Align messaging with standards bodies and OSS maintainers to reinforce the “overlay, not replacement” story.
