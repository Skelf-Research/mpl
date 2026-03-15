# MPL Security Architecture

This document outlines the security model, threat considerations, and defensive mechanisms for MPL deployments. While MPL focuses on semantic contracts rather than transport security, it provides primitives for provenance, integrity, and policy enforcement that complement existing security layers.

**Critical addition:** See `docs/adversarial-robustness.md` for detailed analysis of how MPL defends against adversarial attacks on AI agents (prompt injection, jailbreaking, data exfiltration, etc.). This is essential reading for security teams evaluating MPL for production deployment.

## 1. Security Principles

- **Defense in depth:** MPL security works alongside TLS, authentication, and authorization layers provided by underlying transports (MCP/A2A/HTTP).
- **Semantic integrity:** cryptographic hashes and optional signatures ensure meaning is not corrupted across hops or retries.
- **Policy as code:** consent, redaction, and access controls are negotiated and enforced programmatically rather than via tribal knowledge.
- **Auditability:** provenance chains and QoM reports create tamper-evident logs for incident response and compliance reviews.
- **Least privilege:** tools and agents advertise minimal required capabilities; handshake negotiation prevents privilege escalation through capability drift.

## 2. Threat Model

### 2.1 Threat Actors

| Actor | Motivation | Capabilities |
| ----- | ---------- | ------------ |
| **Malicious client** | Extract unauthorized data, inject malicious payloads, cause denial-of-service. | Can send crafted handshake/payload; may compromise client credentials. |
| **Compromised tool/server** | Exfiltrate data, corrupt responses, violate policy constraints. | Can modify payloads, skip QoM checks, forge provenance metadata. |
| **Man-in-the-middle** | Intercept/modify semantic payloads, inject false SType definitions. | Can observe/tamper with traffic if TLS is compromised or absent. |
| **Registry attacker** | Publish malicious STypes, inject backdoors via schema changes. | Can submit PRs or exploit governance gaps. |
| **Insider threat** | Abuse legitimate access to exfiltrate sensitive data or disable policy enforcement. | Has valid credentials and deep system knowledge. |

### 2.2 Attack Vectors

#### Schema Poisoning
Attacker publishes malicious SType definitions with hidden side effects (e.g., `additionalProperties: true` allowing arbitrary fields, or schemas referencing external URLs for data exfiltration).

**Mitigations:**
- Registry governance with CODEOWNERS approval for all schema changes.
- Automated lint checks forbidding unbounded `additionalProperties`, external `$ref` URLs, and suspicious patterns.
- Schema immutability: published versions are frozen; changes require new major versions.
- Client-side schema caching with integrity checks (hash validation against known-good registry state).

#### Payload Injection
Attacker crafts payloads that pass schema validation but contain malicious content (XSS strings, SQL injection attempts, command injection).

**Mitigations:**
- Schema-level constraints (regex patterns, format validators, enum restrictions).
- QoM Instruction Compliance checks validate business logic constraints beyond schema structure.
- Ontology Adherence rules enforce domain-specific safety (e.g., chronological order, bounded ranges).
- Tool-level input sanitization remains the responsibility of tool implementers; MPL provides schema contracts as the first line of defense.

#### Semantic Drift / Downgrade Attacks
Attacker manipulates handshake to force capability downgrades (e.g., removing QoM requirements, disabling policy checks).

**Mitigations:**
- Handshake downgrades are logged with telemetry; anomalies trigger alerts.
- Clients can enforce minimum acceptable profiles (fail-closed if negotiation falls below thresholds).
- Mutual authentication ensures both parties are authorized before negotiation begins.
- Downgrade rate monitoring (target <5%); spikes indicate potential attack or misconfiguration.

#### Provenance Forgery
Attacker tampers with provenance metadata to hide malicious intent or evade audit trails.

**Mitigations:**
- Semantic hashes (BLAKE3) over canonical payloads detect tampering.
- Optional signatures bind agent identity to `sem_hash` values, creating non-repudiable audit trails.
- Provenance chains are append-only; modification breaks hash linkage (similar to Merkle trees).
- Registry-published agent keys allow signature verification.

#### QoM Bypass
Attacker disables or manipulates QoM checks to allow low-quality or malicious outputs.

**Mitigations:**
- QoM enforcement is mandatory in production profiles; cannot be disabled without explicit policy override (logged).
- QoM evaluation engine runs in isolated sandboxes when processing untrusted payloads.
- Centralized QoM control planes prevent local overrides by compromised clients.
- Audit logs track all QoM breaches and profile degradations.

#### Policy Evasion
Attacker bypasses consent checks, redaction requirements, or access controls.

