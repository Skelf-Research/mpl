# MPL vs. Schema Registries: What's the Difference?

This document addresses the inevitable question: **"Isn't MPL just another schema registry? Why not use Confluent Schema Registry, AWS Glue, or Buf?"**

**TL;DR:** Schema registries solve **data contract validation**. MPL solves **semantic meaning + quality + compliance for AI agents**. They're complementary, not competitive.

---

## The Question

**Skeptic:** "We already have schema registries (Confluent, AWS Glue, Buf). Why do we need MPL? Can't we just use Avro/Protobuf schemas?"

**Answer:** Schema registries validate **structure**. MPL validates **structure + behavior + quality + provenance + compliance**. For traditional APIs, schema registries are sufficient. For AI agents making autonomous decisions, you need MPL.

---

## Schema Registries: What They Solve

### Purpose
Schema registries ensure data contracts are enforced between producers and consumers in data pipelines and microservices.

### Examples
- **Confluent Schema Registry** (Kafka + Avro/Protobuf/JSON Schema)
- **AWS Glue Data Catalog** (ETL, data lakes)
- **Azure Schema Registry** (Event Hubs, Avro/JSON Schema)
- **Buf Schema Registry** (Protobuf, gRPC)
- **OpenAPI/Swagger** (REST APIs)

### What They Provide
| Capability | Schema Registry | Purpose |
|------------|----------------|---------|
| **Schema validation** | ✅ Yes | Ensure payload matches structure |
| **Versioning** | ✅ Yes | Track schema evolution |
| **Compatibility checks** | ✅ Yes | Prevent breaking changes |
| **Discovery** | ✅ Yes | Find available schemas |
| **Code generation** | ✅ Yes | Generate types from schemas |

### What They DON'T Provide
| Capability | Schema Registry | Why Not |
|------------|----------------|---------|
| **Behavioral constraints** | ❌ No | "Amount < $10k" is not a schema concern |
| **Quality metrics** | ❌ No | No concept of "groundedness" or "determinism" |
| **Provenance** | ❌ No | No chain of custody or audit trail |
| **Compliance artifacts** | ❌ No | No QoM reports or control evidence |
| **AI-specific validation** | ❌ No | No handling of citations, claims, jitter |
| **Capability negotiation** | ❌ No | No handshake for models, tools, policies |

---

## MPL: What It Adds

### Purpose
MPL ensures AI agents produce **correct, compliant, auditable outputs** through semantic contracts, quality metrics, and provenance tracking.

### What MPL Provides Beyond Schema Registries

| Capability | Schema Registry | MPL | Why It Matters |
|------------|----------------|-----|----------------|
| **Schema validation** | ✅ Structure only | ✅ Structure + Semantic Type | AI outputs need intent, not just shape |
| **Business rules** | ❌ No | ✅ Assertions (IC) | "Amount < $10k" is a control, not a schema |
| **Quality metrics** | ❌ No | ✅ QoM (SF, IC, G, DJ, OA, TOC) | Need to prove AI outputs are "good enough" |
| **Provenance** | ❌ No | ✅ Full lineage + semantic hashes | Compliance requires "who, what, when, why" |
| **Audit artifacts** | ❌ No | ✅ QoM reports, typed errors | Regulators want evidence, not just schemas |
| **Capability negotiation** | ❌ No | ✅ AI-ALPN (models, tools, policies) | Agents need to negotiate what they can do |
| **Adversarial defenses** | ❌ No | ✅ Input/output validation, pattern detection | AI agents face prompt injection, jailbreaking |
| **Policy enforcement** | ❌ No | ✅ Consent, redaction, regional restrictions | AI needs governance beyond data validation |

---

## Concrete Example: Why Schemas Alone Aren't Enough

### Scenario: Trading Agent

**Schema Registry approach (Avro/Protobuf):**

```protobuf
// trade.proto
message TradeOrder {
  string ticker = 1;
  double amount = 2;
  string order_type = 3;
  int64 timestamp = 4;
}
```

**What this validates:**
- ✅ `ticker` is a string
- ✅ `amount` is a number
- ✅ `order_type` is a string
- ✅ `timestamp` is an integer

**What this DOESN'T validate:**
- ❌ Is `amount < $10k`? (business rule)
- ❌ Is `ticker` a valid symbol? (domain constraint)
- ❌ Is this during market hours? (temporal constraint)
- ❌ Does user have permission? (authorization)
- ❌ Who approved this trade? (provenance)
- ❌ Was this validated for compliance? (audit trail)

**Compliance team's reaction:** ❌ "This doesn't prove the agent followed our rules."

---

**MPL approach:**

