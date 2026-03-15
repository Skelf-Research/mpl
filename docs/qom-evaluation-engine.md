# QoM Evaluation Engine

The Quality of Meaning (QoM) evaluation engine enforces semantic contracts negotiated by the Meaning Protocol Layer (MPL). This document describes its architecture, workflow, deployment patterns, and developer interfaces so teams can reason about how QoM metrics are computed and enforced across MCP and A2A integrations.

## 1. Goals & Responsibilities

- Validate payloads against negotiated STypes and QoM profiles.
- Produce structured reports (`qom_report`) summarising metric scores, pass/fail status, and artifact references.
- Emit typed errors (`E-QOM-BREACH`, `E-SCHEMA-FIDELITY`, etc.) with actionable hints when thresholds are not met.
- Provide hooks for retriable degradation (e.g., downgrade from `qom-strict-argcheck` to `qom-basic`) while maintaining auditability.
- Instrument observability sinks (metrics, logs) to quantify semantic quality across workflows.

## 2. Deployment Models

| Model | Description | When to use |
| --- | --- | --- |
| **Embedded SDK** | Evaluation engine runs inside the MPL client/server code. | Lightweight integrations; direct tool handlers. |
| **Sidecar service** | Dedicated process alongside proxy/orchestrator; invoked via gRPC/HTTP. | Shared infrastructure, language heterogeneity. |
| **Central service** | Multi-tenant QoM control plane serving many clients. | Enterprises needing central governance/audit. |

In all cases, the engine should expose the same API shape so applications can switch deployment models without code changes.

## 3. Inputs & Outputs

### Inputs

- **Payload:** MPL envelope payload (JSON/Protobuf) plus SType metadata.
- **Profile:** Negotiated QoM profile (thresholds, sampling policy).
- **Aux data:** Assertions, citation references, ontology constraints, post-check functions, provenance metadata.

### Outputs

- **Success:** `qom_report` containing per-metric scores, `meets_profile` boolean, and artifact references (claim sets, diffs, raw outputs).
- **Failure:** typed error struct with metric values, threshold mismatches, and remediation hints.
- **Telemetry:** structured events for metrics pipelines (e.g., Prometheus, OpenTelemetry).

## 4. Component Architecture

```
┌───────────────────┐
│ QoM API Layer     │  <-- gRPC/HTTP/SDK surface
└──────┬────────────┘
       │
┌──────▼────────────┐
│ Request Router    │  (loads profile, dispatches modules)
└──────┬────────────┘
       │
┌──────▼────────────┐
│ Schema Validator  │  (JSON Schema / Proto)
├──────┼────────────┤
│ Assertion Runner  │  (Instruction compliance)
├──────┼────────────┤
│ Grounding Module  │  (Claim extraction / citation check)
├──────┼────────────┤
│ Jitter Sampler    │  (Determinism under jitter)
├──────┼────────────┤
│ Ontology Checker  │  (SHACL/OWL/Rule engine)
├──────┼────────────┤
│ Post-check Hooks  │  (Tool outcome correctness)
└──────┴────────────┘
       │
┌──────▼────────────┐
│ Aggregator        │  (scores vs thresholds)
└──────┬────────────┘
       │
┌──────▼────────────┐
│ Report Generator  │  (`qom_report`, errors, telemetry)
└───────────────────┘
```

Each module is pluggable; deployments can enable only the metrics required by negotiated profiles to manage cost.

## 5. Workflow Steps

1. **Profile load:** fetch QoM profile definition (thresholds, modules) from cache/registry.
2. **Schema validation:**
   - Run JSON Schema/Protobuf validators.
   - Collect failure paths (e.g., `/end` missing, `type mismatch`).
   - On failure: raise `E-SCHEMA-FIDELITY`.
3. **Instruction compliance:**
   - Execute assertion scripts. Assertions can be:
     - Declarative (JSONLogic, CEL).
     - Imperative (JS/Python functions).
   - Track pass/fail counts.
4. **Groundedness:**
   - Extract claims via deterministic patterns (numbers, dates, named entities).
   - Resolve citations from provenance (`artifacts`, `sources`).
   - Check each claim’s support; compute ratio.
5. **Determinism under jitter:**
   - Trigger K reruns with controlled perturbations (temperature, context shuffling).
   - Use semantic similarity (BLEU/ROUGE/embedding cosine) to measure stability.
   - Average into `determinism_jitter`.
6. **Ontology adherence:**
   - Use SHACL/OWL/Rule engine to verify domain constraints (e.g., chronological order, enum membership).
