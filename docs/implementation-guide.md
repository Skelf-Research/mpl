# MPL Implementation Guide

This guide provides a practical checklist for engineers building the MPL overlay alongside MCP/A2A sessions. Read `docs/protocol-architecture.md` for the canonical technical specification.

**NEW: Before implementing, review `docs/integration-modes.md` to choose your integration path.**

**MPL's integration priority order:**
1. **Sidecar Proxy** (#1, RECOMMENDED) — Zero-code, <30 min, works everywhere. Start here.
2. **Native Integration** (#2, VENDORS) — For MCP/A2A vendors to drive ecosystem adoption.
3. **SDK** (#3, POWER USERS) — Only if you need stateful assertions or custom telemetry.

**Key insight:** 90% of adopters should use **Proxy** and stay with Proxy until vendors add native MPL support. SDK is for specialized use cases only.

The sections below provide technical details applicable across all integration modes.

## 1. Prepare the Semantic Registry

- **Namespace and semver rules:** adopt URNs such as `urn:stype:org.finance.InvestmentRecommendation.v1`; only the major version appears in the wire name, while minor/patch revisions live in schema metadata.
- **Repository layout:** store schemas under `/stypes/{org}/{domain}/{Name}/v{MAJOR}/schema.json` with examples, negative test vectors, semantic notes, and changelogs.
- **Governance automation:** enforce lint checks for duplicate IDs, unresolved `$ref`, missing examples, and unbounded `additionalProperties`. Require CODEOWNERS approval per namespace.
- **Deprecation workflow:** mark STypes as deprecated with sunset dates and upgrade maps; surface flags in the handshake so clients can plan migrations.

## 2. Implement the AI-ALPN Handshake

- **ClientHello payload:** list supported MPL/MCP/A2A protocol versions, models, STypes, tools (with `args_stype`), policies, QoM profiles, and optional feature flags.
- **ServerSelect payload:** return chosen options and downgrade explanations (e.g., tool unavailable, stricter policy required).
- **Telemetry:** log downgrade events and incompatibilities; expose metrics such as downgrade rate and unknown SType rate.
- **Backwards compatibility:** legacy peers can ignore extra fields; MPL-aware runtimes should hard-fail on incompatible SType major versions instead of silently coercing.

## 3. Wrap Messages in the MPL Envelope

Minimum fields:

```json
{
  "id": "uuid",
  "stype": "org.finance.InvestmentRecommendation.v1",
  "payload": { "...": "..." },
  "args_stype": "org.finance.InvestmentRecommendation.v1",
  "profile": "qom-strict-argcheck",
  "sem_hash": "b3:...",
  "provenance": {
    "intent": "advisor.recommend.v1",
    "inputs_ref": ["ctx:risk-assessment.step#3"]
  }
}
```

- Keep transport headers (`Content-Type`) untouched; add `Semantic-Type` and `Semantic-URI` metadata for discovery.
- Encode optional `features` arrays rather than exploding STypes for minor capabilities.
- Include `consent_ref`, `policy_ref`, or `redaction_plan_id` where required by negotiated policy.

## 4. Build the QoM Evaluation Pipeline

- **Metrics:** Schema Fidelity (mandatory), Instruction Compliance (assertion harness), Groundedness (claim support), Determinism under Jitter (sampled reruns), Ontology Adherence (SHACL/OWL checks), Tool Outcome Correctness (post-check).
- **Profiles:** configure thresholds per workflow (e.g., `qom-strict-argcheck` with SF=1.0, IC≥0.97, DJ≥0.95). Attach profile metadata in the handshake and enforce per call.
- **Reporting:** return a `qom_report` payload with metric scores, pass/fail boolean, and references to stored artifacts (claims, diffs, logs).
- **Failure semantics:** emit typed errors (`E-QOM-BREACH`, `E-TOOL-ARG-COERCION`) with hints. Orchestrators should retry or degrade profiles before surfacing to users.
- **Cost controls:** sample expensive checks (e.g., DJ 20% in production) and cache schema validators/ontology shapes.

See `docs/qom-evaluation-engine.md` for the detailed architecture, workflow, and API surface.

## 5. Enforce Semantic Integrity

- **Canonicalization:** deterministically sort keys and normalise types before hashing.
- **Semantic hash:** compute `blake3(canonical_payload)` and include it in messages for drift detection.
- **Optional signatures:** sign the hash with agent keys when provenance or compliance requires tamper evidence.
- **Audit trails:** chain `sem_hash` values to create Merkle-like transcripts for session replay and incident analysis.

## 6. Manage Tool Metadata

- Publish descriptors (`tool.{name}.v{major}.json`) with input/output STypes, supported features, QoM expectations, policy requirements, and implementation bindings (`impl.url`, `impl.type`).
- During handshake, advertise tool IDs and negotiate feature subsets or profiles.
- Provide schema mappers/adapters (`/adapters/{from}->{to}/map.jsonnet`) to bridge version skew across teams.
- Ensure orchestrators cache descriptors, validate task plans against declared STypes, and log provenance per execution.

### 6.1 Adversarial Defenses

MPL provides defense-in-depth against adversarial manipulation of AI agents through schema validation, assertion enforcement, output validation, and semantic integrity checks. Key defensive mechanisms include:

- **Input validation:** Schema Fidelity blocks prompt injection attempts by rejecting unexpected fields or malformed data
- **Business logic enforcement:** Instruction Compliance assertions prevent control evasion and enforce domain constraints
- **Output validation:** Schema Fidelity on responses prevents data exfiltration and jailbreaking attempts
- **Semantic integrity:** BLAKE3 hashes and provenance chains detect tampering and enable forensic analysis
- **Temporal controls:** Assertions can enforce freshness constraints to prevent replay attacks

For comprehensive coverage of adversarial threats and defensive strategies, see `docs/adversarial-robustness.md`. For general security architecture, see `docs/security.md`.

### 6.2 Policy Engine

- **Policy manifests:** store policy definitions (e.g., Rego rules, JSONLogic) in the registry under `/policies/{name}/v{MAJOR}/policy.rego`.
- **Runtime enforcement:** MPL runtimes load negotiated policy references and execute them before dispatching tool calls or after receiving results.
- **Consent cache:** maintain per-subject consent receipts with TTL; attach `consent_ref` to outgoing messages.
- **Redaction plans:** policies can specify redaction templates that the runtime applies to payloads before logging or forwarding.
- **Violation handling:** emit `E-POLICY-DENIED` with remediation hints (e.g., missing consent scope) and optionally trigger workflow escalation.

## 7. Operationalize MPL

- **Observability:** record QoM metrics, downgrade events, unknown SType counts, and semantic checksum mismatches; feed existing telemetry systems.
- **Policy enforcement:** integrate consent receipts, policy engines (Rego-like), and redaction filters defined in the registry.
- **Context hygiene:** adopt typed session logs with TTL/compaction rules; enforce groundedness by requiring citations in retrieval responses.
- **Change management:** simulate releases with conformance suites (positive/negative schema vectors, jitter harness, equivalence fuzzing) before promoting new STypes or tools.

## 8. Adoption Modes and Workstreams

- **Sidecar proxy path:**
  1. Intercept MCP/A2A traffic; add handshake, envelope augmentation, schema validation, and QoM checks.
  2. Maintain translation layer for legacy clients; strip MPL headers if downstream cannot handle them.
  3. Emit telemetry to central observability stack.

- **SDK path:**
  1. Wrap transport client; expose typed methods (`call(tool_id, args_stype, payload, profile)`).
  2. Bake in schema/QoM validators and typed error handling.
  3. Provide helpers for provenance logging and semantic hashing.

- **Native integration path:**
  1. Embed handshake and MPL envelope support in model runtime.
  2. Surface QoM failure signals to user prompts and retrievers.
  3. Offer provider-signed provenance for downstream audit.

## 9. Delivery Priorities

Recommended implementation sequence:

1. Schema Fidelity + Instruction Compliance checks in proxy/SDK.
2. AI-ALPN handshake with downgrade reporting.
3. QoM reporting envelope and typed errors.
4. Registry tooling + governance automation.
5. Semantic hashes/signatures and advanced QoM metrics (Groundedness, Determinism).

Treat each as a shippable milestone to de-risk adoption and gather feedback from design partners before scaling up the protocol surface area.

## 10. Developer Workflow & Interfaces

### 10.1 Registry Management

- **CLI tooling:** ship an `mpl-registry` CLI (or scripts) to scaffold and validate STypes, tools, and profiles.

```bash
$ mpl-registry init my-org.finance
$ mpl-registry add-stype org.finance.InvestmentRecommendation.v1 schema.json --examples examples/*.json
$ mpl-registry lint
$ mpl-registry publish --registry=https://registry.mpl.dev
```

- Commands should:
  - Generate folder layout `/stypes/{org}/{domain}/{Name}/v{MAJOR}/`.
  - Embed examples, negative vectors, semantic notes, and changelog skeletons.
  - Run JSON Schema validation and lint rules locally before pushing PRs.
- **Review workflow:** developers open PRs against the registry repo; CODEOWNERS enforce reviews; CI runs the lint/conformance suite.

### 10.2 Tool Descriptor Authoring

- Developers author `tool.{name}.v{major}.json` manifests via CLI scaffolding:

```bash
$ mpl-registry add-tool advisor.recommend.v1 \
    --args-stype=org.finance.InvestmentRecommendation.v1 \
    --returns-stype=org.finance.InvestmentRecommendation.v1 \
    --policy=policy.ref#fiduciary-duty-v1 \
    --profile=qom-strict-argcheck \
    --impl-url=https://api.example.com/v1/advisor/recommendations
```

- CLI validates references to registered STypes, policies, and QoM profiles.
- Tool manifests are packaged with the MCP/A2A service or exposed via registry APIs for discovery.

### 10.3 QoM Profile Configuration

- Profiles live in the registry (`profiles/{name}.json`). Developers can manage them via CLI or SDK:

```bash
$ mpl-registry add-profile qom-strict-argcheck profile.json
$ mpl-registry lint-profile qom-strict-argcheck
```

- Profiles include metric thresholds, sampling rates, and retry guidance. MPL runtimes load negotiated profiles at startup.

### 10.4 SDK Usage (Client/Server)

- **Client SDK:** thin wrappers expose typed calls and assertions.

```python
from mpl.sdk import Session

session = Session.connect(
    transport="wss://advisor.example.com",
    stypes=["org.finance.InvestmentRecommendation.v1"],
    tools=["advisor.recommend.v1"],
    profile="qom-strict-argcheck"
)

resp = session.call(
    tool="advisor.recommend.v1",
    payload={"symbol": "VOO", "action": "buy", "riskLevel": "moderate", "rationale": "..."}
)

resp.assert_schema()
resp.assert_qom()
```

- **Server SDK:** decorators enforce schema/QoM before invoking tool handlers.

```typescript
// NOTE: TypeScript SDK is Phase 2. Python SDK available in MVP.
import { defineTool } from "@mpl/sdk";

export const generateRecommendation = defineTool({
  id: "advisor.recommend.v1",
  argsStype: "org.finance.InvestmentRecommendation.v1",
  returnsStype: "org.finance.InvestmentRecommendation.v1",
  handler: async ({ payload }) => {
    const recommendation = await advisorAPI.generateRecommendation(payload);
    return recommendation;
  },
});
```

- SDKs surface telemetry hooks (`onQoMResult`, `onDowngrade`) so developers can wire dashboards quickly.

### 10.5 Proxy Configuration

- Ship declarative config for the MPL proxy:

```yaml
transport:
  listen: 0.0.0.0:9443
  upstream: mcp-server:8080
mpl:
  registry: https://registry.mpl.dev
  required_profile: qom-strict-argcheck
  enforce_policies: true
observability:
  metrics_port: 9100
  logs: stdout
```

- Developers manage proxy deployments via helm/terraform, reusing existing MCP infrastructure.

### 10.6 Local Development Loop

1. Scaffold new SType/tool/profile with CLI.
2. Write examples and negative vectors; run `mpl-registry lint`.
3. Implement tool handler or agent logic using SDK helpers.
4. Run local conformance suite (`mpl test --tool advisor.recommend.v1`).
5. Launch MPL proxy/SDK in dev mode (`mpl proxy --mock-qom`).
6. Iterate until QoM and schema checks pass; then submit registry + code changes together.

By providing CLI tools, SDK helpers, and proxy configs, developers gain a tangible interface for managing STypes, QoM profiles, tool metadata, and semantic telemetry without manually editing raw JSON or wiring validators from scratch.
