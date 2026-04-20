# Quality of Meaning (QoM) Guide

This guide covers how to use MPL's Quality of Meaning (QoM) system to ensure AI agent outputs meet your quality and safety requirements.

## Overview

QoM is a framework for measuring and enforcing quality standards on AI agent outputs. It provides six complementary metrics:

| Metric | Abbrev | What it Measures |
|--------|--------|------------------|
| Schema Fidelity | SF | JSON Schema conformance |
| Instruction Compliance | IC | Business rule adherence (via CEL assertions) |
| Tool Outcome Correctness | TOC | Side-effect verification |
| Groundedness | G | Citation/source support |
| Determinism Jitter | DJ | Output stability across runs |
| Ontology Adherence | OA | Domain constraint conformance |

## Quick Start

### 1. Enable QoM in the Proxy

Start the proxy with a QoM profile:

```bash
mpl proxy start --profile qom-strict-argcheck
```

### 2. Create Assertions for Your STypes

Create an `assertions.json` file in your registry:

```
registry/stypes/my/namespace/MyType/v1/assertions.json
```

```json
{
  "$schema": "https://mpl.dev/schemas/assertions.json",
  "stype": "my.namespace.MyType.v1",
  "name": "my_type_validations",
  "description": "Business logic assertions for MyType",
  "assertions": [
    {
      "id": "field_positive",
      "expression": "payload.count > 0",
      "message": "Count must be positive",
      "severity": "error",
      "tags": ["required"]
    }
  ]
}
```

### 3. Monitor QoM Metrics

Access the QoM dashboard:
- Dashboard: `http://localhost:8080/_mpl/qom`
- Events: `http://localhost:8080/_mpl/qom/events`
- History: `http://localhost:8080/_mpl/qom/history?period=24h`

## QoM Profiles

MPL includes four pre-defined profiles:

### qom-basic
- **SF = 1.0** (schema must pass)
- Best for: Development, initial adoption

### qom-strict-argcheck
- **SF = 1.0**
- **IC >= 0.97**
- Best for: Production with business rules

### qom-outcome
- **SF = 1.0**
- **TOC >= 0.9**
- Best for: Tool-using agents with side effects

### qom-comprehensive
- **SF = 1.0**
- **IC >= 0.95**
- **TOC >= 0.9**
- **G >= 0.8**
- **DJ >= 0.9**
- **OA >= 0.95**
- Best for: High-stakes applications (healthcare, finance)

## Writing CEL Assertions

