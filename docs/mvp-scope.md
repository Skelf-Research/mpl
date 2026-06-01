# MPL MVP Scope & Implementation Roadmap

Based on the feasibility analysis, this document defines a pragmatic MVP scope that balances ambition with implementability.

## Quick Reference

| Component | MVP Status | Rationale |
|-----------|-----------|-----------|
| ✅ **STypes + Schema Validation** | INCLUDE | Core value prop, low risk, well-understood tech |
| ✅ **AI-ALPN Handshake** | INCLUDE | Essential for negotiation, straightforward to implement |
| ✅ **Instruction Compliance** | INCLUDE | High value, manageable complexity (JSONLogic/CEL only) |
| ✅ **Registry (Git-based)** | INCLUDE | GitHub repo sufficient for MVP, defer custom API |
| ✅ **Python SDK** | INCLUDE | Primary target language, enables fast adoption |
| ✅ **Sidecar Proxy** | INCLUDE | Zero-code integration path for legacy systems |
| ⚠️ **Ontology Adherence** | DEFER to Phase 2 | JSONLogic rules sufficient; SHACL/OWL too complex |
| ⚠️ **Tool Outcome Correctness** | DEFER to Phase 2 | Nice-to-have, adds latency, optional use case |
| ❌ **Groundedness** | EXCLUDE | Research-grade, unreliable, ML infrastructure overhead |
| ❌ **Determinism under Jitter** | EXCLUDE | Too expensive (10x cost), questionable value |
| ❌ **Full Policy Engine** | EXCLUDE | 6-12 month project, defer to Phase 2 |
| ❌ **Consent Management** | EXCLUDE | Separate product, partner with existing vendors |

**Key insight:** MVP provides **foundational** compliance value (audit trails + controls-as-code) that enables pilot approval without full policy engine. This addresses the #1 compliance blocker: "can we prove the agent followed our rules?" Full policy enforcement (consent, redaction, regional restrictions) deferred to Phase 2. See `docs/regulated-enterprise-value.md` for detailed regulatory mapping.

---

## Phase 1: MVP (3-6 months, 3 engineers)

### Goals
1. Prove the "overlay" model works (MPL on MCP)
2. Demonstrate schema validation + basic QoM enforcement
3. Achieve time-to-first-typed-call <30 minutes
4. Sign 3-5 design partners

### Deliverables

#### 1. Core Protocol
- [x] SType specification (JSON Schema only, no Protobuf)
- [x] AI-ALPN handshake messages (ClientHello, ServerSelect)
- [x] MPL envelope structure (typed payloads + provenance)
- [x] Error taxonomy (E-SCHEMA-FIDELITY, E-QOM-BREACH, etc.)
- [x] **Wire format specification** (JSON, UTF-8, no compression)

#### 2. Registry
- [x] GitHub-based registry (no custom API)
  - Structure: `/stypes/{namespace}/{domain}/{Name}/v{major}/schema.json`
  - CODEOWNERS for namespace governance
  - GitHub Actions CI for schema validation
- [x] Starter registry with 25+ STypes:
  - `org.calendar.*` (Event, Query, Reminder)
  - `eval.*` (RAGQuery, RAGResponse, SearchResult, Feedback)
  - `data.*` (Table, Record, Query, FileMetadata)
  - `org.*` (Step, Pipeline, Profile, Alert, Rating, AccessControl)
  - `ai.*` (Template, Response, Reasoning)
- [x] CLI tooling (`mplx`)
  - `validate`, `conformance`, schema validation
  - JSON Schema validation + lint rules
  - Example/negative test runner

#### 3. QoM Engine (Minimal)
- [x] Schema Fidelity (mandatory, 2-5ms p50, 20-50ms p99)
  - JSON Schema validation (jsonschema for Rust/Python, ajv for TS)
  - Caching for performance (critical for p50)
- [x] Instruction Compliance (optional, 5-20ms depending on complexity)
  - JSONLogic only (no arbitrary scripts)
  - 100ms timeout per assertion
  - Pass/fail + details reporting
  - Note: Complex assertions with context lookups may reach 50ms
- [x] QoM reporting envelope
  - `qom_report` structure
  - Per-metric scores
  - Artifact references (logs, traces)

#### 4. SDK (Python)
- [x] Client SDK
  ```python
  from mpl import Session

  session = Session.connect(
      transport="wss://mcp.example.com",
      stypes=["org.calendar.Event.v1"],
      profile="qom-basic"
  )

  resp = session.call(
      tool="calendar.create.v1",
      payload={...}
  )

  assert resp.qom_report.meets_profile
  ```
