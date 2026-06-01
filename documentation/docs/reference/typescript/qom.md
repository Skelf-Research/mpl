---
title: QoM
description: Quality of Meaning profiles, metrics, and evaluation for measuring AI agent output quality
---

# QoM (Quality of Meaning)

The QoM module defines profiles and metrics for measuring the quality of AI agent outputs. Profiles specify minimum thresholds for metrics like schema fidelity, instruction compliance, and tool outcome correctness.

---

## Import

```typescript
import {
  QomMetrics,
  QomProfile,
  QomProfileConfig,
  QomEvaluation,
  MetricFailure,
  MetricThreshold,
} from '@mpl/sdk';
```

---

## QomMetrics Interface

The set of measurable quality dimensions for an AI agent response.

```typescript
interface QomMetrics {
  /** Schema Fidelity - 1.0 if payload matches schema, 0.0 if invalid */
  schemaFidelity: number;
  /** Instruction Compliance - assertion pass rate (0.0 to 1.0) */
  instructionCompliance?: number;
  /** Groundedness - claim support score (0.0 to 1.0) */
  groundedness?: number;
  /** Determinism under Jitter - consistency across repeated calls (0.0 to 1.0) */
  determinismJitter?: number;
  /** Ontology Adherence - semantic constraint compliance (0.0 to 1.0) */
  ontologyAdherence?: number;
  /** Tool Outcome Correctness - business logic validation (0.0 to 1.0) */
  toolOutcomeCorrectness?: number;
}
```

| Metric | Range | Description |
|--------|-------|-------------|
| `schemaFidelity` | 0.0 - 1.0 | Binary: 1.0 if the payload validates against the schema, 0.0 otherwise |
| `instructionCompliance` | 0.0 - 1.0 | Proportion of prompt assertions that the output satisfies |
| `groundedness` | 0.0 - 1.0 | How well output claims are supported by provided context |
| `determinismJitter` | 0.0 - 1.0 | Semantic similarity across multiple calls with same input |
| `ontologyAdherence` | 0.0 - 1.0 | Compliance with domain ontology constraints beyond schema |
| `toolOutcomeCorrectness` | 0.0 - 1.0 | Whether the tool achieved the intended business outcome |

!!! info "Required vs Optional Metrics"
    Only `schemaFidelity` is required. All other metrics are optional and only evaluated if the profile defines thresholds for them and they are present in the metrics object.

---

## MetricThreshold Interface

Defines acceptable bounds for a single metric.

```typescript
interface MetricThreshold {
  /** Minimum acceptable value (inclusive) */
  min?: number;
  /** Maximum acceptable value (inclusive) */
  max?: number;
}
```

| Property | Type | Description |
|----------|------|-------------|
| `min` | `number \| undefined` | Metric must be >= this value to pass |
| `max` | `number \| undefined` | Metric must be <= this value to pass |

Either `min`, `max`, or both can be specified:

```typescript
// Metric must be at least 0.95
const threshold: MetricThreshold = { min: 0.95 };

// Metric must be at most 0.1 (e.g., for error rates)
const errorThreshold: MetricThreshold = { max: 0.1 };

// Metric must be between 0.8 and 1.0
const rangeThreshold: MetricThreshold = { min: 0.8, max: 1.0 };
```

---

## QomProfileConfig Interface

Configuration for creating a QoM profile.

```typescript
interface QomProfileConfig {
  /** Profile name (e.g., "qom-basic", "qom-strict-argcheck") */
  name: string;
  /** Human-readable description */
  description?: string;
  /** Metric thresholds to enforce */
  metrics: {
    schemaFidelity?: MetricThreshold;
    instructionCompliance?: MetricThreshold;
    groundedness?: MetricThreshold;
    determinismJitter?: MetricThreshold;
    ontologyAdherence?: MetricThreshold;
    toolOutcomeCorrectness?: MetricThreshold;
  };
}
```

| Property | Type | Description |
|----------|------|-------------|
| `name` | `string` | Unique identifier for the profile |
| `description` | `string \| undefined` | Human-readable explanation |
| `metrics` | `object` | Map of metric names to their thresholds |

---

## MetricFailure Interface