7. **Tool outcome correctness:**
   - Execute post-check hook (read-after-write, external API check).
   - Return pass/fail boolean.
8. **Aggregation:**
   - Compare each metric against profile thresholds.
   - Flag metrics needing remediation.
   - Determine `meets_profile`.
9. **Report generation:**
   - Success: emit `qom_report` with metrics and artifact refs.
   - Failure: emit typed error (e.g., `E-QOM-BREACH` with metric {`determinism_jitter`:0.81}) and hints.
10. **Telemetry:**
    - Emit structured logs/events:
      - `qom.schema.failure`, `qom.ic.rate`, `qom.groundedness.score`.
    - Export metrics to Prometheus/OpenTelemetry.

## 6. API Surface

### 6.1 gRPC/HTTP schema (illustrative)

```proto
service QoMEngine {
  rpc Evaluate(QoMRequest) returns (QoMResponse);
}

message QoMRequest {
  string stype = 1;
  bytes payload = 2;              // json/protobuf blob
  string profile = 3;
  map<string, bytes> artifacts = 4; // citations, assertions, ontology refs
  map<string, string> metadata = 5; // provenance, policy info
}

message QoMResponse {
  bool meets_profile = 1;
  map<string, double> metrics = 2;
  repeated Artifact artifacts = 3;
  optional QoMError error = 4;
}

message QoMError {
  string code = 1;
  string hint = 2;
  map<string, double> metrics = 3;
}
```

### 6.2 SDK helpers

```python
from mpl.qom import evaluate

result = evaluate(
    stype="org.calendar.Event.v1",
    payload=event_payload,
    profile="qom-strict-argcheck",
    assertions=[assert_title_not_empty],
    citations=[citation_bundle],
    postcheck=lambda: verify_event_created(event_payload["eventId"])
)

if result.meets_profile:
    attach_qom_report(result.report)
else:
    handle_qom_error(result.error)
```

## 7. Configuration & Profiles

- Profiles are declarative JSON files stored in the registry:

```json
{
  "name": "qom-strict-argcheck",
  "metrics": {
    "schema_fidelity": {"min": 1.0},
    "instruction_compliance": {"min": 0.97},
    "groundedness": {"min": 0.95, "sample_rate": 0.5},
    "determinism_jitter": {"min": 0.95, "sample_rate": 0.2},
    "ontology_adherence": {"min": 0.98}
  },
  "retry_policy": {
    "max_retries": 1,
    "degrade_to": "qom-basic",
    "on_failure": "escalate"
  }
}
```

- Engine loads profile definitions into caches; updates via registry events or polling.
- Sample rates allow expensive metrics (groundedness, jitter) to run probabilistically.

## 8. Observability & Audit

- **Metrics:** expose counters/gauges:
  - `qom_schema_failures_total`
  - `qom_profile_pass_ratio{profile="qom-strict-argcheck"}`
  - `qom_determinism_avg`
- **Logs:** structured events with message ID, SType, metric scores, downgrade decisions.
- **Artifacts:** store detailed claim sets, diffs, or jitter outputs in CAS/object store; reference via URIs inside `qom_report`.
- **Audit trail:** semantic hashes plus QoM reports create a forensic trail for post-incident analysis.

## 9. Extensibility

- **Custom metrics:** allow pluggable modules (e.g., bias detection, toxicity checks) registered in the profile.
- **Language adapters:** provide bindings for multiple languages (Python, TS, Go) that call the same engine.
- **Policy integration:** share consent/policy context so evaluation modules can enforce redaction or scope rules as part of QoM.
- **Sandboxes:** run engines in secure sandboxes when evaluating untrusted payloads or third-party tools.

## 10. Developer Workflow

1. **Define profile:** create/update profile JSON in the registry.
2. **Implement assertions/post-checks:** register assertion scripts or hooks with the engine.
3. **Configure engine:** deploy via SDK, sidecar, or control plane; point to registry and telemetry endpoints.
4. **Integrate with MPL wrappers:** ensure responses include `qom_report` or typed errors; wire retry/degrade logic.
5. **Monitor metrics:** watch QoM dashboards to adjust thresholds, sampling rates, and assertions.
6. **Iterate:** refine metrics based on incident reviews and partner feedback; version profiles as requirements evolve.

With a dedicated QoM evaluation engine, MPL deployments maintain consistent semantic quality guarantees, measurable SLOs, and actionable diagnostics across MCP and A2A environments.

## 11. Technical Appendix: Metric Computation Details

### 11.1 Claim Extraction for Groundedness

**Deterministic patterns** identify verifiable claims in responses:

