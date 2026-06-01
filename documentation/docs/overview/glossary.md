# Glossary

## Core Concepts

MPL (Meaning Protocol Layer)
:   A semantic overlay protocol that runs alongside MCP and A2A sessions to make the meaning of every exchange explicit, verifiable, and portable. MPL adds semantic types, capability negotiation, quality metrics, and provenance without replacing underlying transports.

MCP (Model Context Protocol)
:   A client-server protocol for connecting LLM runtimes to tool hosts and resource providers. MCP handles transport, discovery, and invocation but does not enforce semantic contracts or quality guarantees.

A2A (Agent-to-Agent)
:   A peer-to-peer protocol framework for autonomous agent communication. A2A provides routing and capability advertisement but leaves schema governance and quality enforcement to application layers.

---

## Semantic Types

SType (Semantic Type)
:   A globally unique, versioned identifier that declares the intent and schema of a payload. Format: `namespace.domain.Intent.vMajor` (e.g., `org.calendar.Event.v1`). Each SType links to a canonical JSON Schema definition in the MPL registry.

SType Registry
:   A centralized, version-controlled repository of SType definitions, schemas, tool descriptors, QoM profiles, and policy manifests. The registry enforces namespace governance, semver rules, and deprecation workflows.

Semantic URN
:   The canonical identifier format for STypes: `urn:stype:namespace.domain.Intent.vMajor`.

Namespace
:   The organizational scope of an SType (e.g., `org`, `com.acme`). Namespaces are governed by CODEOWNERS rules in the registry to prevent conflicts and ensure accountability.

---

## Handshake & Negotiation

AI-ALPN (AI Application-Layer Protocol Negotiation)
:   A capability negotiation protocol modeled after TLS ALPN. Before exchanging work, peers negotiate protocols, models, STypes, tools, QoM profiles, and policies. Incompatibilities are logged as downgrade events.

ClientHello
:   The initial handshake message sent by a client proposing supported protocols, models, STypes, tools, policies, QoM profiles, and optional feature flags.

ServerSelect
:   The handshake response from a server selecting the capabilities it can honor and explaining any downgrades.

Downgrade
:   When a server cannot satisfy all client-proposed capabilities, it selects a compatible subset and logs the reasons. Downgrade telemetry helps teams track capability drift.

Feature Flag
:   An optional capability extension namespaced to avoid SType explosion (e.g., `ext.qom.determinism@v1`). Features are negotiated during handshake.

---

## Quality of Meaning (QoM)

QoM (Quality of Meaning)
:   A framework for measuring and enforcing semantic quality through observable metrics, negotiated profiles, and actionable breach detection.

QoM Profile
:   A named configuration defining metric thresholds, sampling rates, and retry policies (e.g., `qom-strict-argcheck`, `qom-basic`). Profiles are negotiated during handshake and enforced per message.

Schema Fidelity (SF)
:   **Mandatory metric.** Measures whether a payload conforms to its declared SType schema. Computed via JSON Schema validation. Target: 1.0 (100% conformance).

Instruction Compliance (IC)
:   Measures adherence to explicit assertions or constraints embedded in the request. Computed by executing declarative (CEL, JSONLogic) or imperative assertion scripts. Reported as pass/fail ratio.

Groundedness (G)
:   Measures whether claims in a response are supported by cited sources. Computed by extracting verifiable claims and checking them against provenance artifacts. Reported as supported claims / total claims.

Determinism under Jitter (DJ)
:   Measures output stability when inputs are perturbed (temperature changes, context shuffling). Computed by running K re-executions and measuring semantic similarity.

Ontology Adherence (OA)
:   Measures conformance to domain-specific rules (chronological order, enum membership, cardinality constraints). Validated using SHACL, OWL, or custom rule engines.

Tool Outcome Correctness (TOC)
:   Measures whether tool side effects match expectations. Validated via post-check hooks (read-after-write, external API verification). Reported as pass/fail.

QoM Report
:   A structured payload attached to responses summarizing metric scores, profile evaluation status (`meets_profile`), and references to detailed artifacts.

QoM Breach
:   A typed error (`E-QOM-BREACH`) emitted when a response fails to meet negotiated thresholds. Includes metric values, threshold mismatches, and remediation hints.

---

## Semantic Integrity

Semantic Hash (sem_hash)
:   A BLAKE3 cryptographic hash computed over a canonicalized payload. Used to detect meaning drift across retries, hops, or storage/replay scenarios.

Canonicalization
:   The process of deterministically normalizing a payload (sorted keys, consistent encoding) before hashing. Ensures semantic equivalence checks are reliable.

Provenance
:   Metadata tracking the origin, intent, and transformation chain of a payload. Includes `intent`, `inputs_ref`, `consent_ref`, and optional signatures.

---

## Envelopes & Messages

MPL Envelope
:   The semantic wrapper around payloads transmitted over MCP/A2A. Core fields: `id`, `stype`, `payload`, `args_stype`, `profile`, `sem_hash`, `provenance`, optional `qom_report`.

Typed Call
:   A tool or agent invocation carrying an MPL envelope. The `stype` and `args_stype` fields declare input/output semantics.

Typed Error
:   A structured error response with a semantic error code (e.g., `E-QOM-BREACH`, `E-SCHEMA-FIDELITY`, `E-POLICY-DENIED`). Errors include hints for remediation.

---

## Policy & Governance

Policy Engine
:   A rule-based enforcement system that evaluates policies against messages at runtime. Supports allow/deny decisions, QoM profile overrides, and consent requirements.

Policy Manifest
:   A declarative definition specifying consent requirements, redaction rules, or access controls. Stored in the registry.

Consent Reference (consent_ref)
:   A pointer to a stored consent receipt indicating user authorization for data use. Included in provenance when sharing user-linked data.

---

## Deployment Models

Sidecar Proxy
:   A deployment pattern where MPL logic runs in a separate process alongside the MCP/A2A client or server. Enables zero-code integration.

MPL SDK
:   A client library embedding handshake negotiation, schema validation, QoM enforcement, and provenance logging. Available for Python and TypeScript.

Native Integration
:   When an LLM runtime or MCP/A2A framework natively understands MPL envelopes and handshakes without proxies or SDK wrappers.

---

## Error Codes

| Code | Description |
|------|-------------|
| `E-QOM-BREACH` | QoM metrics failed negotiated thresholds |
| `E-SCHEMA-FIDELITY` | Payload failed schema validation |
| `E-TOOL-ARG-COERCION` | Tool arguments don't match `args_stype` |
| `E-POLICY-DENIED` | Request violated negotiated policy |
| `E-UNKNOWN-STYPE` | SType not found in registry |
| `E-UNKNOWN-TOOL` | Tool not available |
| `E-NEGOTIATION-INCOMPATIBLE` | Handshake failed—no compatible capabilities |

---

## Abbreviations

| Abbreviation | Full Form |
|-------------|-----------|
| ALPN | Application-Layer Protocol Negotiation |
| CEL | Common Expression Language |
| DJ | Determinism under Jitter |
| G | Groundedness |
| IC | Instruction Compliance |
| OA | Ontology Adherence |
| QoM | Quality of Meaning |
| SF | Schema Fidelity |
| SLO | Service Level Objective |
| SType | Semantic Type |
| TOC | Tool Outcome Correctness |
| URN | Uniform Resource Name |
