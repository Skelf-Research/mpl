# MPL Policy Engine

The MPL policy engine enforces semantic authorization, consent management, data governance, and compliance requirements negotiated during the AI-ALPN handshake. While QoM focuses on output quality, the policy engine governs what data can be accessed, how it can be used, and what protections must apply.

## Implementation Status

**Policy Engine Lite is implemented** in `crates/mpl-core/src/policy.rs`. This provides:

| Feature | Status | Description |
|---------|--------|-------------|
| Rule-based enforcement | ✅ | SType pattern matching with wildcards |
| Access control | ✅ | Allow/deny lists per principal |
| QoM profile overrides | ✅ | Per-namespace/domain profile requirements |
| Rate limiting config | ✅ | Requests per window, per-principal |
| Custom constraints | ✅ | Metadata checks, payload size limits |
| Version constraints | ✅ | Eq, Gte, Lte, Range for SType versions |

**Full OPA-based Policy Engine** (consent management, redaction, regional compliance) is planned for Phase 3.

### Quick Start (Rust)

```rust
use mpl_core::policy::{PolicyEngine, Policy, PolicyContext, StypePattern, Operation};

// Create engine
let mut engine = PolicyEngine::new();

// Add policy requiring strict QoM for eval namespace
let policy = Policy::new("eval-strict")
    .with_stype_pattern(StypePattern::namespace("eval"))
    .with_qom_override("qom-strict-argcheck");

engine.add_policy(policy);

// Evaluate
let stype = SType::parse("eval.rag.RAGQuery.v1")?;
let context = PolicyContext::new(stype, Operation::Execute);
let decision = engine.evaluate(&context);

if decision.is_allowed() {
    println!("Required profile: {:?}", decision.required_profile);
}
```

### Configuration (YAML)

```yaml
# policy-config.yaml
default_profile: qom-basic
policies:
  - name: eval-strict
    stype_patterns:
      - namespace: eval
    qom_override:
      profile: qom-strict-argcheck

  - name: restricted-access
    stype_patterns:
      - namespace: org
        domain: user
    access_control:
      allow: [admin, service-account]
      default: deny
```

---

## 1. Goals & Responsibilities

- **Semantic authorization:** enforce capability-level access control beyond transport-layer auth.
- **Consent management:** validate user consent scope before processing personal data.
- **Data governance:** apply redaction, anonymization, or regional restrictions per policy.
- **Compliance automation:** codify regulatory requirements (GDPR, HIPAA, SOX) as machine-verifiable policies.
- **Auditability:** log all policy decisions (allow/deny) with contextual metadata for forensics.

## 2. Policy Manifest Structure

Policies are declarative definitions stored in the registry under `/policies/{name}/v{MAJOR}/`. Manifests specify rules, consent requirements, redaction templates, and enforcement hooks.

### 2.1 Example Policy Manifest (Rego)

```rego
# policies/consent-basic/v1/policy.rego
package mpl.policy.consent_basic.v1

import future.keywords.if
import future.keywords.in

# Default deny
default allow := false

# Allow if valid consent reference exists
allow if {
    input.provenance.consent_ref != null
    consent := get_consent(input.provenance.consent_ref)
    consent.status == "active"
    consent.expiry > time.now_ns()
    input.provenance.intent in consent.scopes
}

# Helper to fetch consent record
get_consent(ref) := consent if {
    # In production, query consent store via HTTP
    # For testing, use mock data
    consent := data.consents[ref]
}

# Redaction requirements
redact_fields := {"user.email", "user.phone"} if {
    input.provenance.policy_ref == "policy.ref#consent-basic-v1"
    input.provenance.consent_ref == null
}

# Violation details
violation[{"msg": msg}] if {
    not allow
    msg := sprintf("Missing or invalid consent_ref for intent %v", [input.provenance.intent])
}
```

### 2.2 Policy Manifest Metadata

```json
{
  "id": "policy.ref#consent-basic-v1",
  "name": "consent-basic",
  "version": "1.0.0",
  "description": "Basic consent enforcement for personal data workflows.",
  "language": "rego",
  "enforcement_points": ["pre-dispatch", "post-response"],
  "required_provenance": ["consent_ref", "intent"],
  "redaction_templates": [
    {"path": "$.user.email", "method": "mask"},
    {"path": "$.user.phone", "method": "mask"}
  ],
  "audit_level": "full"
}
```

## 3. Deployment Models

