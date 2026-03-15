# MPL Glossary

This glossary defines all technical terms, abbreviations, and concepts used throughout the MPL documentation.

## Core Concepts

### MPL (Meaning Protocol Layer)
A semantic overlay protocol that runs alongside MCP and A2A sessions to make the meaning of every exchange explicit, verifiable, and portable. MPL adds semantic types, capability negotiation, quality metrics, and provenance without replacing underlying transports.

### MCP (Model Context Protocol)
A client-server protocol for connecting LLM runtimes to tool hosts and resource providers. MCP handles transport, discovery, and invocation but does not enforce semantic contracts or quality guarantees.

### A2A (Agent-to-Agent)
A peer-to-peer protocol framework for autonomous agent communication. A2A provides routing and capability advertisement but leaves schema governance and quality enforcement to application layers.

## Semantic Types

### SType (Semantic Type)
A globally unique, versioned identifier that declares the intent and schema of a payload. Format: `namespace.domain.Intent.vMajor` (e.g., `org.calendar.Event.v1`). Each SType links to a canonical JSON Schema or Protobuf definition in the MPL registry.

### SType Registry
A centralized, version-controlled repository of SType definitions, schemas, tool descriptors, QoM profiles, and policy manifests. The registry enforces namespace governance, semver rules, and deprecation workflows.

### Semantic URN
The canonical identifier format for STypes: `urn:stype:namespace.domain.Intent.vMajor` (e.g., `urn:stype:org.calendar.Event.v1`).

### Namespace
The organizational scope of an SType (e.g., `org`, `com.acme`). Namespaces are governed by CODEOWNERS rules in the registry to prevent conflicts and ensure accountability.

## Handshake & Negotiation

### AI-ALPN (AI Application-Layer Protocol Negotiation)
A capability negotiation protocol modeled after TLS ALPN. Before exchanging work, peers negotiate protocols, models, STypes, tools, QoM profiles, and policies. Incompatibilities are logged as downgrade events.

### ClientHello
The initial handshake message sent by a client proposing supported protocols, models, STypes, tools, policies, QoM profiles, and optional feature flags.

### ServerSelect
The handshake response from a server selecting the capabilities it can honor and explaining any downgrades (e.g., unavailable tools, unsupported profiles).

### Downgrade
When a server cannot satisfy all client-proposed capabilities, it selects a compatible subset and logs the reasons (e.g., "tool unavailable", "profile too strict"). Downgrade telemetry helps teams track capability drift.

### Feature Flag
An optional capability extension namespaced to avoid SType explosion (e.g., `ext.qom.determinism@v1`, `recurrence`, `attendee_roles`). Features are negotiated during handshake.

## Quality of Meaning (QoM)

### QoM (Quality of Meaning)
A framework for measuring and enforcing semantic quality through observable metrics, negotiated profiles, and actionable breach detection.

### QoM Profile
A named configuration defining metric thresholds, sampling rates, and retry policies (e.g., `qom-strict-argcheck`, `qom-basic`). Profiles are negotiated during handshake and enforced per message.

### QoM Metrics

#### Schema Fidelity (SF)
**Mandatory metric.** Measures whether a payload conforms to its declared SType schema. Computed via JSON Schema or Protobuf validation. Target: 1.0 (100% conformance).

#### Instruction Compliance (IC)
Measures adherence to explicit assertions or constraints embedded in the request. Computed by executing declarative (JSONLogic, CEL) or imperative (JS/Python) assertion scripts. Reported as pass/fail ratio.

#### Groundedness (G)
Measures whether claims in a response are supported by cited sources. Computed by extracting verifiable claims (numbers, dates, entities) and checking them against provenance artifacts. Reported as a ratio (supported claims / total claims).

#### Determinism under Jitter (DJ)
Measures output stability when inputs are perturbed (temperature changes, context shuffling). Computed by running K re-executions and measuring semantic similarity (BLEU, ROUGE, embedding cosine). Reported as average similarity score.

#### Ontology Adherence (OA)
Measures conformance to domain-specific rules (chronological order, enum membership, cardinality constraints). Validated using SHACL, OWL, or custom rule engines.

#### Tool Outcome Correctness (TOC)
Measures whether tool side effects match expectations. Validated via post-check hooks (e.g., read-after-write, external API verification). Reported as pass/fail boolean.