**Mitigations:**
- Policy manifests are versioned and stored in registry; tampering requires registry compromise (defended by governance).
- Policy enforcement hooks run before tool dispatch and after response receipt; both client and server validate.
- `consent_ref` and `policy_ref` fields are mandatory for regulated workflows; missing references trigger `E-POLICY-DENIED`.
- Policy violations are logged to immutable audit stores with timestamps and actor identities.

#### Denial of Service (DoS)
Attacker floods system with expensive QoM checks (Determinism, Groundedness) or registry lookups.

**Mitigations:**
- Rate limiting on handshake negotiation and registry API endpoints.
- QoM sampling rates control cost of expensive metrics (e.g., DJ sampled at 20%).
- Schema/profile caching reduces registry load.
- Resource quotas per client/session prevent single actor from exhausting system capacity.
- Jitter checks and expensive validations can be offloaded to async queues with backpressure.

#### Supply Chain Attacks
Attacker compromises registry infrastructure, CI/CD pipelines, or SDK dependencies to inject malicious code.

**Mitigations:**
- Registry artifacts (schemas, profiles, policies) are signed by maintainers; clients verify signatures before use.
- Reproducible builds and SBOM (Software Bill of Materials) for SDK releases.
- Multi-party approval for registry changes (CODEOWNERS, multi-sig for critical namespaces).
- Continuous security scanning of registry and SDK dependencies (Dependabot, Snyk).

## 3. Authentication & Authorization

### 3.1 Transport-Layer Auth
MPL assumes underlying transports (MCP/A2A) handle authentication:
- **MCP over HTTP/WebSocket:** TLS + OAuth2 / API keys / mTLS.
- **A2A over gRPC:** mTLS, JWT tokens, or service mesh auth (Istio, Linkerd).
- **MQTT brokers:** TLS + username/password or client certificates.

MPL does not reimplement auth; it relies on transport guarantees.

### 3.2 Semantic-Layer Authorization
MPL adds policy-based authorization on top of transport auth:
- **Tool access control:** handshake negotiation ensures clients only access advertised tools.
- **SType permissions:** registry can enforce read/write permissions per namespace (e.g., only `org.acme` maintainers can publish `org.acme.*` STypes).
- **Policy profiles:** consent requirements and redaction rules act as semantic authorization checks.

### 3.3 Agent Identity
- Agents include identity metadata in provenance (`agent://planner`, `agent://executor`).
- Optional cryptographic signatures bind identity to payloads for non-repudiation.
- Registry publishes agent public keys for signature verification.

## 4. Cryptographic Primitives

### 4.1 Semantic Hashes
- **Algorithm:** BLAKE3 (chosen for speed and security).
- **Purpose:** detect payload tampering, enable deduplication, support replay protection.
- **Canonicalization:** deterministic key sorting, consistent encoding (UTF-8), whitespace normalization.
- **Usage:** every MPL envelope includes `sem_hash`; mismatches trigger alerts.

### 4.2 Signatures (Optional)
- **Algorithm:** Ed25519 (fast, compact, widely supported).
- **Signing:** `signature = sign(private_key, sem_hash)`.
- **Verification:** clients fetch agent public keys from registry; validate `signature` against `sem_hash`.
- **Use cases:** high-assurance workflows, regulated environments requiring non-repudiation.

### 4.3 Key Management
- **Registry keys:** stored in secure vaults (HSM, KMS); used to sign published schemas/profiles.
- **Agent keys:** managed by orchestrators or identity providers; rotated periodically.
- **Key distribution:** public keys published in registry under `/agents/{id}/pubkey.pem`.

## 5. Secure Registry Operations

### 5.1 Governance Controls
- **Namespace ownership:** CODEOWNERS enforces approval workflows.
- **Change auditing:** all registry commits are logged; Git history provides tamper-evident trail.
- **Multi-party approval:** critical namespaces (e.g., `core.*`) require 2+ maintainer approvals.
- **Automated validation:** CI runs lint, schema validation, and security scans on every PR.

### 5.2 Schema Integrity
- **Immutability:** published SType versions cannot be edited; changes require new major versions.
- **Content hashing:** registry serves schemas with `ETag` headers; clients cache and validate integrity.
- **Deprecation transparency:** sunset dates and upgrade paths are machine-readable.

### 5.3 Access Control
- **Public read:** schemas and tools are world-readable for discoverability.
- **Authenticated write:** only namespace maintainers can publish; enforced via OAuth/GitHub auth.
- **Rate limiting:** API endpoints throttle unauthenticated clients to prevent scraping attacks.

## 6. Privacy & Data Protection

### 6.1 Consent Management
- **Consent receipts:** stored with TTL; referenced via `consent_ref` in provenance.
- **Scope enforcement:** policies validate consent scope matches requested operations.
- **Revocation:** expired or revoked consent triggers `E-POLICY-DENIED`.

