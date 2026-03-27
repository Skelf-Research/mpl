# Financial Advisor Workflow Example

This example demonstrates a complete MPL workflow for generating and managing investment recommendations in a regulated financial advisory context. It includes all necessary artifacts: SType schemas, tool descriptors, QoM profiles, and sample request/response payloads.

## Overview

The workflow shows:
1. **AI-ALPN handshake** negotiating capabilities, STypes, tools, and QoM profile with fiduciary duty policies
2. **Typed tool call** to generate an investment recommendation with full MPL envelope
3. **QoM validation** ensuring Schema Fidelity, Instruction Compliance, and regulatory compliance
4. **Typed response** with QoM report, compliance artifacts, and provenance

## Files

```
financial-advisor-workflow/
├── README.md                                      # This file
├── stypes/
│   ├── InvestmentRecommendation.v1.json          # Investment recommendation SType schema
│   └── Query.v1.json                              # Query SType schema
├── tools/
│   ├── advisor.recommend.v1.json                  # Recommendation tool descriptor
│   └── advisor.query.v1.json                      # Query tool descriptor
├── profiles/
│   └── qom-strict-argcheck.json                   # QoM profile for validation
├── requests/
│   ├── 01-handshake-client-hello.json             # Capability negotiation
│   ├── 02-handshake-server-select.json            # Server response
│   └── 03-generate-recommendation-request.json    # Typed tool call
└── responses/
    ├── 01-generate-recommendation-success.json    # Successful response with QoM
    └── 02-generate-recommendation-qom-breach.json # QoM breach example (fiduciary violation)
```

## Workflow Steps

### 1. AI-ALPN Handshake

Client proposes capabilities:
```bash
cat requests/01-handshake-client-hello.json
```

Server selects compatible subset:
```bash
cat requests/02-handshake-server-select.json
```

### 2. Generate Investment Recommendation

Client sends typed request:
```bash
cat requests/03-generate-recommendation-request.json
```

Server validates schema, QoM, and regulatory compliance, then responds:
```bash
cat responses/01-generate-recommendation-success.json
```

### 3. QoM Breach Handling (Fiduciary Duty Violation)

Example of QoM validation failure due to risk misalignment:
```bash
cat responses/02-generate-recommendation-qom-breach.json
```

This example demonstrates how MPL catches fiduciary duty violations when an agent recommends an aggressive investment (ARKK) to a conservative client, triggering compliance controls and remediation guidance.

## Running the Example

### Using the MPL SDK (Python)

```python
from mpl.sdk import Session

# Establish session with handshake
session = Session.connect(
    transport="wss://advisor.example.com",
    stypes=["org.finance.InvestmentRecommendation.v1"],
    tools=["advisor.recommend.v1"],
    profile="qom-strict-argcheck",
    policies=["policy.ref#fiduciary-duty-v1"]
)

# Generate recommendation with typed payload
response = session.call(
    tool="advisor.recommend.v1",
    payload={
        "recommendationId": "rec_001",
        "clientId": "client_abc123",
        "symbol": "VOO",
        "assetClass": "etf",
        "action": "buy",
        "amount": 10000.00,
        "allocationPercentage": 25.0,
        "rationale": "Based on client's moderate risk tolerance and 10-year time horizon...",
        "riskLevel": "moderate",
        "timeHorizon": "long_term",
        "confidenceScore": 0.89,
        "generatedAt": "2025-11-08T14:30:00Z"
    }
)

# Validate response
assert response.qom_report.meets_profile
print(f"Recommendation generated: {response.payload['recommendationId']}")
print(f"Compliance verified: {response.qom_report.metrics['instruction_compliance']}")
```

### Using the MPL Proxy

```bash
# Start MPL proxy with financial regulatory policies
mpl-proxy start \
  --upstream http://advisor-server:8080 \
  --registry https://registry.mpl.dev \
  --profile qom-strict-argcheck \
  --policies policy.ref#fiduciary-duty-v1

# Send request via proxy
curl -X POST http://localhost:9443/tools/advisor.recommend \
  -H "Content-Type: application/json" \
  -H "Semantic-Type: org.finance.InvestmentRecommendation.v1" \
  -d @requests/03-generate-recommendation-request.json
```

## Validation

### Schema Validation

```bash
# Validate request against SType schema
mpl-validate \
  --schema stypes/InvestmentRecommendation.v1.json \
  --payload requests/03-generate-recommendation-request.json
```

### QoM Evaluation

```bash
# Run QoM checks including fiduciary compliance
mpl-qom evaluate \
  --profile profiles/qom-strict-argcheck.json \
  --payload requests/03-generate-recommendation-request.json \
  --response responses/01-generate-recommendation-success.json
```

## Key Concepts Demonstrated

1. **Semantic Types (STypes):** `org.finance.InvestmentRecommendation.v1` declares recommendation structure with financial domain constraints
2. **Tool Descriptors:** `advisor.recommend.v1` specifies input/output STypes with regulatory policy bindings
3. **QoM Profile:** `qom-strict-argcheck` enforces SF=1.0, IC≥0.97 with financial-specific assertions (risk alignment, fiduciary compliance)
4. **Provenance:** tracks intent, risk assessments, client consent, and policy references across workflow for audit trails
5. **Typed Errors:** distinguishes schema failures from QoM breaches and regulatory violations
6. **Semantic Hashes:** detects payload tampering or drift - critical for compliance and audit
7. **Fiduciary Duty Enforcement:** demonstrates how MPL assertions catch unsuitable recommendations before execution

## Regulatory Compliance Demonstrated

This example shows how MPL supports financial regulatory requirements:

- **Suitability (FINRA Rule 2111):** Risk alignment assertions ensure recommendations match client profiles
- **Best Interest (Reg BI):** Fiduciary duty policy enforcement in tool descriptors
- **Audit Trail (SEC 17a-4):** Complete provenance chain with immutable semantic hashes
- **Risk Disclosure:** Mandatory disclaimer fields in recommendation schema
- **Consent Management:** Client consent references in provenance for GDPR/data governance

## Extension Points

- Add tax-loss harvesting support via feature flags
- Implement portfolio rebalancing constraints (maximum turnover, sector limits)
- Add policy enforcement for accredited investor verification
- Demonstrate jitter checks for determinism in portfolio optimization
- Show adapter usage for InvestmentRecommendation.v1 → v2 migration
- Add ESG screening and impact investing constraints
- Implement multi-asset portfolio optimization with correlation analysis

## References

- `docs/protocol-architecture.md` - Core MPL architecture
- `docs/mpl-with-mcp.md` - MCP integration details
- `docs/qom-evaluation-engine.md` - QoM metrics and enforcement
- `docs/regulated-enterprise-value.md` - Financial regulatory compliance mapping
- `GLOSSARY.md` - Term definitions
