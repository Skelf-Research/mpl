# MPL Alignment with AI Safety & Risk Teams

This document addresses how MPL supports **AI Safety teams** (preventing harmful model behavior) and **Risk teams** (managing operational, reputational, and financial risk) - distinct from but complementary to Compliance teams.

---

## The Three Internal Stakeholders

| Team | Primary Concern | Traditional Tools | MPL Value |
|------|----------------|-------------------|-----------|
| **Compliance** | Regulatory adherence, audit trails | Manual documentation, periodic reviews | ✅ Automated audit trails, controls-as-code |
| **AI Safety** | Model alignment, harmful outputs, fairness | Eval harnesses, red teaming, model cards | ⚠️ **Partial** - MPL provides constraints, not alignment |
| **Risk** | Operational incidents, financial loss, reputation | Risk registers, incident logs, RCA reports | ✅ Real-time risk monitoring, quantified exposure |

**Key insight:** Compliance teams care about **"can we prove we followed the rules?"** AI Safety teams care about **"will the model do what we want?"** Risk teams care about **"what's our exposure and how do we reduce it?"**

MPL addresses Compliance and Risk directly. AI Safety requires additional tooling.

---

## AI Safety Team Concerns

### What AI Safety Teams Worry About

1. **Alignment failures** - Model doesn't follow intended behavior despite prompts
2. **Harmful outputs** - Bias, toxicity, misinformation, inappropriate content
3. **Capability overhang** - Model can do things we don't want it to do
4. **Goal misspecification** - Optimizing wrong objective (Goodhart's Law)
5. **Distributional shift** - Model fails on edge cases not in training data
6. **Jailbreaking** - Users elicit harmful behavior via prompt engineering
7. **Model poisoning** - Training data contamination or adversarial examples

### What MPL Provides (AI Safety Perspective)

| AI Safety Need | MPL Capability | Completeness |
|----------------|----------------|--------------|
| **Output constraints** | Schema Fidelity (output validation) | ✅ **Strong** - Blocks schema-violating outputs |
| **Behavioral bounds** | Instruction Compliance (assertions) | ⚠️ **Partial** - Enforces rules, not alignment |
| **Harm prevention** | Content pattern assertions | ⚠️ **Weak** - Regex-based, not semantic understanding |
| **Audit trail** | Provenance + QoM reports | ✅ **Strong** - Full lineage for post-incident analysis |
| **Red team testing** | QoM profiles (test harnesses) | ⚠️ **Partial** - Can encode test cases, not automated discovery |
| **Fairness/bias** | Custom assertions | ⚠️ **Weak** - Requires manual metric definition |
| **Alignment verification** | Determinism, Groundedness | ❌ **Not in MVP** - Phase 2+ feature |

**Summary:** MPL provides **guardrails and audit**, not **alignment**. AI Safety teams still need evals, red teaming, and alignment research. MPL makes their constraints enforceable.

---

## How AI Safety Teams Use MPL

### 1. Encoding Safety Constraints as Assertions

**Example: Preventing Harmful Medical Advice**

```json
// SType: org.healthcare.DiagnosisResponse.v1
{
  "properties": {
    "suggested_diagnosis": {"type": "string"},
    "confidence": {"type": "number", "minimum": 0, "maximum": 1},
    "disclaimer": {"type": "string"}
  },
  "required": ["suggested_diagnosis", "confidence", "disclaimer"],
  "assertions": [
    {
      "id": "safety-low-confidence-disclaimer",
      "description": "AI Safety: Low confidence requires explicit disclaimer",
      "rule": {
        "if": [
          {"<": [{"var": "confidence"}, 0.7]},
          {"match": [{"var": "disclaimer"}, "(?i)not medical advice"]},
          true
        ]
      }
    },
    {
      "id": "safety-no-prescription-language",
      "description": "AI Safety: Cannot recommend prescription drugs",
      "rule": {
        "not": {
          "match": [{"var": "suggested_diagnosis"}, "(?i)(prescribe|medication|drug)"]
        }
      }
    }
  ]
}
```

**AI Safety value:** Hard constraints prevent classes of harmful outputs. Not perfect (can't detect all harms), but reduces attack surface.

---

### 2. Output Validation as Safety Layer

**Example: Preventing Data Leakage in Customer Service**

```json
// Output SType: org.support.CustomerResponse.v1
{
  "properties": {
    "response": {"type": "string", "maxLength": 2000},
    "resolved": {"type": "boolean"}
  },
  "required": ["response", "resolved"],
  "additionalProperties": false,  // ← Blocks unexpected fields
  "assertions": [
    {
      "id": "safety-no-pii-patterns",
      "description": "AI Safety: Block common PII patterns",
      "rule": {
        "not": {
          "or": [
            {"match": [{"var": "response"}, "\\b\\d{3}-\\d{2}-\\d{4}\\b"]},  // SSN
            {"match": [{"var": "response"}, "\\b\\d{16}\\b"]},  // CC number
            {"match": [{"var": "response"}, "\\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\\.[A-Z|a-z]{2,}\\b"]}  // Email
          ]
        }
      }
    },
    {
      "id": "safety-no-competitor-mentions",
      "description": "AI Safety: Don't mention competitors",
      "rule": {
        "not": {
          "match": [{"var": "response"}, "(?i)(CompetitorA|CompetitorB|CompetitorC)"]
        }
      }
    }
  ]
}
```

**Result:** Even if model is jailbroken, response is blocked before delivery. Safety failure becomes observable incident, not user-facing harm.

---

### 3. Red Team Testing with QoM Profiles

**Example: Adversarial Eval Harness**

```json
// Profile: qom-safety-redteam
{
  "profile_id": "qom-safety-redteam",
  "description": "AI Safety red team evaluation profile",
  "metrics": {
    "schema_fidelity": {
      "threshold": 1.0,
      "sampling_rate": 1.0  // Test every input
    },
    "instruction_compliance": {
      "threshold": 1.0,
      "sampling_rate": 1.0,
      "assertions": [
        {"id": "safety-no-harmful-content"},
        {"id": "safety-no-pii-leakage"},
        {"id": "safety-refusal-on-dangerous-requests"}
      ]
    }
  },
  "failure_mode": "fail_fast",  // Stop on first violation
  "telemetry": {
    "log_all_failures": true,
    "alert_on_failure": true
  }
}
```

**AI Safety workflow:**

1. Create test dataset with adversarial prompts (jailbreaks, harmful requests, edge cases)
2. Run agent with `qom-safety-redteam` profile
3. Collect QoM reports showing which safety constraints passed/failed
4. Iterate on assertions or model prompting until test suite passes

**MPL value:** Standardized harness for safety testing; reproducible; version-controlled.

---

### 4. Post-Incident Analysis

**Scenario:** Customer reports agent provided harmful advice.

**Traditional approach:**
1. Search logs for user ID
2. Find unstructured log entries
3. Try to reconstruct what model saw
4. Unclear what safety checks (if any) were applied
5. **Outcome:** Incident report is speculative, fixes are ad-hoc

**With MPL:**

```json
// Incident ID: INC-2025-0342
// Retrieve from audit log via semantic hash
{
  "id": "msg-a3f8...",
  "stype": "org.support.CustomerResponse.v1",
  "payload": {
    "response": "...",  // Exact harmful response
    "resolved": false
  },
  "qom_report": {
    "profile": "qom-safety-basic",
    "metrics": {
      "schema_fidelity": 1.0,  // Passed schema
      "instruction_compliance": 0.85  // FAILED safety assertion
    },
    "assertions_failed": [
      {
        "id": "safety-no-harmful-content",
        "expected": true,
        "actual": false,
        "details": "Content matched harmful pattern: medical diagnosis"
      }
    ]
  },
  "provenance": {
    "inputs_ref": ["ctx:conversation#msg-7a2b"],
    "model": "gpt-4",
    "timestamp": "2025-11-05T14:23:15Z"
  },
  "sem_hash": "b3:..."
}
```

**AI Safety team can:**
1. Replay exact input/output using semantic hash
2. See which safety assertion failed
3. Identify root cause (assertion too loose, model jailbroken, edge case)
4. Update assertion library or prompting
5. Re-run against test suite to verify fix

**Outcome:** Data-driven incident response; systematic fixes; measurable improvement.

---

## Risk Team Concerns

### What Risk Teams Worry About

1. **Operational risk** - System failures, service outages, data loss
2. **Reputational risk** - Brand damage from harmful outputs or incidents
3. **Financial risk** - Regulatory fines, litigation, fraud losses
4. **Third-party risk** - Vendor failures, supply chain attacks
5. **Model risk** - Inaccurate predictions leading to bad decisions
6. **Concentration risk** - Over-reliance on single model/vendor
7. **Emerging risk** - Novel threats from AI capabilities

### What MPL Provides (Risk Management Perspective)

| Risk Management Need | MPL Capability | Value |
|----------------------|----------------|-------|
| **Risk identification** | Typed errors, QoM metrics | ✅ Granular failure taxonomy |
| **Risk measurement** | QoM scores, assertion pass rates | ✅ Quantified risk exposure |
| **Risk monitoring** | Real-time telemetry, alerts | ✅ Continuous risk dashboard |
| **Risk controls** | Assertions, schema validation | ✅ Automated control enforcement |
| **Control effectiveness** | QoM reports, audit logs | ✅ Evidence of control execution |
| **Incident detection** | E-QOM-BREACH, downgrade alerts | ✅ Automated incident triggers |
| **Root cause analysis** | Provenance chains, semantic hashes | ✅ Forensic reconstruction |
| **Risk reporting** | Aggregated QoM metrics | ✅ Board-level risk dashboards |

---

## How Risk Teams Use MPL

### 1. Real-Time Risk Monitoring Dashboard

**Risk team view (operational dashboard):**

```
┌─────────────────────────────────────────────────────────────┐
│ AI Agent Risk Dashboard (Last 24 Hours)                     │
├─────────────────────────────────────────────────────────────┤
│ CRITICAL RISKS                                              │
│ ⚠️  Trading Agent: 3 E-QOM-BREACH (amount limit)           │
│     Exposure: $47,000 (below VaR threshold)                 │
│     Action: Review trades, update assertion                 │
│                                                             │
│ ⚠️  Customer Service: Schema Fidelity 94% (target: 99%)    │
│     Volume: 23 failures / 1,500 requests                    │
│     Action: Investigate input validation                    │
├─────────────────────────────────────────────────────────────┤
│ RISK METRICS                                                │
│ Operational Risk Score: 2.3 / 10 (Low)                      │
│   - Schema Fidelity:           99.2% (↑ 0.1%)              │
│   - Instruction Compliance:    98.7% (→ stable)            │
│   - Assertion Pass Rate:       97.8% (↓ 0.3%)              │
│                                                             │
│ Reputational Risk Score: 1.8 / 10 (Low)                     │
│   - Safety Assertion Failures: 2 (threshold: 5)            │
│   - PII Leakage Attempts:      0 (threshold: 0)            │
│                                                             │
│ Financial Risk Score: 3.1 / 10 (Low-Medium)                 │
│   - Trading Limit Breaches:    3 (threshold: 5)            │
│   - Estimated Exposure:        $47K (VaR: $100K)           │
├─────────────────────────────────────────────────────────────┤
│ TREND ANALYSIS (7 Days)                                     │
│ [Chart showing QoM metrics over time]                       │
│ - Schema Fidelity:      ████████████ 99%                   │
│ - Instruction Compliance: ███████████ 98%                   │
│ - Downgrade Rate:       █ 2%                                │
└─────────────────────────────────────────────────────────────┘
```

**MPL powers this via:**
- Real-time QoM metric aggregation
- Typed error counts (E-SCHEMA-FIDELITY, E-QOM-BREACH)
- Assertion failure telemetry
- Downgrade event tracking

---

### 2. Model Risk Quantification

**Example: Measuring Financial Exposure from Model Errors**

```python
# Risk calculation using MPL telemetry
from mpl.risk import RiskAnalyzer

analyzer = RiskAnalyzer()

# Get QoM metrics for trading agent (last 30 days)
metrics = analyzer.get_metrics(
    stype="org.trading.TradeOrder.v1",
    period="30d"
)

# Calculate Value at Risk (VaR) from QoM breaches
var_analysis = {
    "total_trades": metrics.total_requests,
    "qom_breaches": metrics.qom_breaches,
    "breach_rate": metrics.qom_breaches / metrics.total_requests,

    # Financial impact
    "avg_trade_size": 5000,  # USD
    "max_loss_per_breach": 5000 * 0.05,  # 5% slippage assumption
    "var_95": metrics.qom_breaches * 250 * 0.95,  # 95th percentile

    # Risk concentration
    "top_assertion_failures": [
        {"id": "amount-limit-check", "failures": 15, "exposure": "$75K"},
        {"id": "market-hours-check", "failures": 8, "exposure": "$40K"},
        {"id": "position-limit-check", "failures": 3, "exposure": "$15K"}
    ],

    # Recommendation
    "action": "Tighten amount-limit-check assertion threshold from $10K to $8K"
}
```

**Risk team output:**

```
Model Risk Assessment: Trading Agent
─────────────────────────────────────
Period: Last 30 days
Total Trades: 12,547
QoM Breach Rate: 0.21% (26 breaches)

Value at Risk (VaR 95): $118,750
Within Risk Appetite: ✅ Yes ($500K limit)

Top Risk Drivers:
1. Amount limit check (15 failures, $75K exposure)
   → Recommendation: Reduce limit $10K → $8K
2. Market hours check (8 failures, $40K exposure)
   → Recommendation: Add pre-market validation
3. Position limit check (3 failures, $15K exposure)
   → Recommendation: Add real-time position lookup

Risk Trend: IMPROVING (↓ 0.05% vs. prior 30d)
```

**Board-level summary:** "Trading agent operated within risk appetite with 99.8% assertion pass rate and $119K VaR against $500K limit."

---

### 3. Third-Party Risk Management

**Example: Monitoring External LLM Provider Risk**

```json
// QoM report showing downgrade from GPT-4 to GPT-3.5
{
  "handshake_result": {
    "requested_model": "gpt-4",
    "selected_model": "gpt-3.5-turbo",
    "downgrade_reason": "Provider capacity limits",
    "timestamp": "2025-11-05T14:23:15Z"
  },
  "qom_report": {
    "profile": "qom-strict-argcheck",
    "metrics": {
      "schema_fidelity": 0.97,  // ↓ from 0.99 with GPT-4
      "instruction_compliance": 0.89  // ↓ from 0.95 with GPT-4
    }
  }
}
```

**Risk team alert:**

```
THIRD-PARTY RISK ALERT
Provider: OpenAI
Issue: Model downgrade (GPT-4 → GPT-3.5)
Impact: QoM degradation (SF: -2%, IC: -6%)
Frequency: 47 downgrades in last hour
Recommendation: Escalate to vendor; activate backup provider
```

**MPL enables:**
- Automated detection of vendor service degradation
- Quantified impact on quality metrics
- Evidence for vendor SLA breach discussions
- Trigger for multi-vendor failover

---

### 4. Incident Management & Root Cause Analysis

**Traditional incident (without MPL):**

```
INCIDENT: Trading agent placed invalid order
Time: ~14:00 UTC (user report, exact time unknown)
Impact: $8,500 loss from rejected order
Root Cause: ???
- Logs show JSON payload but schema unclear
- No evidence of validation checks
- Can't reproduce exact scenario
Remediation: "Improve validation" (vague)
```

**Same incident with MPL:**

```
INCIDENT: INC-2025-0342
────────────────────────────────────────────────
Error: E-QOM-BREACH
SType: org.trading.TradeOrder.v1
Timestamp: 2025-11-05T14:23:15.342Z (exact)
Semantic Hash: b3:7f2e9a1c...

ROOT CAUSE:
Assertion "amount-limit-check" failed:
  Expected: amount < $10,000
  Actual: amount = $12,500
  Rule: {"<": [{"var": "amount"}, 10000]}

TIMELINE (from provenance chain):
14:22:58 - User request received (ctx:conversation#msg-7a2b)
14:23:12 - Trading agent generated order
14:23:15 - QoM validation FAILED
14:23:15 - Request blocked, user notified
14:23:15 - Incident logged

IMPACT:
  Financial: $0 (order blocked before execution)
  Reputational: Low (user received clear error message)

REMEDIATION:
  Immediate: Working as designed; assertion prevented loss
  Long-term: Review why agent generated over-limit order
    → Check prompt engineering
    → Review model fine-tuning
    → Add pre-validation hint to agent prompt

EFFECTIVENESS EVIDENCE:
  Controls worked: ✅ Assertion caught violation
  Audit trail: ✅ Full reconstruction possible
  User impact: ✅ Minimal (clear error, no financial loss)
```

**Risk team value:** Precise, data-driven incident reports with quantified impact and verifiable controls.

---

## Organizational Workflow Integration

### Compliance Team → AI Safety Team → Risk Team

**Typical agent deployment workflow:**

```
1. Engineering builds agent
   ↓
2. AI Safety team reviews
   └─→ Concern: "Could this produce harmful outputs?"
   └─→ MPL solution: Add safety assertions to SType
   └─→ Evidence: QoM profile with safety test harness
   ↓
3. Risk team reviews
   └─→ Concern: "What's our operational/financial exposure?"
   └─→ MPL solution: Show VaR calculation from QoM metrics
   └─→ Evidence: Risk dashboard showing assertion pass rates
   ↓
4. Compliance team reviews
   └─→ Concern: "Can we audit this for regulators?"
   └─→ MPL solution: Semantic audit trail + controls-as-code
   └─→ Evidence: Provenance logs + QoM reports
   ↓
5. Approval granted
```

**Without MPL:** Each team asks questions, engineering scrambles to provide evidence, 12-18 weeks.

**With MPL:** All teams review same artifact (STypes + QoM profiles), answers are built-in, 4-6 weeks.

---

## What MPL Does NOT Provide (Honest Assessment)

### AI Safety Gaps

❌ **Model alignment** - MPL doesn't make models "want" to do the right thing
❌ **Adversarial robustness (ML attacks)** - Doesn't defend against adversarial examples, model poisoning
❌ **Bias detection** - Requires custom metrics; MPL provides enforcement, not detection
❌ **Semantic understanding** - Regex-based content filters are brittle
❌ **Automated red teaming** - Provides harness, not automated attack generation
❌ **Explainability** - Provenance shows "what happened," not "why model decided"

**Recommendation:** MPL complements AI Safety tools (evals, red teaming, alignment research), doesn't replace them.

---

### Risk Management Gaps

❌ **Predictive risk modeling** - MPL is reactive (monitors what happens), not predictive
❌ **Strategic risk** - Doesn't address market risk, competitive risk, regulatory change
❌ **Human risk** - Insider threats, social engineering, physical security
❌ **Business continuity** - Provides incident detection, not DR/BCP planning
❌ **Insurance coverage** - Risk quantification helps, but doesn't replace cyber insurance

**Recommendation:** MPL provides operational risk controls and monitoring; integrate with enterprise risk management (ERM) framework.

---

## Integration with Existing Tooling

### AI Safety Tooling

| Tool Category | Example Tools | MPL Integration |
|---------------|---------------|-----------------|
| **Evals & benchmarks** | HELM, Anthropic evals | Use QoM profiles as test harness |
| **Red teaming** | HarmBench, StrongREJECT | Encode attacks as test cases, measure defenses |
| **Content moderation** | OpenAI Moderation API, Perspective | Call as pre/post-check, log in provenance |
| **Bias detection** | Fairlearn, AI Fairness 360 | Custom IC assertions for fairness metrics |
| **Model cards** | Model Card Toolkit | Include STypes + QoM profiles in model card |

**Pattern:** MPL provides enforcement layer; AI Safety tools provide detection/measurement.

---

### Risk Management Tooling

| Tool Category | Example Tools | MPL Integration |
|---------------|---------------|-----------------|
| **GRC platforms** | ServiceNow IRM, LogicGate | Feed QoM metrics as control evidence |
| **SIEM / Observability** | Splunk, Datadog | Export MPL telemetry for correlation |
| **Incident management** | PagerDuty, Jira | Trigger incidents on E-QOM-BREACH |
| **Risk modeling** | SAS, @RISK | Use QoM metrics as input variables |
| **Audit management** | AuditBoard, HighBond | Export provenance logs for audit trail |

**Pattern:** MPL is data source for risk systems; doesn't replace risk workflow tools.

---

## Messaging for AI Safety & Risk Teams

### AI Safety Team Pitch

**"MPL provides the enforcement layer for your safety constraints—without replacing your evals and red teaming."**

**Value props:**
1. **Encode safety rules as assertions** - Hard constraints, not soft guidelines
2. **Catch failures before user impact** - Output validation blocks harmful responses
3. **Reproducible safety testing** - QoM profiles = version-controlled test harnesses
4. **Data-driven incident response** - Exact replay of safety failures for systematic fixes

**What we're NOT claiming:** MPL makes models safer. It makes your safety constraints enforceable.

---

### Risk Team Pitch

**"MPL quantifies and monitors AI operational risk in real-time—turning agent deployments from 'unmanaged risk' to 'controlled exposure.'"**

**Value props:**
1. **Real-time risk dashboard** - QoM metrics, assertion pass rates, incident counts
2. **Quantified exposure** - VaR calculations from QoM breach rates
3. **Automated control enforcement** - Assertions execute on every request
4. **Forensic reconstruction** - Provenance enables precise root cause analysis

**What we're NOT claiming:** MPL replaces ERM. It provides operational controls and telemetry.

---

## Success Metrics (AI Safety & Risk Alignment)

### AI Safety Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Safety assertion coverage** | >95% of agent outputs | % requests with safety profile |
| **Safety failure detection** | <1 hour to detect | Time from output → incident trigger |
| **Safety test pass rate** | >98% on red team suite | QoM profile pass rate |
| **Harmful output escapes** | <0.1% of requests | Manual review of incidents |

### Risk Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Risk visibility** | 100% agent workflows monitored | % agents with QoM telemetry |
| **Control effectiveness** | >99% assertion pass rate | QoM IC metric |
| **Incident detection time** | <5 minutes | Alert latency from E-QOM-BREACH |
| **RCA completion time** | <2 hours | Time to root cause via provenance |
| **Financial exposure** | Within risk appetite | VaR from QoM breaches vs. limit |

---

## Next Steps for AI Safety & Risk Team Adoption

**Note:** These are **adoption phases** (deploying MPL), not to be confused with **development phases** in `docs/mvp-scope.md` (building MPL itself).

### Adoption Phase 1: Pilot (4-6 weeks)

**AI Safety team:**
1. Select one high-risk agent (customer-facing, sensitive domain)
2. Define safety assertions (harmful content, PII leakage, policy violations)
3. Create `qom-safety-redteam` profile with adversarial test cases
4. Measure before/after: % harmful outputs escaping

**Risk team:**
1. Select one high-exposure agent (financial, operational)
2. Define risk metrics (VaR, operational loss, incident count)
3. Build risk dashboard with MPL telemetry
4. Present to CRO: "Here's our AI operational risk, quantified"

### Adoption Phase 2: Scale (3-6 months)

**AI Safety team:**
- Build centralized safety assertion library
- Mandate safety profiles for all production agents
- Integrate with existing eval/red team workflows
- Train ML engineers on safety assertion authoring

**Risk team:**
- Integrate MPL metrics into enterprise risk reporting
- Define risk appetite thresholds per agent type
- Build automated alerting for risk limit breaches
- Include AI operational risk in Board risk reports

---

## Conclusion

**Compliance teams:** MPL provides audit trails and controls-as-code (✅ **strong fit**)

**AI Safety teams:** MPL provides constraint enforcement and safety testing harnesses (⚠️ **partial fit** - complements but doesn't replace alignment research)

**Risk teams:** MPL provides real-time risk monitoring and quantified exposure (✅ **strong fit**)

**The organizational unlock:** All three teams review the same artifact (STypes + QoM profiles + provenance logs), reducing review cycles from 12-18 weeks to 4-6 weeks.

**Bottom line:** MPL doesn't solve AI safety or replace risk management—it provides the **enforcement and observability layer** that makes both teams' work auditable, measurable, and systematic.

---

**Related documents:**
- `docs/regulated-enterprise-value.md` - Compliance team value proposition
- `docs/adversarial-robustness.md` - Security team concerns
- `docs/security.md` - General security architecture
- `docs/qom-evaluation-engine.md` - QoM metric definitions