### 6.2 Redaction & PII Handling
- **Redaction plans:** declarative templates specify fields to mask (e.g., `$.user.email` → `***`).
- **Enforcement points:** applied before logging, before forwarding to third-party tools, and in audit exports.
- **PII detection:** optional hooks for automated PII scanning (regex, ML models).

### 6.3 Data Residency
- **Policy profiles:** can enforce regional restrictions (e.g., GDPR compliance requires `policy.ref#gdpr-eu-v1`).
- **Tool bindings:** SType definitions include `impl.region` metadata; orchestrators route accordingly.

## 7. Incident Response

### 7.1 Audit Trails
- **Provenance chains:** every message includes `inputs_ref` linking to prior steps; forms a DAG for reconstruction.
- **QoM reports:** stored with semantic hashes; used to identify when/where quality degraded.
- **Downgrade logs:** handshake negotiation failures and capability reductions are timestamped and indexed.
- **Policy violations:** all `E-POLICY-DENIED` events are logged with actor, timestamp, and remediation hints.

### 7.2 Forensics
- **Semantic hash replay:** reconstruct exact payloads using `sem_hash` references and CAS storage.
- **Signature verification:** validate non-repudiation claims during incident reviews.
- **QoM metric history:** analyze when/why quality breaches occurred; correlate with deployment changes.

### 7.3 Remediation
- **Schema rollback:** deprecate compromised STypes; publish patched versions with security notices.
- **Agent quarantine:** revoke compromised agent keys; reject signatures from revoked identities.
- **Policy updates:** tighten redaction/consent rules; force re-negotiation of active sessions.

## 8. Compliance & Regulatory Alignment

### 8.1 GDPR
- Consent receipts satisfy "lawful basis" requirements.
- Redaction plans enable "right to be forgotten."
- Provenance logs support "right to explanation" for automated decisions.

### 8.2 SOX / Financial Services
- Semantic hashes and signatures provide tamper-evidence for audit.
- QoM reports demonstrate control effectiveness (e.g., "all trades validated to SF=1.0").

### 8.3 HIPAA / Healthcare
- Policy profiles enforce BAA requirements and PHI redaction.
- Audit trails satisfy "accounting of disclosures."

### 8.4 AI Regulations (EU AI Act, etc.)
- QoM metrics (Schema Fidelity, Instruction Compliance) demonstrate "accuracy and robustness."
- Provenance chains enable "traceability of AI system decisions."
- Policy profiles codify "human oversight" requirements.

### 8.5 UK Financial Services (FCA/PRA)
- **SM&CR (Senior Managers & Certification Regime):** Provenance links agent decisions to accountable Senior Managers.
- **Consumer Duty:** Assertions enforce good outcomes and harm prevention requirements.
- **SYSC (Systems and Controls):** Controls-as-code satisfy effective control requirements.
- **Operational Resilience (PS21/3):** QoM monitoring enables impact tolerance tracking.

See `docs/regulated-enterprise-value.md` for detailed UK financial services compliance mappings.

## 9. Security Best Practices for Deployers

1. **Always use TLS:** MPL assumes encrypted transports; never deploy over plaintext channels.
2. **Enable signatures in production:** opt-in for regulated workflows; adds minimal overhead.
3. **Enforce minimum QoM profiles:** fail-closed when negotiation cannot meet thresholds.
4. **Monitor downgrade rates:** spikes indicate attacks or misconfiguration.
5. **Rotate agent keys regularly:** limit blast radius of key compromise.
6. **Audit registry PRs carefully:** schema changes are trust boundaries.
7. **Isolate QoM engines:** run in sandboxes when evaluating untrusted payloads.
8. **Log everything:** provenance, QoM, policy violations; retain for incident response.
9. **Test failure modes:** simulate schema poisoning, QoM bypass, policy evasion in staging.
10. **Participate in vulnerability disclosure:** report security issues via responsible disclosure channels.

## 10. Open Questions & Future Work

- **Formal verification:** explore theorem proving for QoM metric correctness.
- **Differential privacy:** integrate DP mechanisms for sensitive aggregations.
- **Federated learning:** support privacy-preserving model updates across MPL peers.
- **Secure multi-party computation:** enable collaborative workflows without revealing raw data.
- **Hardware security modules (HSMs):** integrate for registry key management and signature operations.

---

For operational security guidance, see:
- `docs/implementation-guide.md` - Deployment patterns and configurations
- `docs/qom-evaluation-engine.md` - Sandboxing and isolation for QoM checks
- `GLOSSARY.md` - Security-related term definitions (consent_ref, redaction plan, etc.)
