# Challenges With Today’s MCP/A2A Stacks

MPL is designed as an overlay, not a replacement. To motivate the work, we catalogue the systemic gaps we observe when teams run pure MCP or A2A integrations at scale. Each challenge below ties to a capability MPL introduces.

## 1. Meaning Is Implicit and Brittle

- **Loose JSON everywhere.** Payloads rarely ship with canonical schemas; tooling depends on human convention instead of enforceable contracts.
- **Silent breaking changes.** When servers ship new fields or change semantics, clients discover incompatibilities in production—often after multi-step workflows have mutated state.
- **Sparse provenance.** Debugging “what was intended vs what the model did” requires spelunking through logs rather than inspecting a typed contract.

**MPL response:** versioned Semantic Types (STypes) with registry-backed schemas and provenance hashes. Schema fidelity and ontology adherence become first-class SLOs.

## 2. Capability Negotiation Is Ad Hoc

- **Handshake ambiguity.** MCP clients learn about tool availability only after first failure; A2A frameworks diverge in how they describe model/tool vocabularies.
- **No downgrade detection.** When a server silently drops to a weaker capability set, clients have no telemetry to understand the change.
- **Policy blind spots.** Safety constraints, consent requirements, and QoM expectations are kept in playbooks, not negotiated by the stack.

**MPL response:** an AI-ALPN style negotiation that explicitly selects protocol version, models, STypes, tools, QoM profiles, and policies up front—plus telemetry for downgrade events.

## 3. Quality Is Unmeasured

- **“Works on my prompt.”** Teams rely on manual QA to validate agent outputs; regressions slip through because no structural metric protects the workflow.
- **No shared vocabulary for quality.** MCP/A2A say nothing about schema fidelity, groundedness, or deterministic behaviour—yet these are the axes that break downstream integrations.
- **Incident retros without data.** When a tool call misfires, there is no contract to show whether the agent, registry, or policy logic violated expectations.

**MPL response:** Quality of Meaning (QoM) profiles with measurable metrics:
- **MVP (Phase 1):** Schema Fidelity, Instruction Compliance
- **Phase 2+:** Groundedness, Determinism under Jitter, Ontology Adherence, Tool Correctness

All metrics have negotiable thresholds and clear breach semantics. See `docs/mvp-scope.md` for phased feature rollout and `docs/implementation-feasibility-analysis.md` for technical feasibility assessment.

## 4. Governance and Ecosystem Friction

- **Custom schemas per team.** Without shared registries, each integration reinvents schema definitions and tool metadata, preventing reuse and ecosystem growth.
- **Untracked lifecycle changes.** Deprecations and version bumps are social processes; SType drift accumulates, breaking compatibility across organisations.
- **Audit requirements unmet.** Regulated industries require evidence of semantic controls and policy compliance—capabilities absent from current stacks.
- **Risk sign-off delays.** Compliance teams rely on manual checklist reviews because there is no machine-verifiable quality or policy evidence, delaying production launches.

**MPL response:** a curated public registry with namespace governance, semantic checksums, and policy profiles that can be inspected and audited.

## 5. Operational Complexity

- **Triage overload.** Support teams face long MTTR because semantic mismatches masquerade as transport failures.
- **Context bloat.** Evidence rot and prompt sprawl increase latency and cost but there is no mechanism to enforce minimality.
- **Tooling gap.** Observability tools measure latency/uptime, not semantic correctness.

**MPL response:** typed envelopes with structured telemetry, downgrade/error taxonomies, and optional semantic signatures that route incidents to the root cause quickly.

---

## Why an Overlay Wins

We believe teams adopt MPL faster than a greenfield protocol because it:

- Works with existing transports (reuse MCP WebSockets/A2A channels).
- Adds one handshake and a compact envelope, respecting developer tolerance for friction.
- Provides immediate value—typed payloads, QoM scores, provenance—without rewriting orchestrators.

These hypotheses are being validated through the experiments defined in `docs/roadmap.md`.
