# MPL Protocol Architecture

MPL layers semantic guarantees on top of existing MCP/A2A transports. This document defines the core architectural pillars and serves as the canonical technical reference for the protocol.

## 1. Semantic Grounding Layer

- **Semantic Types (STypes):** globally unique, versioned identifiers (`namespace.domain.Intent.vMajor`) that describe meaning, where only the major version appears in the identifier (e.g., `org.finance.InvestmentRecommendation.v1`). Minor and patch versions are tracked in schema metadata. Each SType links to a canonical schema (JSON Schema or Protobuf) and optional ontology references (RDF/OWL/SHACL).
- **Dual typing:** transports keep `Content-Type` for syntax; MPL adds `Semantic-Type` metadata plus an `stype` field in the payload.
- **Registry governance:** namespaces, semver rules, deprecation policies, and change logs live in the public MPL registry. Clients can fetch and cache schema definitions via immutable URIs.
- **Canonicalisation:** payloads can be normalised into a deterministic form for hashing, diffing, and schema validation.

## 2. Capability & Vocabulary Negotiation (AI-ALPN)

- **Handshake messages** mirror TLS ALPN: the client proposes supported protocols, models, STypes, tools, policies, QoM profiles, and quality-of-service expectations; the server selects a compatible subset.
- **Downgrade telemetry:** every handshake result is logged with downgrade reasons (e.g., tool unavailable, policy mismatch).
- **Feature flags & namespacing:** optional extensions are namespaced (`ext.qom.determinism@v1`) to avoid capability explosion; clients default to the minimal viable set.

## 3. Quality of Meaning (QoM)

- **Metric suite:** Schema Fidelity (mandatory), Instruction Compliance, Groundedness, Determinism under Jitter, Ontology Adherence, Tool Outcome Correctness.
- **Profiles:** reusable QoM “contracts” (e.g., `qom-strict-argcheck`, `qom-basic`) define thresholds and sampling policies. Negotiated during handshake, enforced per message.
- **Evaluation pipeline:** orchestrators validate payloads, run assertions, verify citations, sample jitter runs, and record pass/fail with actionable hints. See `docs/qom-evaluation-engine.md` for architecture and API details.
- **Reporting envelope:** responses can embed a `qom_report` object summarising metrics, profile evaluation, and references to longer-form artifacts.

## 4. Semantic Integrity & Provenance

- **Semantic checksums:** BLAKE3 hashes over canonical payloads detect meaning drift across retries and multi-hop flows.
- **Signatures:** optional signatures bind agent identity to semantic hashes for provenance/audit.
- **Error taxonomy:** typed error codes (`E-QOM-BREACH`, `E-TOOL-ARG-COERCION`) differentiate semantic failures from transport issues.
- **Adversarial defenses:** Schema validation, assertion enforcement, and semantic integrity checks provide defense-in-depth against prompt injection, jailbreaking, data exfiltration, and other agent manipulation attacks. See `docs/adversarial-robustness.md` for detailed threat model and defensive strategies.

## 5. Tool & Schema Lifecycle

- **Typed tool descriptors:** each tool advertises argument/return STypes, version bounds, and policy requirements. Metadata is versioned and discoverable via the registry.
- **Policy bindings:** tool manifests include required policies (`policy.ref#...`), consent scopes, and redaction plans so MPL runtimes enforce them automatically (see `docs/implementation-guide.md#62-policy-engine`).
- **Change management:** deprecation notices, compatibility matrices, and conformance tests guard against SType drift and schema sprawl.
- **Observability hooks:** QoM metrics, downgrade events, policy violations, and unknown SType rates feed existing monitoring systems.

## 6. Transport Independence

- MPL assumes reliable transports (HTTP, WebSocket, gRPC) and focuses on semantics; it can run atop MQTT-style brokers if higher-level negotiation is preserved.
- Clients can adopt MPL incrementally: reuse existing sessions, wrap payloads with MPL headers, negotiate QoM only for critical workflows.
- Sidecar proxies or SDKs mediate between legacy clients and MPL-aware services, enabling gradual rollout.

## 7. Minimal Message Flow

1. **Handshake:** negotiate protocols, capabilities, QoM profile.
2. **Typed call:** send payload with `Semantic-Type`, `stype`, schema version, optional provenance hash.
3. **Validation & response:** server/tool executes, validates against agreed schema/QoM, returns payload + `qom_report` or typed error.
4. **Telemetry:** both sides log QoM metrics, downgrade causes, and semantic hashes for audit.

## 8. Implementation Priorities

Ship in the following order to derisk adoption:

1. **Schema fidelity + instruction compliance** in the proxy/SDK.
2. **Handshake negotiation** with downgrade tracking.
3. **QoM reporting** (structured envelope).
4. **Registry tooling** with governance automation.
5. **Semantic hashes & signatures** for high-assurance domains.

Refer back to `docs/roadmap.md` for the staged delivery plan and validation experiments.