```json
// SType: org.trading.TradeOrder.v1
{
  "schema": {
    "type": "object",
    "properties": {
      "ticker": {"type": "string", "pattern": "^[A-Z]{1,5}$"},
      "amount": {"type": "number", "minimum": 0.01},
      "order_type": {"enum": ["market", "limit"]},
      "timestamp": {"type": "integer"}
    },
    "required": ["ticker", "amount", "order_type", "timestamp"],
    "additionalProperties": false
  },
  "assertions": [
    {
      "id": "sox-amount-limit",
      "description": "SOX §404: Single trades <$10k without dual approval",
      "rule": {"<": [{"var": "amount"}, 10000]}
    },
    {
      "id": "market-hours-check",
      "description": "Trading only during market hours (9:30-16:00 ET)",
      "rule": {
        "and": [
          {">=": [{"hour": [{"var": "timestamp"}]}, 9.5]},
          {"<=": [{"hour": [{"var": "timestamp"}]}, 16]}
        ]
      }
    },
    {
      "id": "valid-ticker",
      "description": "Ticker must be in approved list",
      "rule": {"in": [{"var": "ticker"}, {"context.get": ["approved_tickers"]}]}
    }
  ]
}
```

**What MPL validates:**
- ✅ Schema structure (like schema registry)
- ✅ Amount < $10k (business rule)
- ✅ Valid ticker symbol (domain constraint)
- ✅ Market hours (temporal constraint)
- ✅ Provenance (who, what, when)
- ✅ QoM report (evidence for compliance)

**Compliance team's reaction:** ✅ "This proves the agent followed our rules. Approved."

---

## When Schema Registries Are Sufficient

**Use a schema registry (NOT MPL) when:**

✅ Traditional microservices (REST/gRPC APIs)
✅ Data pipelines (Kafka, ETL)
✅ Human-authored payloads (developers writing JSON)
✅ No compliance/audit requirements
✅ No behavioral constraints beyond structure
✅ No adversarial threats

**Examples:**
- User service → Order service (e-commerce)
- Kafka stream processing (clickstream analytics)
- S3 → Glue → Redshift (data warehouse)
- gRPC microservices (standard CRUD)

**Why schema registry is enough:** Humans control the system; structure validation = sufficient trust.

---

## When You Need MPL

**Use MPL (in addition to or instead of schema registry) when:**

✅ AI agents making autonomous decisions
✅ Regulated industries (finance, healthcare, EU AI Act)
✅ Compliance/audit requirements (SOX, GDPR, HIPAA)
✅ Adversarial environments (prompt injection, jailbreaking)
✅ Need to prove quality (groundedness, determinism)
✅ Multi-hop agent workflows (provenance critical)
✅ Business rules beyond structure ("amount < $10k")

**Examples:**
- Trading agent (autonomous stock purchases)
- Diagnosis assistant (medical recommendations)
- Loan approval agent (GDPR Article 22 decisions)
- Customer service agent (adversarial prompt injection)
- Multi-agent orchestration (provenance across hops)

**Why MPL is needed:** AI is non-deterministic; structure validation ≠ sufficient trust. Need quality + provenance + compliance.

---

## MPL + Schema Registry: Better Together

**MPL is NOT a replacement for schema registries. It's complementary.**

### Integration Pattern 1: MPL on Top of Existing Schemas

```yaml
# Use your existing Protobuf/Avro schemas
# Add MPL envelope + assertions

SType: org.trading.TradeOrder.v1
  schema_format: protobuf
  schema_ref: buf.build/acme/trading/TradeOrder

  # MPL adds behavioral constraints
  assertions:
    - id: sox-amount-limit
      rule: {"<": [{"var": "amount"}, 10000]}

  # MPL adds quality enforcement
  qom_profile: qom-sox-trading

  # MPL adds provenance
  provenance:
    required: true
    fields: [approved_by, risk_score]
```

**Benefit:** Keep existing schema registry; add MPL for AI-specific concerns.

---

### Integration Pattern 2: Schema Registry for Wire Format, MPL for Semantics

```
┌──────────────────────────────────────────────┐
│ Confluent Schema Registry                    │
│ - Avro schema for TradeOrder                 │
│ - Ensures binary compatibility               │
│ - Used by Kafka producers/consumers          │
└──────────────┬───────────────────────────────┘
               │
               ├─> Schema validated by Confluent
               │
               v
┌──────────────────────────────────────────────┐
│ MPL Layer (Proxy or Native)                  │
│ - Semantic Type: org.trading.TradeOrder.v1   │
│ - Assertions: amount limits, market hours    │
│ - QoM evaluation: SF=1.0, IC=1.0             │
│ - Provenance: approved_by, timestamp         │
└──────────────────────────────────────────────┘
```