| Model | Description | When to use |
| ----- | ----------- | ----------- |
| **Embedded** | Policy engine runs inside MPL proxy/SDK. | Lightweight deployments; single-tenant. |
| **Sidecar** | Dedicated OPA (Open Policy Agent) sidecar alongside proxy. | Language heterogeneity; shared policies. |
| **Centralized** | Multi-tenant control plane serving policy decisions via API. | Enterprises with central governance/audit. |

All models expose the same API surface (gRPC/HTTP) so applications can switch without code changes.

## 4. Policy Evaluation Workflow

```
┌──────────────────┐
│ Incoming Request │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ Policy Engine    │
│ (Pre-Dispatch)   │
└────────┬─────────┘
         │
         ▼ [allow?]
    ┌────┴────┐
    │         │
   Yes       No
    │         │
    │         └──→ E-POLICY-DENIED (with hints)
    │
    ▼
┌──────────────────┐
│ Tool Execution   │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ Policy Engine    │
│ (Post-Response)  │
└────────┬─────────┘
         │
         ▼ [allow?]
    ┌────┴────┐
    │         │
   Yes       No
    │         │
    │         └──→ E-POLICY-DENIED (with hints)
    │
    ▼
┌──────────────────┐
│ Apply Redaction  │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ Return Response  │
└──────────────────┘
```

### 4.1 Pre-Dispatch Check
Executed before tool invocation:
- Validate `consent_ref` (if required).
- Check scope alignment (intent matches consent scopes).
- Enforce regional restrictions (data residency).
- Verify caller identity and authorization.

### 4.2 Post-Response Check
Executed after tool returns:
- Validate response data against policy constraints.
- Apply redaction templates (mask PII).
- Check for data leakage (unexpected fields).
- Enforce rate limits or quota tracking.

### 4.3 Audit Logging
Every policy decision is logged:
- **Timestamp:** ISO 8601 with nanosecond precision.
- **Policy ID:** `policy.ref#consent-basic-v1`.
- **Decision:** `allow` or `deny`.
- **Input:** provenance metadata, request context.
- **Output:** redacted fields, violation messages.
- **Actor:** agent/user identity.

## 5. Inputs & Outputs

### 5.1 Policy Evaluation Input

```json
{
  "envelope": {
    "id": "msg-42",
    "stype": "org.calendar.Event.v1",
    "payload": { ... },
    "provenance": {
      "intent": "calendar.create.v1",
      "consent_ref": "consent://user123/v2025-06-01",
      "policy_ref": "policy.ref#consent-basic-v1",
      "inputs_ref": ["ctx:plan.step#2"]
    }
  },
  "context": {
    "agent_id": "agent://planner",
    "timestamp": "2025-11-05T14:30:00Z",
    "session_id": "sess-xyz"
  }
}
```

### 5.2 Policy Evaluation Output (Allow)

```json
{
  "decision": "allow",
  "policy_id": "policy.ref#consent-basic-v1",
  "redaction": {
    "fields": [],
    "method": null
  },
  "audit_ref": "audit-log-123"
}
```

### 5.3 Policy Evaluation Output (Deny)

```json
{
  "decision": "deny",
  "policy_id": "policy.ref#consent-basic-v1",
  "violation": {
    "code": "E-POLICY-DENIED",
    "message": "Missing or invalid consent_ref for intent calendar.create.v1",
    "hints": [
      "Obtain user consent with scope 'calendar.create.v1'",
      "Include consent_ref in provenance metadata"
    ]
  },
  "audit_ref": "audit-log-124"
}
```

## 6. Consent Management

### 6.1 Consent Receipt Schema

```json
{
  "consent_id": "consent://user123/v2025-06-01",
  "subject": "user://user123",
  "scopes": ["calendar.create.v1", "calendar.read.v1"],
  "granted_at": "2025-06-01T10:00:00Z",
  "expiry": "2026-06-01T10:00:00Z",
  "status": "active",
  "purpose": "Task automation and scheduling",
  "revoker_url": "https://consent.example.com/revoke/user123"
}
```

### 6.2 Consent Store
- **Technology:** Redis (low-latency cache) + PostgreSQL (durable audit trail).
- **TTL:** consents expire automatically; policy engine checks `expiry` field.
- **Revocation:** webhook triggers cache invalidation; subsequent checks return `E-POLICY-DENIED`.

### 6.3 Consent Lifecycle
1. **Grant:** user approves via consent UI; receipt stored in consent store.
2. **Reference:** agent includes `consent_ref` in provenance for each request.
3. **Validation:** policy engine fetches receipt, validates `status == "active"` and `expiry > now`.
4. **Expiry:** TTL expires; policy engine denies requests until re-consent.
5. **Revocation:** user revokes; webhook invalidates cache; active sessions fail on next policy check.