Details about a metric that did not meet its threshold.

```typescript
interface MetricFailure {
  /** Name of the metric that failed */
  metric: string;
  /** Actual measured value */
  actual: number;
  /** Required threshold value */
  threshold: number;
  /** Whether the failure was below min or above max */
  direction: 'min' | 'max';
}
```

| Property | Type | Description |
|----------|------|-------------|
| `metric` | `string` | The metric name (e.g., `"schemaFidelity"`) |
| `actual` | `number` | The measured value |
| `threshold` | `number` | The threshold that was not met |
| `direction` | `'min' \| 'max'` | Whether actual was below `min` or above `max` |

---

## QomEvaluation Interface

The result of evaluating metrics against a profile.

```typescript
interface QomEvaluation {
  /** Whether all metric thresholds are satisfied */
  meetsProfile: boolean;
  /** Name of the profile evaluated against */
  profile: string;
  /** The metrics that were evaluated */
  metrics: QomMetrics;
  /** List of failed thresholds (empty if meetsProfile is true) */
  failures: MetricFailure[];
}
```

| Property | Type | Description |
|----------|------|-------------|
| `meetsProfile` | `boolean` | `true` if all thresholds pass |
| `profile` | `string` | Profile name |
| `metrics` | `QomMetrics` | The input metrics |
| `failures` | `MetricFailure[]` | Detailed failure information |

---

## QomProfile Class

Defines quality thresholds and evaluates metrics against them.

```typescript
class QomProfile {
  readonly name: string;
  readonly description?: string;

  constructor(config: QomProfileConfig);

  static basic(): QomProfile;
  static strictArgcheck(): QomProfile;
  static outcome(): QomProfile;

  evaluate(metrics: QomMetrics): QomEvaluation;
}
```

---

### Constructor

```typescript
constructor(config: QomProfileConfig)
```

Creates a custom profile with the specified thresholds.

```typescript
const customProfile = new QomProfile({
  name: 'medical-grade',
  description: 'High-fidelity profile for medical AI agents',
  metrics: {
    schemaFidelity: { min: 1.0 },
    instructionCompliance: { min: 0.99 },
    groundedness: { min: 0.95 },
    determinismJitter: { min: 0.90 },
    ontologyAdherence: { min: 0.98 },
  },
});
```

---

### QomProfile.basic()

Create a basic profile that only requires schema fidelity.

```typescript
static basic(): QomProfile
```

**Equivalent to:**

```typescript
new QomProfile({
  name: 'qom-basic',
  description: 'Basic QoM profile requiring schema fidelity',
  metrics: {
    schemaFidelity: { min: 1.0 },
  },
});
```

| Metric | Threshold |
|--------|-----------|
| `schemaFidelity` | >= 1.0 |

Use this when you only need to ensure the output matches the declared schema.

---

### QomProfile.strictArgcheck()

Create a strict profile requiring both schema fidelity and instruction compliance.

```typescript
static strictArgcheck(): QomProfile
```

**Equivalent to:**

```typescript
new QomProfile({
  name: 'qom-strict-argcheck',
  description: 'Strict QoM profile with schema fidelity and instruction compliance',
  metrics: {
    schemaFidelity: { min: 1.0 },
    instructionCompliance: { min: 0.95 },
  },
});
```

| Metric | Threshold |
|--------|-----------|
| `schemaFidelity` | >= 1.0 |
| `instructionCompliance` | >= 0.95 |

Use this when the agent must both produce valid schemas and follow instructions precisely.

---

### QomProfile.outcome()

Create an outcome-focused profile that validates business logic correctness.

```typescript
static outcome(): QomProfile
```

**Equivalent to:**

```typescript
new QomProfile({
  name: 'qom-outcome',
  description: 'QoM profile focused on tool outcome correctness',
  metrics: {
    schemaFidelity: { min: 1.0 },
    toolOutcomeCorrectness: { min: 0.9 },
  },
});
```

| Metric | Threshold |
|--------|-----------|
| `schemaFidelity` | >= 1.0 |
| `toolOutcomeCorrectness` | >= 0.9 |

Use this for tool-calling agents where the correctness of the tool's effect matters more than instruction following.

---

### evaluate()