**Division of labor:**
- **Schema Registry:** Wire format compatibility, code generation
- **MPL:** Semantic validation, quality metrics, compliance artifacts

**Example (Kafka + MPL):**

```python
from confluent_kafka.schema_registry import SchemaRegistryClient
from mpl.sdk import Session

# Use Confluent for schema enforcement
schema_client = SchemaRegistryClient({'url': 'http://schema-registry:8081'})

# Use MPL for semantic validation + QoM
mpl_session = Session.connect(
    stypes=['org.trading.TradeOrder.v1'],
    profile='qom-sox-trading'
)

# Produce to Kafka with both validations
def produce_trade(trade):
    # Confluent validates Avro schema
    schema_client.validate(trade, schema='TradeOrder')

    # MPL validates semantics + QoM
    mpl_session.call(
        tool='trading.execute.v1',
        payload=trade
    )
```

**Benefit:** Get best of both worlds—schema compatibility + semantic compliance.

---

### Integration Pattern 3: Import Schemas from Registry into MPL

```bash
# Convert Protobuf schema to MPL SType
$ mpl-registry import \
    --source buf.build/acme/trading/TradeOrder \
    --target org.trading.TradeOrder.v1 \
    --add-assertions sox-amount-limit,market-hours-check

# Output: SType with Protobuf schema + MPL assertions
```

**Benefit:** Leverage existing schema investment; add MPL incrementally.

---

## Objection Handling

### Objection 1: "Why not just extend Confluent Schema Registry with assertions?"

**Answer:**

Schema registries are **passive**—they store schemas and validate on demand. MPL is **active**—it negotiates capabilities, enforces policies, generates audit artifacts, and computes quality metrics.

Extending Confluent would require:
- Adding AI-ALPN handshake (out of scope for schema registry)
- QoM evaluation engine (not a schema concern)
- Provenance tracking (not a schema concern)
- Policy enforcement (not a schema concern)
- Typed errors for AI failures (not a schema concern)

**Result:** You'd rebuild MPL inside Confluent. Better to keep them separate and composable.

---

### Objection 2: "Can't we just add assertions to JSON Schema?"

**Answer:**

You can add **validation keywords** to JSON Schema (e.g., `minimum`, `maximum`, `pattern`). But you can't:

❌ Reference application state (e.g., "current position + new order < limit")
❌ Temporal constraints (e.g., "market hours only")
❌ Cross-field logic (e.g., "if order_type=market, then limit_price must be null")
❌ Generate QoM reports (proof that assertions passed)
❌ Track provenance (who approved, when, why)
❌ Enforce policies (consent, redaction, regional restrictions)

**MPL's Instruction Compliance** is specifically designed for **business logic validation** that schemas can't express.

---

### Objection 3: "This adds operational complexity. Why not keep it simple?"

**Answer:**

For traditional systems, **keep it simple—use schema registries.**

For AI agents in regulated industries, **"simple" = unauditable = blocked by compliance.**

The complexity MPL adds is **necessary** for:
- Proving to regulators that agents follow rules (SOX, GDPR, HIPAA)
- Defending against adversarial attacks (prompt injection, jailbreaking)
- Quantifying AI operational risk (VaR, assertion pass rates)
- Enabling post-incident forensics (semantic hash replay)

**Without MPL:** 18 weeks to compliance approval, manual audit archaeology, "hope the logs exist"
**With MPL:** 5 weeks to approval, automated audit trails, verifiable controls

**ROI:** $110k saved per agent, 13 weeks saved. Complexity is worth it.

---

### Objection 4: "What if schema registry vendors add these features?"

**Answer:**

**Great!** If Confluent/AWS/Azure add:
- Semantic types
- Assertion enforcement
- QoM metrics
- Provenance tracking
- AI-ALPN handshake

...then they've implemented MPL. We'd integrate with them.

**More likely:** They'll partner/integrate rather than rebuild. Schema registries focus on **data engineering**; MPL focuses on **AI compliance**. Different domains, complementary solutions.

---

## Competitive Positioning Summary

| Concern | Schema Registry | MPL | Integration |
|---------|----------------|-----|-------------|
| **Wire format compatibility** | ✅ Avro, Protobuf | ⚠️ JSON Schema (MVP) | Use schema registry for wire format |
| **Code generation** | ✅ Strong | ⚠️ Roadmap (Phase 2) | Use schema registry for codegen |
| **Behavioral constraints** | ❌ No | ✅ Assertions | MPL adds this layer |
| **Quality metrics** | ❌ No | ✅ QoM | MPL adds this layer |
| **Compliance artifacts** | ❌ No | ✅ QoM reports, provenance | MPL adds this layer |
| **AI-specific validation** | ❌ No | ✅ Groundedness, Determinism | MPL adds this layer |
| **Adversarial defenses** | ❌ No | ✅ Input/output validation | MPL adds this layer |

