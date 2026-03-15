# MPL Roadmap & Validation Plan

MPL’s rollout is structured as a startup hypothesis machine. We make explicit beliefs, design fast tests, and tie milestones to the results. Success means we can ship an overlay that teams adopt alongside MCP/A2A; failure quickly redirects scope or audience.

## 1. Beliefs (Explicit Assumptions)

1. **Semantic contracts matter.** Teams building on MCP/A2A want typed meaning (STypes), QoM, and provenance to avoid flaky multi-tool flows.
2. **Overlay beats replacement.** A drop-in MPL proxy/SDK will be adopted faster than a wholesale protocol swap.
3. **Enterprise pull exists.** Regulated buyers (finance/health/legal) will pay for semantic guarantees (QoM SLOs, policy, audit).
4. **Minimal friction wins.** If we add one handshake and a slim envelope, developers will accept it; heavier changes stall adoption.
5. **Ecosystem effects accrue.** A public SType + tool registry with governance creates network effects and reduces integration cost.
6. **Measurable quality sells.** A QoM score/SLO pass rate becomes a KPI stakeholders demand, similar to uptime.

## 2. Validation Tests

| Test | Target | Definition | Success Criteria | Kill Criteria |
| ---- | ------ | ---------- | ---------------- | ------------- |
| **T1 – Overlay Fit** | 2 weeks | Sidecar proxy wrapping existing MCP server | Time-to-first-typed-call < 30 min, breakage rate <5% | Median >60 min → cut envelope features |
| **T2 – Dev Ergonomics** | 2 weeks | SDK alpha (Python/TS) with typed tool flow | 10 external devs succeed in ≤100 LOC, DX NPS ≥ +30 | NPS < +10 → reduce handshake scope |
| **T3 – Enterprise Pull** | 4–6 weeks | 2 pilots (finance + healthcare) | ≥1 paid PoC tied to QoM SLOs | 0/2 paid → pivot ICP (platform/tool vendors) |
| **T4 – Registry Value** | 3 weeks | Publish 12 STypes + 6 tools | ≥5 third-party PRs without hand-holding | <3 PRs → seed adapters, publish opinionated profiles |
| **T5 – QoM Demand** | 3 weeks | QoM report widget in dashboards | ≥3 teams adopt in QA/CI gate | Teams hide widget → rework metrics tied to real incidents |
| **T6 – Backwards Compatibility** | Ongoing | Proxy against Claude MCP + popular A2A lib | Downgrade/compat errors <2% sessions | >5% incompat → ship official adapters/mappers |

## 3. Milestones

- **M0 (Day 0–30):** Overlay proxy + SDK alpha, public demo repo, first design partner. *Miss DX targets? Simplify envelope/handshake.*
- **M1 (Day 31–60):** Two pilots live, registry governance CI, QoM v0 (Schema Fidelity + Instruction Compliance). *No paid PoC? Shift ICP to platforms/tool vendors.*
- **M2 (Day 61–90):** Conformance suite v1, QoM expands (Groundedness/Determinism sampling), pricing experiment. *Registry contributions stall? Publish curated profiles + adapters.*
- **M3 (Day 91–180):** Five paying customers, >25 STypes, >15 tools, weekly ecosystem PRs. *Sales cycle too long? Launch “MPL Lite” (schema-only) SKU.*

## 4. Assumption Board

| Assumption | Status | Linked Test | Kill Criteria |
| ---------- | ------ | ----------- | ------------- |
| Overlay integrates in <30 min | 🧪 Untested | T1 | Median >60 min → cut features until fit |
| Devs accept handshake/envelope | ❓ Assumed | T2 | NPS < +10 → reduce handshake scope |
| Enterprises pay for QoM/Audit | ❓ Assumed | T3 | 0/2 paid PoCs → change ICP or bundle with existing observability |
| Registry attracts contributions | 🧪 Untested | T4 | <3 community PRs → seed adapters, publish profiles |
| QoM shows visible value | 🧪 Untested | T5 | Teams hide widget → tie metrics to real incidents |
| MCP/A2A compatibility holds | ❓ Assumed | T6 | >5% incompat → build official adapters/mappers |