### QoM Report
A structured payload attached to responses summarizing metric scores, profile evaluation status (`meets_profile`), and references to detailed artifacts (claim sets, diffs, logs).

### QoM Breach
A typed error (`E-QOM-BREACH`) emitted when a response fails to meet negotiated thresholds. Includes metric values, threshold mismatches, and remediation hints.

### QoM Evaluation Engine
A pluggable service (embedded SDK, sidecar, or central control plane) that validates payloads against negotiated profiles and produces QoM reports or breach errors.

### Sampling Rate
The probability (0.0–1.0) that an expensive QoM metric (Groundedness, Determinism) is computed for a given message. Used to manage cost in production.

## Semantic Integrity

### Semantic Hash (sem_hash)
A BLAKE3 cryptographic hash computed over a canonicalized payload. Used to detect meaning drift across retries, hops, or storage/replay scenarios.

### Canonicalization
The process of deterministically normalizing a payload (sorted keys, consistent encoding) before hashing or validation. Ensures semantic equivalence checks are reliable.

### Provenance
Metadata tracking the origin, intent, and transformation chain of a payload. Includes `intent` (SType reference), `inputs_ref` (context pointers), `consent_ref` (policy receipts), and optional signatures.

### Semantic Signature
An optional cryptographic signature over a semantic hash, binding agent identity to a payload for tamper-evidence and audit trails.

## Envelopes & Messages

### MPL Envelope
The semantic wrapper around payloads transmitted over MCP/A2A. Core fields: `id`, `stype`, `payload`, `args_stype`, `profile`, `sem_hash`, `provenance`, optional `qom_report`.

### Typed Call
A tool or agent invocation carrying an MPL envelope. The `stype` and `args_stype` fields declare input/output semantics; the envelope includes provenance and QoM context.

### Typed Error
A structured error response with a semantic error code (e.g., `E-QOM-BREACH`, `E-SCHEMA-FIDELITY`, `E-POLICY-DENIED`, `E-UNKNOWN-STYPE`). Errors include hints for remediation and relevant metric values.

## Policy & Governance

### Policy Manifest
A declarative definition (Rego, JSONLogic, or custom DSL) specifying consent requirements, redaction rules, or access controls. Stored in the registry under `/policies/{name}/v{MAJOR}/`.

### Policy Profile
A named policy configuration negotiated during handshake (e.g., `policy.ref#consent-basic-v1`). Enforced by the MPL runtime before dispatching calls or after receiving results.

### Consent Reference (consent_ref)
A pointer to a stored consent receipt indicating user/subject authorization for data use. Included in provenance when sharing user-linked data.

### Redaction Plan
A template specifying which fields to mask or remove from payloads before logging or forwarding to satisfy policy requirements.

### Policy Violation
A typed error (`E-POLICY-DENIED`) emitted when a message breaches negotiated policy. Includes remediation hints (e.g., missing consent scope, redaction required).

## Tools & Metadata

### Tool Descriptor
A manifest defining a tool's identity, input/output STypes, supported features, QoM expectations, policy requirements, and implementation bindings. Stored in the registry as `tool.{name}.v{major}.json`.

### args_stype
The SType identifier for a tool's input payload schema.

### returns_stype
The SType identifier for a tool's output payload schema.

### Tool Binding
The implementation reference for a tool (e.g., `impl.url`, `impl.type`). Allows multiple implementations (HTTP, gRPC, local function) to share a single semantic descriptor.

### Adapter / Mapper
A transformation script (often JSONLogic or Jsonnet) that bridges version skew between SType versions or tool APIs. Stored in registry under `/adapters/{from}->{to}/map.jsonnet`.

## Deployment Models

### Sidecar Proxy
A deployment pattern where MPL logic (handshake, envelope augmentation, QoM validation) runs in a separate process alongside the MCP/A2A client or server. Enables zero-code integration for legacy systems.

### MPL SDK
A client library embedding handshake negotiation, schema validation, QoM enforcement, and provenance logging. Used for native integration in orchestrators or agent runtimes.

### Native Integration
When an LLM runtime or MCP/A2A framework natively understands MPL envelopes and handshakes without proxies or SDK wrappers.

## Registry Operations

### Deprecation
Marking an SType or tool version as obsolete with a sunset date and upgrade path. Deprecated items emit warnings during handshake to encourage migration.