1. **Numerical claims:** Regex for numbers with units (e.g., `\d+(\.\d+)?\s*(dollars|meters|percent|GB)`).
2. **Date/time claims:** ISO 8601 patterns, natural language dates (e.g., "October 27, 2025").
3. **Named entities:** Use NER (spaCy, Stanza) to extract PERSON, ORG, GPE, DATE, MONEY.
4. **Factual assertions:** Declarative sentences with subject-verb-object structure.

**Extraction algorithm:**
```python
def extract_claims(text, artifacts):
    claims = []
    # Numerical claims
    claims.extend(re.findall(r'\d+(?:\.\d+)?\s*(?:dollars|meters|%)', text))
    # Date claims
    claims.extend(extract_dates(text))  # dateutil.parser
    # Named entities
    doc = nlp(text)  # spaCy NER
    claims.extend([ent.text for ent in doc.ents])
    # Factual assertions (heuristic: sentences with proper nouns + verbs)
    claims.extend(extract_factual_sentences(text))
    return claims
```

**Citation resolution:**
- Parse `provenance.artifacts` for referenced documents/URLs.
- For each claim, search citation text using:
  - Exact string match (high confidence).
  - Fuzzy match (Levenshtein distance <3 edits, 80%+ similarity).
  - Embedding similarity (cosine >0.85 between claim and citation spans).
- Return ratio: `supported_claims / total_claims`.

**Example:**
```
Response: "The meeting is scheduled for October 27, 2025 at 1:00 PM."
Claims extracted: ["October 27, 2025", "1:00 PM"]
Citations: {"artifacts": [{"ref": "calendar:event_123", "text": "Event starts 2025-10-27T13:00:00Z"}]}
Resolution: Both claims supported (date + time match citation) → groundedness = 1.0
```

### 11.2 Determinism under Jitter

**Controlled perturbations** measure output stability:

1. **Temperature variation:** Re-run with `temperature ± 0.1` (e.g., 0.0 → 0.1, 0.7 → 0.8).
2. **Context shuffling:** Reorder non-causal context elements (e.g., shuffle bullet points in prompt).
3. **Seed randomization:** Change random seed for sampling-based models.
4. **Token-level noise:** Introduce typos or synonym substitutions in <5% of input tokens.

**Jitter protocol:**
```python
def determinism_jitter(model, input, k=5):
    baseline = model.generate(input, temperature=0.7)
    variants = []
    for i in range(k):
        if i % 2 == 0:
            # Temperature jitter
            variant = model.generate(input, temperature=0.7 + random.uniform(-0.1, 0.1))
        else:
            # Context shuffle
            shuffled_input = shuffle_context(input)
            variant = model.generate(shuffled_input, temperature=0.7)
        variants.append(variant)

    # Measure similarity
    similarities = [semantic_similarity(baseline, v) for v in variants]
    return sum(similarities) / len(similarities)
```

**Similarity metrics:**
- **BLEU:** n-gram overlap (BLEU-4 for long-form text).
- **ROUGE-L:** longest common subsequence.
- **Embedding cosine:** sentence embeddings (e.g., `sentence-transformers`) with cosine similarity.
- **Semantic equivalence:** Use NLI model to check entailment (baseline entails variant?).

**Sampling strategy:**
- Full evaluation: `k=10` reruns (expensive, used in CI/staging).
- Production sampling: `k=2–3` with 10–20% sampling rate (cost-effective).
- Target threshold: DJ ≥ 0.95 for high-reliability workflows.

### 11.3 Canonicalization Algorithm

**Purpose:** deterministic payload normalization for semantic hashing.

**Steps:**
1. **Parse JSON:** load payload into object tree.
2. **Sort keys:** recursively sort all object keys alphabetically.
3. **Normalize types:**
   - Convert numbers to standard precision (avoid `1.0` vs `1.00` differences).
   - Normalize strings: trim whitespace, consistent Unicode encoding (NFC).
   - Sort arrays if order is semantically irrelevant (annotated in schema).
4. **Remove metadata:** strip comments, `$schema` directives, or debug fields.
5. **Serialize:** deterministic JSON encoding (no pretty-printing, consistent escaping).

**Example:**
```python
import json
import hashlib
from blake3 import blake3

def canonicalize(payload):
    # Sort keys recursively
    canonical = json.dumps(payload, sort_keys=True, separators=(',', ':'), ensure_ascii=False)
    # Normalize whitespace
    canonical = canonical.strip()
    return canonical

def compute_sem_hash(payload):
    canonical = canonicalize(payload)
    hash_bytes = blake3(canonical.encode('utf-8')).digest()
    return f"b3:{hash_bytes.hex()}"
```