- [x] Server SDK
  ```python
  from mpl import defineTool

  @defineTool(
      id="calendar.create.v1",
      args_stype="org.calendar.Event.v1"
  )
  async def create_event(payload):
      return await db.insert(payload)
  ```
- [x] Telemetry hooks (onQoM, onDowngrade)

#### 5. Sidecar Proxy
- [x] Intercepts MCP WebSocket/HTTP traffic
- [x] Performs handshake negotiation
- [x] Validates schemas + runs QoM checks
- [x] Config-driven (YAML)
  ```yaml
  transport:
    listen: 0.0.0.0:9443
    upstream: mcp-server:8080
  mpl:
    registry: https://github.com/Skelf-Research/mpl/raw/main/registry
    profile: qom-basic
  ```
- [x] Prometheus metrics export
- [x] Structured logging (JSON)

#### 6. Documentation
- [x] Protocol architecture (done)
- [x] Implementation guide (done)
- [x] Integration modes (done)
- [x] MPL vs. Schema Registries (done)
- [x] QoM evaluation engine (done)
- [x] Security model (done)
- [x] Adversarial robustness (done)
- [x] Regulated enterprise value (done)
- [x] AI Safety & Risk alignment (done)
- [x] Glossary (done)
- [x] **MVP scope document** (this file)
- [x] **Getting Started guide** (`docs/getting-started.md`)
- [ ] **Migration guide** (MCP → MPL)

#### 7. Examples
- [x] Calendar workflow (`examples/tutorials/calendar-workflow/`)
- [x] RAG query workflow (`examples/tutorials/rag-workflow/`)
- [x] Multi-agent task delegation (`examples/tutorials/multi-agent/`)

---

## What's Explicitly Out of Scope for MVP

### ❌ Advanced QoM Metrics
- **Groundedness:** Unreliable (70-85% accuracy), requires ML infrastructure (GPU, models), adds 200-500ms latency
- **Determinism under Jitter:** Too expensive (K=3 reruns = 3x cost), adds 1-5s per sampled request
- **Rationale:** Core value is schema + assertions; advanced metrics are research-grade

### ❌ Full Policy Engine
- **OPA integration:** Requires Rego expertise, policy authoring, sidecar management
- **Consent management:** Separate product (consent store, UI, webhooks, audit)
- **Regional compliance:** Legal review required, moving target
- **Rationale:** 6-12 month effort; defer to Phase 2 or partner integrations

### ❌ SHACL/OWL Ontologies
- **RDF/triple-store infrastructure:** Complex, niche use case
- **Developer experience:** Few devs know SHACL
- **Rationale:** JSONLogic rules sufficient for 90% of validation needs

### ❌ Protobuf Support
- **Wire format:** JSON sufficient for MVP; Protobuf is optimization
- **Rationale:** Adds complexity; defer to Phase 2 after validating demand

### ❌ Custom Registry API
- **REST/GraphQL API:** GitHub raw URLs sufficient for MVP
- **Search:** "grep the repo" good enough for <100 STypes
- **Rationale:** Premature optimization; validate need with usage data

---

## Success Metrics (MVP)

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Time-to-first-typed-call** | <30 min | User installs proxy, sends typed request, receives QoM report |
| **Schema validation overhead** | 2-5ms p50, 20-50ms p99 | Proxy telemetry (with caching) |
| **QoM evaluation overhead** | 10-30ms p50, 100-200ms p99 | (SF + IC only, typical assertions) |
| **Design partner adoption** | 3-5 partners | Using in staging/production |
| **Registry size** | 50+ STypes | Community contributions |
| **Developer satisfaction** | NPS ≥ +30 | SDK alpha survey |
| **Downgrade rate** | <5% | Handshake telemetry |
| **Unknown SType rate** | <0.1% | Proxy metrics |

---

## Phase 2: Production Hardening (6-12 months) ✅ COMPLETE

### Goals
1. ✅ Expand QoM metrics (Tool Outcome Correctness)
2. ✅ Add registry API (REST + caching)
3. ✅ Multi-language SDKs (TypeScript, Go)
4. ✅ Policy engine lite (consent_ref validation)
5. 🚧 Conformance test suite (in progress)
6. 🚧 A2A integration (in progress)

### Deliverables
- [x] Tool Outcome Correctness (post-check hooks) - `crates/mpl-protocol/src/qom.rs`
- [x] Ontology Adherence (JSONLogic rules)
- [x] Registry API (REST + Moka cache) - `crates/mpl-registry-api/`
- [x] TypeScript SDK - `typescript/`
- [ ] Go SDK (deferred - Rust covers performance needs)
- [x] Policy engine lite - `crates/mpl-protocol/src/policy.rs`:
  - Rule-based enforcement with SType patterns
  - Access control (allow/deny lists)
  - QoM profile overrides per namespace/domain
  - Rate limiting configuration
  - Custom constraints (metadata checks, payload size)