### Semver (Semantic Versioning)
Versioning scheme for STypes and tools: major.minor.patch. Only major version appears in wire identifiers; minor/patch changes are backward-compatible and tracked in schema metadata.

### CODEOWNERS
A governance mechanism requiring designated reviewers to approve changes to registry namespaces, preventing unauthorized schema modifications.

### Conformance Suite
A test harness with positive/negative schema vectors, jitter samples, and equivalence fuzzing. Used to validate SType implementations before registry publication.

## Error Codes

### E-QOM-BREACH
QoM metric(s) failed to meet negotiated thresholds. Includes metric values and hints.

### E-SCHEMA-FIDELITY
Payload failed JSON Schema or Protobuf validation. Includes validation error paths.

### E-TOOL-ARG-COERCION
Tool arguments could not be coerced to the declared `args_stype`. Indicates schema mismatch.

### E-POLICY-DENIED
Request violated negotiated policy (missing consent, redaction required). Includes remediation hints.

### E-UNKNOWN-STYPE
Referenced SType not found in registry or unsupported by peer. Suggests registration or adapter negotiation.

### E-UNKNOWN-TOOL
Referenced tool not available. Includes list of supported alternatives.

### E-NEGOTIATION-INCOMPATIBLE
Handshake failed because client and server have no compatible capability set (protocol version, SType major version mismatch).

## Observability & Telemetry

### Downgrade Event
A logged occurrence when handshake negotiation reduces capabilities (e.g., tool unavailable, profile too strict). Used to track capability drift.

### Downgrade Rate
Metric tracking percentage of sessions experiencing downgrades. Target: <5%.

### Unknown SType Rate
Metric tracking percentage of messages referencing unregistered STypes. Target: <0.1%.

### Time-to-First-Typed-Call (TTFTC)
Developer experience metric measuring elapsed time from MPL setup to successfully sending a typed message. Target: <30 minutes.

### QoM Pass Rate
Metric tracking percentage of messages meeting their negotiated QoM profile. Target: ≥95%.

## Abbreviations

- **ALPN**: Application-Layer Protocol Negotiation (TLS extension MPL adapts)
- **CAS**: Content-Addressable Storage (for storing QoM artifacts)
- **CEL**: Common Expression Language (for declarative assertions)
- **DJ**: Determinism under Jitter (QoM metric)
- **G**: Groundedness (QoM metric)
- **IC**: Instruction Compliance (QoM metric)
- **JSON Schema**: Schema definition language for JSON payloads
- **OA**: Ontology Adherence (QoM metric)
- **OWL**: Web Ontology Language (for ontology definitions)
- **Protobuf**: Protocol Buffers (binary schema/serialization format)
- **QoM**: Quality of Meaning
- **RDF**: Resource Description Framework (semantic web standard)
- **Rego**: Policy language used by Open Policy Agent
- **SF**: Schema Fidelity (QoM metric)
- **SHACL**: Shapes Constraint Language (for RDF validation)
- **SLO**: Service Level Objective (applies to QoM thresholds)
- **SType**: Semantic Type
- **TOC**: Tool Outcome Correctness (QoM metric)
- **URN**: Uniform Resource Name (identifier scheme)

## Related Terms

### Transport
The underlying delivery mechanism (HTTP, WebSocket, gRPC, MQTT). MPL runs atop transports and focuses on semantic contracts, not delivery guarantees.

### Overlay
An architectural pattern where MPL adds semantic capabilities without replacing existing MCP/A2A transports or coordination logic.

### ICP (Ideal Customer Profile)
Go-to-market term for target customer segments (regulated enterprises, platform builders, agent ops teams).

### PoC (Proof of Concept)
A time-limited pilot deployment used to validate MPL value propositions before full adoption.

### DX (Developer Experience)
Qualitative and quantitative measures of how easy MPL is to adopt (TTFTC, NPS, lines of code).

### NPS (Net Promoter Score)
Survey metric measuring developer satisfaction. Target: ≥+30 for SDK alpha.

---

For detailed technical specifications, see:
- `docs/protocol-architecture.md` - Core architecture
- `docs/implementation-guide.md` - Implementation checklist
- `docs/qom-evaluation-engine.md` - QoM metrics and enforcement
- `docs/mpl-with-mcp.md` - MCP integration guide
- `docs/mpl-with-a2a.md` - A2A integration guide