## 7. Redaction & Data Minimization

### 7.1 Redaction Templates

```json
{
  "templates": [
    {
      "name": "mask-pii",
      "rules": [
        {"path": "$.user.email", "method": "mask", "pattern": "***@***.***"},
        {"path": "$.user.phone", "method": "mask", "pattern": "***-***-****"},
        {"path": "$.user.ssn", "method": "remove"}
      ]
    },
    {
      "name": "anonymize-location",
      "rules": [
        {"path": "$.location.gps", "method": "generalize", "precision": "city"}
      ]
    }
  ]
}
```

### 7.2 Redaction Methods
- **Mask:** replace with placeholder pattern (e.g., `***@***.***`).
- **Remove:** delete field entirely from payload.
- **Generalize:** reduce precision (GPS coordinates → city name).
- **Hash:** one-way hash for pseudonymization (e.g., SHA-256).
- **Tokenize:** replace with opaque token; lookup table for authorized services.

### 7.3 Enforcement Points
- **Pre-log:** redact before writing to observability systems.
- **Pre-forward:** redact before sending to third-party tools.
- **Pre-audit:** redact in audit exports for compliance reviews.

## 8. Regional & Compliance Policies

### 8.1 GDPR Policy Example

```rego
# policies/gdpr-eu/v1/policy.rego
package mpl.policy.gdpr_eu.v1

import future.keywords.if

default allow := false

allow if {
    # Data subject consent (Article 6.1.a)
    input.provenance.consent_ref != null
    consent := get_consent(input.provenance.consent_ref)
    consent.status == "active"

    # Purpose limitation (Article 5.1.b)
    input.provenance.intent in consent.scopes

    # Data minimization (Article 5.1.c)
    minimal_fields(input.envelope.payload)
}

minimal_fields(payload) if {
    # Check payload only includes necessary fields for intent
    required := data.intents[input.provenance.intent].required_fields
    actual := object.keys(payload)
    count(actual - required) == 0
}

# Right to be forgotten (Article 17)
redact_on_revocation if {
    consent := get_consent(input.provenance.consent_ref)
    consent.status == "revoked"
}
```

### 8.2 HIPAA Policy Example

```rego
# policies/hipaa-phi/v1/policy.rego
package mpl.policy.hipaa_phi.v1

import future.keywords.if

default allow := false

allow if {
    # BAA (Business Associate Agreement) check
    input.context.agent_id in data.authorized_agents

    # PHI redaction required for logging
    input.envelope.provenance.redaction_plan_id != null

    # Audit trail required (§164.312(b))
    input.context.audit_enabled == true
}

# Minimum necessary standard (§164.502(b))
minimal_phi(payload) if {
    phi_fields := {"ssn", "medical_record_number", "diagnosis"}
    actual_phi := {k | payload[k]; k in phi_fields}
    required_phi := data.intents[input.provenance.intent].required_phi
    count(actual_phi - required_phi) == 0
}
```

## 9. Developer Interfaces

### 9.1 SDK Integration (Python)

```python
from mpl.policy import PolicyEngine

engine = PolicyEngine(
    registry="https://registry.mpl.dev",
    policies=["policy.ref#consent-basic-v1"],
    consent_store="redis://localhost:6379"
)

# Pre-dispatch check
result = engine.evaluate(
    envelope=request_envelope,
    context={"agent_id": "agent://planner", "timestamp": now()}
)

if result.decision == "allow":
    response = tool.execute(request_envelope.payload)

    # Post-response check and redaction
    result = engine.evaluate_response(response, context)
    if result.decision == "allow":
        redacted = engine.apply_redaction(response, result.redaction)
        return redacted
    else:
        raise PolicyViolationError(result.violation)
else:
    raise PolicyViolationError(result.violation)
```

### 9.2 Sidecar Configuration (OPA)

```yaml
# opa-config.yaml
decision_logs:
  console: true
  service: audit-sink
bundles:
  mpl-policies:
    service: registry
    resource: /policies/bundle.tar.gz
    polling:
      min_delay_seconds: 60
      max_delay_seconds: 120
services:
  - name: registry
    url: https://registry.mpl.dev
  - name: audit-sink
    url: https://audit.example.com
```

### 9.3 CLI Testing

```bash
# Test policy locally
$ mpl-policy test \
    --policy policies/consent-basic/v1/policy.rego \
    --input test-cases/valid-consent.json \
    --expect allow

# Validate policy syntax
$ mpl-policy validate policies/consent-basic/v1/policy.rego

# Publish policy to registry
$ mpl-policy publish \
    --policy policies/consent-basic/v1/policy.rego \
    --version v1 \
    --registry https://registry.mpl.dev
```