**Edge cases:**
- **Floating-point precision:** round to 6 decimal places before canonicalization.
- **Datetime formats:** normalize all timestamps to ISO 8601 UTC.
- **Optional fields:** omit `null` fields vs. explicitly include them (schema must specify).

### 11.4 Ontology Adherence Validation

**Technologies:**
- **SHACL (Shapes Constraint Language):** RDF graph validation.
- **OWL (Web Ontology Language):** class hierarchies and property restrictions.
- **Custom rule engines:** CEL, JSONLogic, or imperative scripts.

**Example SHACL constraint (chronological order):**
```turtle
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix mpl: <urn:mpl:ontology#> .

mpl:EventShape a sh:NodeShape ;
    sh:targetClass mpl:Event ;
    sh:property [
        sh:path mpl:end ;
        sh:minExclusive [ sh:path mpl:start ] ;
        sh:message "Event end time must be after start time"
    ] .
```

**Validation workflow:**
1. Convert JSON payload to RDF graph (JSON-LD or custom mapping).
2. Load SHACL shapes from registry (cached).
3. Run SHACL validator (e.g., `pyshacl`).
4. Collect violations; compute adherence ratio: `(total_constraints - violations) / total_constraints`.

**Custom rule example (JSONLogic):**
```json
{
  "and": [
    {"<=": [{"var": "start"}, {"var": "end"}]},
    {"in": [{"var": "status"}, ["pending", "confirmed", "cancelled"]]}
  ]
}
```

### 11.5 Tool Outcome Correctness Post-Checks

**Verification strategies:**

1. **Read-after-write:**
   ```python
   def verify_calendar_create(payload, response):
       event_id = response["eventId"]
       fetched = calendar_api.get_event(event_id)
       return fetched["title"] == payload["title"] and \
              fetched["start"] == payload["start"]
   ```

2. **External API validation:**
   ```python
   def verify_payment(payload, response):
       transaction_id = response["transactionId"]
       status = payment_gateway.check_status(transaction_id)
       return status == "completed"
   ```

3. **Idempotency check:**
   ```python
   def verify_idempotent(payload, response1, response2):
       # Call tool twice with same payload
       return response1 == response2  # Must be identical
   ```

4. **Side-effect inspection:**
   ```python
   def verify_email_sent(payload, response):
       # Query email service logs
       logs = email_service.get_logs(recipient=payload["to"])
       return any(log["subject"] == payload["subject"] for log in logs)
   ```

**Failure handling:**
- Post-check failures trigger `E-TOOL-OUTCOME-INCORRECT`.
- Orchestrators can retry with alternative tools or escalate to human review.
- Audit logs include post-check details for forensics.

### 11.6 Performance Optimization

**Caching:**
- **Schema validators:** cache compiled JSON Schema validators (100x speedup).
- **Ontology shapes:** preload SHACL/OWL graphs (avoid parsing on every request).
- **Consent lookups:** Redis cache with TTL matching consent expiry.

**Sampling:**
- **Determinism jitter:** 10–20% sampling in production (probabilistic check).
- **Groundedness:** 50% sampling for non-critical workflows.
- **Post-checks:** 100% for critical ops (payments, writes), 10% for reads.

**Parallelization:**
- Schema + IC checks run concurrently (independent).
- Jitter reruns execute in parallel thread pool.
- Post-checks async (don't block response if non-critical).

**Estimated latencies (realistic, based on implementation-feasibility-analysis.md):**

**MVP (Phase 1) - SF + IC only:**
- Schema Fidelity: 2-5ms p50, 20-50ms p99 (with caching; first call may be 50-100ms)
- Instruction Compliance: 5-20ms p50 (simple assertions), up to 50ms (complex with context lookups)
- **Total MVP overhead:** 10-30ms p50, 100-200ms p99 (typical assertions)

**Phase 2+ Metrics (NOT in MVP):**
- Groundedness: 200-500ms (NER + citation search + model call) - **EXCLUDED from MVP** (research-grade)
- Determinism Jitter: 2-5s total (K=3 reruns × 500-2000ms each) - **EXCLUDED from MVP** (prohibitive cost)
- Ontology Adherence: 10-50ms (SHACL validation) - **DEFERRED to Phase 2**
- Tool Outcome: 100-500ms (external API call) - **DEFERRED to Phase 2**

**Note:** See `docs/mvp-scope.md` for MVP feature set. Full QoM suite requires Phase 2+ investment.