Update this board weekly; move assumptions from “Assumed” to “Proven” or “Retired” as tests complete.

## 5. MVP Scope

- **MPL Sidecar Proxy (WS/HTTP):** AI-ALPN handshake, `stype`/`args_stype`, minimal QoM (Schema Fidelity + Instruction Compliance), provenance stub.
- **SDK Alpha (TS/Python):** typed envelope helpers, schema validation, QoM assertion hooks.
- **Registry Seed:** `org.calendar.*`, `eval.RAGQuery.v1`, `agent.TaskPlan.v1`, plus three tool descriptors (calendar read/create, knowledge search).
- **Conformance Mini-Suite:** schema positive/negative cases, single jitter test recipe.

## 6. GTM Hypotheses → Experiments

- **ICP #1 – AI platform teams in regulated SaaS.** *Experiment:* paid PoC with QoM SLO (SF=1.0, IC ≥ 0.97) on one workflow; price as a share of “incident cost avoided” or per developer seat.
- **ICP #2 – Framework/tool vendors (LangChain-class).** *Experiment:* OEM/OSS module; success when MPL ships as default “strict mode.”
- **ICP #3 – Agent app builders (B2B automation).** *Experiment:* free MPL Lite (schema-only) with upsell to QoM/policy packs.

**Pricing tests:** compare seat-based ($30–$50/dev/month), usage-based (per 1k validated calls), and control-plane SaaS (QoM dashboards, policy packs).

## 7. Momentum Metrics

- Time-to-first-typed-call (p50/p90).
- Schema fidelity rate (target ≥99.5%).
- QoM SLO pass rate per workflow.
- Risk approval lead time (target: cut by 50% once QoM reports adopted).
- Handshake downgrade rate (target <5%).
- Unknown SType rate (target <0.1%).
- Registry contribution velocity (PRs/month).
- Proxy adoption (# services with sidecar).
- Paid PoCs started/closed.

## 8. Riskiest Assumption Tests

1. **“Do developers care?”** Break an MCP flow for 10 teams; half use MPL proxy, half do not. Measure fix time and retry count.
2. **“Will they pay?”** Offer two pilots with identical features; one includes QoM gating + audit report at a price premium. Observe selection and willingness to pay.
3. **“Interoperability or bust?”** Run MPL proxy with Claude MCP and a mainstream A2A library; publish compatibility matrix and downgrade causes.

## 9. Execution Timeline

**Next 30 days**
- Ship sidecar proxy + SDK alpha.
- Seed registry (12 STypes, 6 tools).
- Launch public calendar workflow demo.
- Book two design partners and define their QoM SLOs.

**Day 31–60**
- Run two PoCs; integrate QoM widgets into partner dashboards.
- Add adapters/mappers for version gaps.
- Publish conformance mini-suite and invite community runs.

**Day 61–90**
- Close at least one paid PoC.
- Expand QoM with Groundedness and Determinism sampling.
- Establish governance CI (lint, CODEOWNERS, deprecation warnings).
- Publish QoM profiles (calendar-minimal, RAG-basic).

## 10. Success & Adaptation Criteria

- **H1:** Typed meaning + QoM reduces incident rate ≥30%. *Test via PoC comparisons; if unmet, re-evaluate metric mix and tooling.*
- **H2:** Negotiated handshake keeps downgrade rate <5%. *If higher, invest in adapters or loosen required features.*
- **H3:** Provenance/QoM become release criteria in ≥2 orgs. *If not, bundle QoM instrumentation with deployment tooling.*

The roadmap is intentionally dynamic. Each completed test should cycle back into this document with outcomes, learnings, and revised hypotheses.