**Recommendation:** Use **both**
- Schema registry for data engineering (Kafka, ETL, microservices)
- MPL for AI compliance (agents, regulated industries)

---

## Market Positioning

### Schema Registries
**Target:** Data engineers, backend developers, platform teams
**Use cases:** Kafka streams, microservices, data pipelines
**Value prop:** Prevent breaking changes, ensure compatibility
**Buyer:** VP Engineering, Platform Engineering

### MPL
**Target:** AI teams, Compliance, Risk, AI Safety
**Use cases:** AI agents in regulated industries
**Value prop:** Prove agents follow rules, enable compliance approval
**Buyer:** VP AI, CTO, Chief Risk Officer, Head of Compliance

**Overlap:** Minimal. Schema registries = data engineering; MPL = AI governance.

---

## FAQ

**Q: Does MPL require a schema registry?**
A: No. MPL includes its own registry (GitHub-based in MVP). But you can integrate with existing schema registries.

**Q: Can MPL use Protobuf/Avro instead of JSON Schema?**
A: Phase 2 roadmap. MVP uses JSON Schema for simplicity. Protobuf support planned.

**Q: Can I import schemas from Confluent into MPL?**
A: Yes, via `mpl-registry import` CLI tool (roadmap).

**Q: Does MPL replace OpenAPI/Swagger?**
A: No. OpenAPI describes REST APIs; MPL describes AI agent semantics. They're complementary. You could generate MPL STypes from OpenAPI specs.

**Q: What about GraphQL schemas?**
A: Similar to OpenAPI—complementary. GraphQL describes query structure; MPL adds semantics, quality, provenance.

**Q: Is MPL just "schema validation on steroids"?**
A: No. It's **semantic contracts + quality metrics + provenance + compliance**. Schema validation is one component (Schema Fidelity), but QoM, provenance, and policy enforcement are equally important.

---

## Messaging for Sales/BD

### When Prospect Says: "We already use Confluent Schema Registry"

**Response:**

"Great! Confluent is perfect for data engineering—Kafka streams, microservices, ETL.

MPL solves a different problem: **AI agent compliance**.

Confluent validates that your trade order has the right fields. MPL validates that the trade is <$10k, during market hours, approved by a senior manager, and logged for SOX audit.

Think of it as **Confluent for data contracts, MPL for AI governance**. They work together."

**Follow-up:**

"Here's a quick question: When your compliance team asks, 'How do we know the AI agent followed our trading rules?'—does Confluent answer that?"

[Prospect: "No, Confluent just validates schema."]

"Exactly. That's what MPL provides. Want to see a 5-minute demo?"

---

### When Prospect Says: "Can't we just add assertions to our existing schemas?"

**Response:**

"You can add basic validation (e.g., 'amount > 0'). But can your schema system:
- Reference application state? ('current_position + new_trade < limit')
- Enforce temporal constraints? ('market hours only')
- Generate compliance reports? ('proof that SOX controls executed')
- Track provenance? ('who approved this, when, why')
- Defend against prompt injection? ('block unexpected fields in AI output')

If yes, you've built MPL. If no, that's what MPL provides."

---

### When Prospect Says: "This seems like duplicate infrastructure"

**Response:**

"I hear you—adding infrastructure needs clear ROI.

Here's the trade-off:

**Without MPL:**
- 18 weeks to compliance approval (your agents are blocked)
- $132k in engineering + compliance time per agent
- Manual audit archaeology when incidents happen

**With MPL:**
- 5 weeks to approval (13 weeks saved)
- $22k cost (83% savings)
- Automated audit trails for incidents

**Break-even:** If you deploy 2+ agents per year, MPL pays for itself.

Plus, MPL's sidecar proxy requires zero code changes—you can evaluate it in <30 minutes. Want to try it?"

---

## Conclusion

**Schema registries and MPL solve different problems:**

| Problem | Solution |
|---------|----------|
| **Data contract enforcement** (structure) | Schema Registry |
| **AI semantic contracts** (meaning + quality + compliance) | MPL |

**For traditional systems:** Schema registries are sufficient.
**For AI agents in regulated industries:** MPL is necessary.

**Best practice:** Use both
- Schema registry for wire format, compatibility, code generation
- MPL for semantic validation, quality metrics, compliance artifacts

**This is not competition—it's complementary tooling for different domains.**

---

**Related documents:**
- `docs/challenges.md` — Core problems MPL solves
- `docs/regulated-enterprise-value.md` — Compliance value proposition
- `docs/protocol-architecture.md` — Technical specification
- `docs/integration-modes.md` — How to deploy MPL