Assertions use the [Common Expression Language (CEL)](https://github.com/google/cel-spec) for expressive validation rules.

### Basic Syntax

```json
{
  "id": "unique_assertion_id",
  "expression": "payload.field > 0",
  "message": "Field must be positive",
  "severity": "error",
  "tags": ["category"]
}
```

### Severity Levels

- **error**: Fails validation, blocks in strict mode
- **warning**: Logged, contributes to IC score
- **info**: Logged only, no impact on score

### Common Patterns

#### Field existence
```cel
has(payload.optional_field)
```

#### String validation
```cel
size(payload.name) >= 3 && size(payload.name) <= 100
payload.email.matches('^[a-zA-Z0-9+_.-]+@[a-zA-Z0-9.-]+$')
payload.status in ['active', 'pending', 'completed']
```

#### Numeric ranges
```cel
payload.amount >= 0 && payload.amount <= 10000
payload.percentage >= 0.0 && payload.percentage <= 1.0
```

#### Array validation
```cel
size(payload.items) > 0
size(payload.items) <= 100
payload.items.all(item, has(item.id) && size(item.id) > 0)
payload.tags.exists(t, t == 'required')
```

#### Conditional logic
```cel
payload.status != 'error' || has(payload.error_message)
!has(payload.discount) || payload.discount <= payload.price
```

#### Cross-field validation
```cel
payload.end_date >= payload.start_date
payload.min_value < payload.max_value
```

### Context Variables

Assertions can access context beyond the payload:

```cel
context.stype == 'my.namespace.Type.v1'
context.timestamp > timestamp('2024-01-01T00:00:00Z')
```

## Tool Outcome Correctness (TOC)

TOC verifies that tool calls produce the expected side effects.

### Header-Based Verification

Return a TOC result in the response header:

```http
X-MPL-TOC-Result: verified
```

Values: `verified`, `failed`, `pending`, `skip`

### Callback-Based Verification

1. Proxy generates a callback ID
2. External system verifies the outcome
3. Reports back via callback endpoint

```bash
curl -X POST http://localhost:8080/_mpl/toc/callback \
  -H "Content-Type: application/json" \
  -d '{
    "callback_id": "toc-0000000000000001",
    "verified": true,
    "details": "File was created successfully"
  }'
```

### Checking TOC Status

```bash
curl http://localhost:8080/_mpl/toc/status/toc-0000000000000001
```

## Groundedness Checking

Groundedness measures how well responses are supported by source documents.

### How It Works

1. Claims are extracted from the response
2. Each claim is matched against provided sources
3. Similarity scoring determines groundedness

### Source Documents

Provide sources in the MPL envelope:

```json
{
  "stype": "eval.rag.RAGResponse.v1",
  "payload": {
    "answer": "Paris is the capital of France",
    "sources": [
      {
        "documentId": "doc1",
        "content": "Paris is the capital city of France...",
        "relevanceScore": 0.95
      }
    ]
  }
}
```

### Configuration

In your proxy config:

```yaml
qom:
  groundedness:
    similarity_threshold: 0.7
    auto_extract_claims: true
    use_llm_fallback: false
```

## Determinism Jitter

Measures output stability across multiple runs of the same request.

### How It Works

1. Proxy tracks response history by request signature
2. New responses are compared to historical responses
3. Jitter score = 1 - similarity

### What's Compared

- JSON structure differences
- Value changes
- Array length changes

### Ignored Fields

By default, these fields are ignored:
- `timestamp`
- `created_at`
- `updated_at`
- `request_id`
- `trace_id`

## Ontology Adherence

Enforces domain-specific constraints beyond JSON Schema.

### Creating an Ontology

```
registry/stypes/my/namespace/MyType/v1/ontology.json
```

```json
{
  "name": "my_type_ontology",
  "description": "Domain constraints for MyType",
  "allowed_values": {
    "status": ["draft", "pending", "approved", "rejected"]
  },
  "relationships": [
    {
      "id": "approved_requires_reviewer",
      "from": "status",
      "to": "reviewer_id",
      "relation_type": "implies",
      "message": "Approved items must have a reviewer"
    }
  ],
  "type_constraints": {
    "email": {
      "semantic_type": "email",
      "message": "Must be a valid email address"
    }
  }
}
```

### Relationship Types

- **implies**: If A exists, B must exist
- **excludes**: A and B cannot both exist
- **less_than**: A < B
- **equals**: A == B
- **contains**: A contains B (for arrays/strings)

## API Reference

### GET /_mpl/qom
Returns current QoM metrics summary.

### GET /_mpl/qom/events?limit=50
Returns recent QoM evaluation events.

### GET /_mpl/qom/history?period=24h
Returns historical QoM data for trends.
Periods: `1h`, `6h`, `24h`, `7d`

### POST /_mpl/qom/persist
Persists QoM history to disk.

### POST /_mpl/toc/callback
Receives TOC verification callbacks.

### GET /_mpl/toc/status/{callback_id}
Checks TOC verification status.

### GET /_mpl/toc/pending
Lists pending TOC verifications.

## Best Practices

### 1. Start Simple
Begin with `qom-basic` and add assertions incrementally.

### 2. Use Severity Wisely
- **error**: Things that should block the request
- **warning**: Important but not blocking
- **info**: Nice-to-know observations

### 3. Tag Your Assertions
Tags help organize and filter assertions:
```json
"tags": ["security", "pii", "required"]
```

### 4. Document Your Assertions
Write clear messages that help developers fix issues:
```json
"message": "Email must be a valid format (e.g., user@example.com)"
```

### 5. Profile Degradation
Configure fallback profiles for graceful degradation:
```
qom-comprehensive -> qom-outcome -> qom-strict-argcheck -> qom-basic
```

### 6. Monitor Trends
Use the history API to detect regressions:
```bash
curl http://localhost:8080/_mpl/qom/history?period=7d
```

## Troubleshooting

### Assertions Not Loading
- Check file is valid JSON
- Verify file path matches SType structure
- Check proxy logs for parse errors

### Low IC Scores
- Review which assertions are failing
- Check assertion expressions for correctness
- Consider adjusting severity levels

### TOC Always Pending
- Ensure external system calls callback
- Check callback endpoint is accessible
- Verify callback_id matches

### High Jitter
- Check for non-deterministic fields
- Add fields to ignore list
- Consider if jitter is expected

## Examples

See complete examples in the registry:
- `registry/stypes/org/feedback/Rating/v1/assertions.json` - Basic validations
- `registry/stypes/eval/rag/RAGResponse/v1/assertions.json` - RAG quality
- `registry/stypes/org/agent/ToolInvocation/v1/assertions.json` - Tool safety
- `registry/stypes/org/finance/InvestmentRecommendation/v1/assertions.json` - Fiduciary compliance
- `registry/stypes/org/healthcare/PatientSummary/v1/assertions.json` - HIPAA-aware