- [x] Conformance test suite (42+ test cases, expanding)
- [ ] A2A integration guide + examples (in progress)
- [x] Helm chart for Kubernetes deployment - `helm/mpl-proxy/`

### Success Metrics
- 20+ production deployments (tracking)
- 25+ STypes in registry ✅
- 5+ tool vendors integrating MPL (tracking)
- 99.9% registry uptime (tracking)
- <100ms p99 registry latency ✅

---

## Phase 3: Ecosystem Scale (12-24 months)

### Goals
1. Full policy engine (OPA + consent management)
2. Advanced QoM metrics (if validated)
3. Federated registries
4. MPL Cloud (managed SaaS)

### Conditional Features
- **Groundedness:** Only if ML research shows production-ready approach (90%+ accuracy, <100ms latency)
- **Determinism:** Only if strong user demand + acceptable cost model (async checks?)
- **SHACL/OWL:** Only if domain experts request (healthcare, finance verticals)

---

## Implementation Order (MVP) ✅ COMPLETE

All MVP milestones have been achieved:

### Week 1-2: Foundations ✅
- [x] Define wire format (JSON schema)
- [x] Implement canonicalization + semantic hash
- [x] Build SType registry (GitHub setup)
- [x] Publish 25+ core STypes (calendar, eval, data, org, ai)

### Week 3-4: Schema Validation ✅
- [x] Implement Schema Fidelity validator (Rust + Python bindings)
- [x] Add caching layer
- [x] Build error reporting (E-SCHEMA-FIDELITY with paths)
- [x] Write unit tests (100+ positive/negative cases)

### Week 5-6: Handshake ✅
- [x] Implement AI-ALPN messages (ClientHello, ServerSelect)
- [x] Add downgrade telemetry
- [x] Build handshake state machine
- [x] Write integration tests

### Week 7-8: QoM Engine ✅
- [x] Implement Instruction Compliance (JSONLogic)
- [x] Add QoM reporting envelope
- [x] Build QoM profiles (qom-basic, qom-strict-argcheck, qom-outcome, qom-comprehensive)
- [x] Add sampling controls

### Week 9-10: SDK ✅
- [x] Build Python SDK (PyO3 bindings)
- [x] Build TypeScript SDK
- [x] Add telemetry hooks
- [x] Write SDK examples + tests

### Week 11-12: Proxy ✅
- [x] Build sidecar proxy (intercepts MCP traffic)
- [x] Add YAML config support
- [x] Implement Prometheus metrics
- [x] Add structured logging

### Week 13-14: Documentation & Examples ✅
- [x] Write Getting Started guide
- [x] Create calendar workflow tutorial
- [x] Create RAG workflow tutorial
- [x] Create multi-agent tutorial
- [x] Publish Docker Compose setup (one-command deploy)

### Week 15-16: Validation 🚧 IN PROGRESS
- [ ] Recruit 3-5 design partners
- [ ] Run validation experiments (time-to-first-call, NPS)
- [ ] Collect feedback, iterate
- [ ] Prepare v0.1 release

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| **No one uses the registry** | Pre-populate with 30+ common STypes; provide importers (OpenAPI → SType) |
| **Performance overhead too high** | Benchmark early; optimize hot paths; add circuit breakers |
| **Adoption friction** | Build zero-config proxy mode; rich error messages; video tutorials |
| **Ecosystem lock-in fears** | Open-source everything; no vendor lock-in; clear governance model |
| **Security/adversarial concerns** | Comprehensive threat model + defenses documented (`docs/adversarial-robustness.md`); 80-90% protection against common attacks in MVP; regular security audits; bug bounty |
| **Regulated enterprise skepticism** | Concrete compliance value demonstrated (`docs/regulated-enterprise-value.md`); regulatory framework mappings (SOX, GDPR, HIPAA, EU AI Act, UK FCA/PRA); design partner validation |

---

## Next Steps

**CRITICAL: Read `docs/market-assessment.md` BEFORE proceeding with development.**

The market assessment shows:
- ✅ MPL is valuable (7/10) with real pain validation
- ⚠️ Market is 12-18 months early (agents not at scale yet)
- 🎯 Requires strategic execution (design partners, registry seeding, vendor partnerships)

**Only proceed with MVP if:**
1. You can recruit 10-20 design partner candidates (target: 3-5 active adopters in staging/production)
2. You can seed registry with 50-100 STypes proactively ($50-100k)
3. You can pursue 2-3 vendor partnership conversations (Anthropic/MCP, A2A)
4. You have 18-24 month runway for PMF validation