Evaluate a set of metrics against this profile's thresholds.

```typescript
evaluate(metrics: QomMetrics): QomEvaluation
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `metrics` | `QomMetrics` | The measured metrics to evaluate |

**Returns:** `QomEvaluation` with pass/fail status and failure details.

#### Evaluation Rules

1. Each metric defined in the profile is checked against the provided metrics
2. If a metric is not present in the input (undefined), it is **skipped** (not a failure)
3. If present, it must satisfy both `min` and `max` bounds (if defined)
4. The profile passes only if **all** defined metrics meet their thresholds

#### Examples

**Passing evaluation:**

```typescript
const profile = QomProfile.strictArgcheck();

const result = profile.evaluate({
  schemaFidelity: 1.0,
  instructionCompliance: 0.98,
});

console.log(result.meetsProfile); // true
console.log(result.failures);     // []
```

**Failing evaluation:**

```typescript
const profile = QomProfile.strictArgcheck();

const result = profile.evaluate({
  schemaFidelity: 1.0,
  instructionCompliance: 0.85,  // Below 0.95 threshold
});

console.log(result.meetsProfile); // false
console.log(result.failures);
// [{
//   metric: 'instructionCompliance',
//   actual: 0.85,
//   threshold: 0.95,
//   direction: 'min'
// }]
```

**Undefined metrics are skipped:**

```typescript
const profile = QomProfile.outcome();

// toolOutcomeCorrectness is not provided - skipped, not failed
const result = profile.evaluate({
  schemaFidelity: 1.0,
});

console.log(result.meetsProfile); // true (undefined metrics are skipped)
```

---

## Custom Profiles

Create domain-specific profiles for your use case:

```typescript
// Financial trading profile - high precision required
const tradingProfile = new QomProfile({
  name: 'financial-trading',
  description: 'Zero-tolerance profile for financial operations',
  metrics: {
    schemaFidelity: { min: 1.0 },
    instructionCompliance: { min: 1.0 },
    determinismJitter: { min: 0.99 },
    toolOutcomeCorrectness: { min: 1.0 },
  },
});

// Research assistant profile - groundedness matters most
const researchProfile = new QomProfile({
  name: 'research-assistant',
  description: 'Profile emphasizing factual accuracy',
  metrics: {
    schemaFidelity: { min: 1.0 },
    groundedness: { min: 0.9 },
    instructionCompliance: { min: 0.8 },
  },
});
```

---

## Using Profiles with Session

Profiles integrate with the `Session` for automatic enforcement:

```typescript
import { Session, QomBreachError } from '@mpl/sdk';

const session = new Session({
  endpoint: 'ws://localhost:9443/ws',
  stypes: ['org.calendar.Event.v1'],
  qomProfile: 'qom-strict-argcheck',  // Profile name sent in handshake
});

const capabilities = await session.connect();
console.log('Selected profile:', capabilities.selectedProfile);
// "qom-strict-argcheck"
```

When the proxy evaluates a response and the QoM report fails the profile, the error is surfaced:

```typescript
session.onMessage('org.calendar.Event.v1', (envelope) => {
  if (envelope.qomReport && !envelope.qomReport.meetsProfile) {
    console.warn('QoM breach detected!');
    for (const failure of envelope.qomReport.failures ?? []) {
      console.warn(`  ${failure.metric}: ${failure.actual} < ${failure.threshold}`);
    }
  }
});
```

---

## Profile Selection in AI-ALPN

During the AI-ALPN handshake, the client advertises its preferred QoM profiles and the server selects one:

```typescript
// Client sends:
{
  "type": "ai-alpn-hello",
  "qom_profiles": ["qom-strict-argcheck", "qom-basic"]
}

// Server responds with selected profile:
{
  "type": "ai-alpn-hello-ack",
  "selected_profile": "qom-strict-argcheck"
}
```

The server may select a different profile than requested, or none if it does not support QoM evaluation.

---

## See Also

- [Session](session.md) - QoM profile negotiation in sessions
- [Types](types.md) - QomReport attached to envelopes
- [Errors](errors.md) - QomBreachError for profile violations
- [Validation](validation.md) - Schema Fidelity metric source