## 10. Observability & Telemetry

### 10.1 Metrics
- **Policy evaluation rate:** decisions per second.
- **Deny rate:** percentage of requests denied (target <1% for legitimate traffic).
- **Latency:** p50/p99 for policy evaluation (target <10ms).
- **Consent cache hit rate:** efficiency of consent store (target >95%).
- **Redaction overhead:** latency added by redaction operations.

### 10.2 Audit Logs
Every policy decision is logged to an immutable audit store:

```json
{
  "timestamp": "2025-11-05T14:30:00.123456Z",
  "policy_id": "policy.ref#consent-basic-v1",
  "decision": "deny",
  "envelope_id": "msg-42",
  "agent_id": "agent://planner",
  "session_id": "sess-xyz",
  "violation": {
    "code": "E-POLICY-DENIED",
    "message": "Missing or invalid consent_ref"
  },
  "audit_ref": "audit-log-124"
}
```

### 10.3 Dashboards
- **Policy health:** evaluation latency, deny rate trends.
- **Compliance coverage:** percentage of requests with valid consent.
- **Redaction usage:** fields redacted per policy, method distribution.
- **Violation patterns:** common policy violations, remediation guidance.

## 11. Error Handling & Remediation

### 11.1 Typed Errors

#### E-POLICY-DENIED
```json
{
  "error": "E-POLICY-DENIED",
  "policy_id": "policy.ref#consent-basic-v1",
  "hint": "Missing or invalid consent_ref for intent calendar.create.v1",
  "remediation": [
    "Obtain user consent with scope 'calendar.create.v1'",
    "Include consent_ref in provenance metadata"
  ]
}
```

#### E-CONSENT-EXPIRED
```json
{
  "error": "E-CONSENT-EXPIRED",
  "consent_ref": "consent://user123/v2025-06-01",
  "expiry": "2026-06-01T10:00:00Z",
  "hint": "Consent expired; re-authorization required",
  "remediation": [
    "Prompt user to re-consent via consent UI",
    "Update consent_ref in provenance"
  ]
}
```

#### E-CONSENT-REVOKED
```json
{
  "error": "E-CONSENT-REVOKED",
  "consent_ref": "consent://user123/v2025-06-01",
  "revoked_at": "2025-10-15T09:00:00Z",
  "hint": "User revoked consent; workflow must terminate",
  "remediation": [
    "Delete cached data per right to be forgotten",
    "Notify user of workflow termination"
  ]
}
```

### 11.2 Remediation Workflows
- **Missing consent:** redirect to consent UI; retry with new `consent_ref`.
- **Expired consent:** prompt re-authorization; extend consent validity.
- **Revoked consent:** purge data per GDPR Article 17; terminate workflow.
- **Policy mismatch:** negotiate compatible policy during handshake.

## 12. Testing & Validation

### 12.1 Policy Test Suites
Each policy should include test cases:

```
policies/consent-basic/v1/
├── policy.rego
├── tests/
│   ├── allow_valid_consent.json
│   ├── deny_missing_consent.json
│   ├── deny_expired_consent.json
│   └── deny_revoked_consent.json
└── README.md
```

Test case format:
```json
{
  "description": "Allow request with valid consent",
  "input": {
    "envelope": { ... },
    "context": { ... }
  },
  "expected": {
    "decision": "allow"
  }
}
```

### 12.2 CI Integration
- **Automated testing:** run policy test suites on every PR.
- **Coverage analysis:** ensure all policy paths are exercised.
- **Performance benchmarks:** validate evaluation latency <10ms.
- **Security scans:** detect overly permissive rules or injection vulnerabilities.

### 12.3 Conformance Suite
Registry publishes canonical test cases for standard policies (consent-basic, gdpr-eu, hipaa-phi). Implementations must pass conformance tests before production use.

## 13. Future Enhancements

- **Machine learning policies:** detect anomalous access patterns using ML models.
- **Federated policy management:** delegate policy decisions across organizational boundaries.
- **Differential privacy:** integrate DP mechanisms for aggregate queries.
- **Blockchain audit logs:** immutable provenance on distributed ledgers.
- **Policy composition:** combine multiple policies with precedence rules.

---

For related documentation, see:
- `docs/security.md` - Privacy, consent, and data protection controls
- `docs/qom-evaluation-engine.md` - Quality enforcement (complementary to policy)
- `docs/registry-architecture.md` - Policy manifest storage and versioning
- `GLOSSARY.md` - Policy-related term definitions (consent_ref, redaction plan, etc.)