**If validation criteria are met:**
1. **Review this scope with stakeholders** (architecture, product, engineering)
2. **Finalize MVP feature set** (lock scope, no additions)
3. **Staff the project** (3 engineers: 1 protocol, 1 SDK/proxy, 1 registry/docs)
4. **Set up infra** (GitHub org, registry repo, CI/CD, monitoring)
5. **Kick off Week 1** (foundations + wire format spec)

---

## Appendices

### A. Technology Stack (MVP)

| Component | Technology | Rationale |
|-----------|-----------|-----------|
| **Registry** | GitHub + GitHub Pages | Free, reliable, familiar to developers |
| **SDK (Python)** | Python 3.10+, asyncio | Target audience (AI/ML engineers) |
| **Proxy** | Python (aiohttp) | Rapid prototyping, easy integration with SDK |
| **Schema validation** | jsonschema (Python) | Mature, fast, widely used |
| **Assertion engine** | jsonlogic-py | Simple, sandboxed, no arbitrary code |
| **Hashing** | BLAKE3 (blake3-py) | Fast, secure, standardized |
| **Telemetry** | Prometheus + OpenTelemetry | Industry standard, excellent ecosystem |
| **Config** | YAML (pyyaml) | Human-readable, widely adopted |

### B. Development Resources

**Team:** 3 engineers, 3-6 months

**Engineer 1 (Protocol):**
- Wire format + canonicalization
- AI-ALPN handshake
- Error taxonomy
- 50% time

**Engineer 2 (SDK/Proxy):**
- Python SDK (client + server)
- Sidecar proxy
- Telemetry + observability
- 100% time

**Engineer 3 (Registry/Docs):**
- GitHub registry setup
- CLI tooling (mpl-registry)
- Documentation + examples
- Design partner support
- 100% time

**Budget estimate:** $300-600K (loaded cost, 3-6 months)

### C. Phase 1 Deliverables Checklist

Copy this to project tracker:

```markdown
## Protocol & Spec
- [ ] Wire format specification (JSON, UTF-8)
- [ ] SType naming convention (urn:stype:namespace.domain.Name.vMajor)
- [ ] AI-ALPN handshake (ClientHello, ServerSelect)
- [ ] MPL envelope structure
- [ ] Error codes (E-SCHEMA-FIDELITY, E-QOM-BREACH, E-UNKNOWN-STYPE)
- [ ] Canonicalization algorithm (BLAKE3)

## Registry
- [ ] GitHub repo setup (/stypes/, /tools/, /profiles/)
- [ ] CODEOWNERS + PR approval workflow
- [ ] CI: schema validation + lint checks
- [ ] Starter STypes (30+ schemas)
- [ ] Tool descriptors (10+ tools)
- [ ] QoM profiles (qom-basic, qom-strict-argcheck)

## QoM Engine
- [ ] Schema Fidelity validator
- [ ] Instruction Compliance (JSONLogic)
- [ ] QoM reporting envelope
- [ ] Profile loading + enforcement
- [ ] Sampling controls
- [ ] Unit tests (100+ cases)

## SDK (Python)
- [ ] Client SDK (Session, call, assert_qom)
- [ ] Server SDK (defineTool decorator)
- [ ] Telemetry hooks (onQoM, onDowngrade)
- [ ] Examples (calendar, RAG)
- [ ] Unit + integration tests

## Proxy
- [ ] MCP traffic interception (WebSocket + HTTP)
- [ ] Handshake negotiation
- [ ] Schema + QoM validation
- [ ] YAML config support
- [ ] Prometheus metrics
- [ ] Structured logging
- [ ] Docker image
- [ ] Helm chart (optional)

## Documentation
- [ ] Getting Started guide
- [ ] Migration guide (MCP → MPL)
- [ ] SDK reference docs
- [ ] Proxy configuration guide
- [ ] Calendar workflow tutorial
- [ ] Demo video (5 min quickstart)

## Validation
- [ ] Recruit 3-5 design partners
- [ ] Validation experiments (TTFTC, NPS, overhead)
- [ ] Feedback collection + iteration
- [ ] v0.1 release announcement

## Infrastructure
- [ ] GitHub org + repo setup
- [ ] CI/CD pipelines (GitHub Actions)
- [ ] Docker registry (GHCR or DockerHub)
- [ ] Monitoring (Grafana + Prometheus)
- [ ] Issue tracker + project board
```

---

**Last updated:** 2025-12-11
**Owner:** MPL Core Team
**Status:** Phase 1 & 2 Complete, Phase 3 In Progress
