---
title: Types
description: Core type definitions - SType identifiers, MplEnvelope message wrappers, Provenance, and QoM reports
---

# Types

The types module provides the foundational data structures used throughout the MPL SDK: semantic type identifiers (`SType`), message envelopes (`MplEnvelope`), provenance tracking, and QoM reporting.

---

## Import

```typescript
import {
  SType,
  STypeComponents,
  STypeParseError,
  MplEnvelope,
  MplEnvelopeOptions,
  Provenance,
  QomReport,
  MetricFailure,
} from '@mpl/sdk';
```

---

## SType

### STypeComponents Interface

The parsed components of an SType identifier.

```typescript
interface STypeComponents {
  namespace: string;
  domain: string;
  name: string;
  majorVersion: number;
}
```

| Property | Type | Description | Example |
|----------|------|-------------|---------|
| `namespace` | `string` | Organization or scope identifier | `"org"`, `"com.acme"` |
| `domain` | `string` | Functional area within the namespace | `"calendar"`, `"finance"` |
| `name` | `string` | PascalCase intent/entity name | `"Event"`, `"Transaction"` |
| `majorVersion` | `number` | Major version number | `1`, `2` |

---

### SType Class

Immutable representation of a parsed Semantic Type identifier.

```typescript
class SType {
  readonly namespace: string;
  readonly domain: string;
  readonly name: string;
  readonly majorVersion: number;

  static parse(stypeStr: string): SType;
  static create(namespace: string, domain: string, name: string, majorVersion: number): SType;

  id(): string;
  urn(): string;
  registryPath(): string;
  toString(): string;
  toJSON(): string;
}
```

!!! note "Private Constructor"
    `SType` uses a private constructor. Instances are created via `SType.parse()` or `SType.create()`.

---

#### SType.parse()

Parse an SType from its string representation.

```typescript
static parse(stypeStr: string): SType
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `stypeStr` | `string` | String in format `"namespace.domain.Name.vMajor"` |

**Returns:** A new `SType` instance.

**Throws:** `STypeParseError` if the format is invalid.

**Parsing rules:**

1. The string is split by `.` (dots)
2. The last segment must match `vN` (e.g., `v1`, `v2`)
3. The second-to-last segment is the name (must start with uppercase)
4. Remaining segments are split into namespace (all but last) and domain (last)
5. At least 4 segments are required

```typescript
const stype = SType.parse('org.calendar.Event.v1');
console.log(stype.namespace);     // "org"
console.log(stype.domain);        // "calendar"
console.log(stype.name);          // "Event"
console.log(stype.majorVersion);  // 1
```

**Multi-segment namespace:**

```typescript
const stype = SType.parse('com.acme.finance.Transaction.v2');
console.log(stype.namespace);     // "com.acme"
console.log(stype.domain);        // "finance"
console.log(stype.name);          // "Transaction"
console.log(stype.majorVersion);  // 2
```

---

#### SType.create()

Create an SType from individual components.

```typescript
static create(
  namespace: string,
  domain: string,
  name: string,
  majorVersion: number,
): SType
```

```typescript
const stype = SType.create('org', 'calendar', 'Event', 1);
console.log(stype.id()); // "org.calendar.Event.v1"
```

---

#### id()

Get the canonical string representation.

```typescript
id(): string
```

**Returns:** `"namespace.domain.Name.vMajor"` format.

```typescript
const stype = SType.parse('org.calendar.Event.v1');
console.log(stype.id()); // "org.calendar.Event.v1"
```

---

#### urn()

Get the full URN representation.

```typescript
urn(): string
```

**Returns:** `"urn:stype:namespace.domain.Name.vMajor"` format.

```typescript
const stype = SType.parse('org.calendar.Event.v1');
console.log(stype.urn()); // "urn:stype:org.calendar.Event.v1"
```

---

#### registryPath()

Get the filesystem path for this SType in a registry.

```typescript
registryPath(): string
```

**Returns:** Path in format `"stypes/namespace/domain/Name/vMajor"`.

```typescript
const stype = SType.parse('org.calendar.Event.v1');
console.log(stype.registryPath());
// "stypes/org/calendar/Event/v1"
```

---

### STypeParseError

Thrown when an SType string cannot be parsed.

```typescript
class STypeParseError extends Error {
  constructor(message: string);
}
```

```typescript
try {
  SType.parse('invalid-format');
} catch (error) {
  if (error instanceof STypeParseError) {
    console.error(error.message);
    // "Invalid SType format: invalid-format. Expected namespace.domain.Name.vMajor"
  }
}
```

**Common parse errors:**

| Input | Error |
|-------|-------|
| `"foo.bar"` | Too few segments (need at least 4) |
| `"org.cal.Event.1"` | Version must start with `v` |
| `"org.cal.event.v1"` | Name must start with uppercase |
| `"org.Event.v1"` | Missing domain segment |

---

## MplEnvelope

### MplEnvelopeOptions Interface

Options for constructing an MplEnvelope.

```typescript
interface MplEnvelopeOptions {
  /** UUID for the envelope. Auto-generated if not provided. */
  id?: string;
  /** SType identifier for the payload. */
  stype: string;
  /** The payload data. */
  payload: Record<string, unknown>;
  /** SType for the arguments (if this is a tool call response). */
  argsStype?: string;
  /** QoM profile used for this envelope. */
  profile?: string;
  /** Pre-computed semantic hash. */
  semHash?: string;
  /** Feature flags for this envelope. */
  features?: string[];
  /** Provenance information. */
  provenance?: Provenance;
}
```

---

### MplEnvelope Class

The envelope wraps typed payloads with metadata for routing, validation, and auditing.

```typescript
class MplEnvelope {
  readonly id: string;
  stype: string;
  payload: Record<string, unknown>;
  argsStype?: string;
  profile?: string;
  semHash?: string;
  features: string[];
  provenance?: Provenance;
  qomReport?: QomReport;

  constructor(options: MplEnvelopeOptions);
  static fromJSON(json: string): MplEnvelope;
  toJSON(): string;
  toObject(): Record<string, unknown>;
}
```

| Property | Type | Description |
|----------|------|-------------|
| `id` | `string` | Unique identifier (UUID v4, auto-generated) |
| `stype` | `string` | SType identifier for the payload |
| `payload` | `Record<string, unknown>` | The actual message data |
| `argsStype` | `string \| undefined` | SType of the request arguments (for responses) |
| `profile` | `string \| undefined` | QoM profile governing this envelope |
| `semHash` | `string \| undefined` | Semantic hash of the payload (`b3:...` format) |
| `features` | `string[]` | Feature flags (e.g., `["streaming", "batch"]`) |
| `provenance` | `Provenance \| undefined` | Origin and intent tracking |
| `qomReport` | `QomReport \| undefined` | QoM evaluation results (attached by proxy) |

---

#### Constructor

```typescript
constructor(options: MplEnvelopeOptions)
```

Creates an envelope with an auto-generated UUID if `id` is not provided.

```typescript
const envelope = new MplEnvelope({
  stype: 'org.calendar.Event.v1',
  payload: {
    title: 'Architecture Review',
    start: '2024-01-15T14:00:00Z',
  },
  profile: 'qom-strict-argcheck',
  provenance: {
    intent: 'create-event',
    inputsRef: ['user-request-123'],
  },
});

console.log(envelope.id); // "a1b2c3d4-..." (auto-generated UUID)
```

---

#### fromJSON()

Deserialize an envelope from a JSON string.

```typescript
static fromJSON(json: string): MplEnvelope
```

Handles both camelCase and snake_case property names for wire compatibility:

```typescript
const json = `{
  "id": "msg-001",
  "stype": "org.calendar.Event.v1",
  "payload": { "title": "Meeting" },
  "args_stype": "org.calendar.CreateArgs.v1",
  "sem_hash": "b3:abc123...",
  "profile": "qom-basic"
}`;

const envelope = MplEnvelope.fromJSON(json);
console.log(envelope.argsStype); // "org.calendar.CreateArgs.v1"
console.log(envelope.semHash);   // "b3:abc123..."
```

---

#### toJSON()

Serialize the envelope to a JSON string (snake_case for wire format).

```typescript
toJSON(): string
```

```typescript
const envelope = new MplEnvelope({
  stype: 'org.calendar.Event.v1',
  payload: { title: 'Meeting' },
  semHash: 'b3:abc123def456',
});

console.log(envelope.toJSON());
// {
//   "id": "a1b2c3d4-...",
//   "stype": "org.calendar.Event.v1",
//   "payload": { "title": "Meeting" },
//   "sem_hash": "b3:abc123def456",
//   ...
// }
```

---

#### toObject()

Convert to a plain JavaScript object (snake_case keys).

```typescript
toObject(): Record<string, unknown>
```

---

## Provenance Interface

Tracks the origin and intent of a message for auditability.

```typescript
interface Provenance {
  /** The intent or purpose of this message */
  intent?: string;
  /** References to input messages that led to this output */
  inputsRef?: string[];
  /** Parent envelope ID (for request-response chains) */
  parentId?: string;
  /** Timestamp of creation */
  timestamp?: string;
}
```

| Property | Type | Description |
|----------|------|-------------|
| `intent` | `string \| undefined` | Human-readable description of purpose |
| `inputsRef` | `string[] \| undefined` | IDs of input envelopes that produced this output |
| `parentId` | `string \| undefined` | ID of the parent envelope in a chain |
| `timestamp` | `string \| undefined` | ISO 8601 timestamp |

#### Example

```typescript
const envelope = new MplEnvelope({
  stype: 'org.calendar.Event.v1',
  payload: { title: 'Generated Meeting' },
  provenance: {
    intent: 'auto-schedule',
    inputsRef: ['user-msg-001', 'calendar-query-002'],
    parentId: 'request-envelope-789',
    timestamp: new Date().toISOString(),
  },
});
```

---

## QomReport Interface

Quality of Meaning evaluation results, typically attached by the MPL proxy after processing.

```typescript
interface QomReport {
  /** Schema Fidelity score (1.0 = fully valid) */
  schemaFidelity: number;
  /** Instruction Compliance score */
  instructionCompliance?: number;
  /** Groundedness score */
  groundedness?: number;
  /** Determinism under Jitter score */
  determinismJitter?: number;
  /** Tool Outcome Correctness score */
  toolOutcomeCorrectness?: number;
  /** Whether the profile requirements are met */
  meetsProfile: boolean;
  /** Name of the evaluated profile */
  profile: string;
  /** List of metric failures, if any */
  failures?: MetricFailure[];
}
```

| Property | Type | Description |
|----------|------|-------------|
| `schemaFidelity` | `number` | 1.0 if schema valid, 0.0 if invalid |
| `instructionCompliance` | `number \| undefined` | How well instructions were followed (0.0-1.0) |
| `groundedness` | `number \| undefined` | Whether claims are supported by evidence (0.0-1.0) |
| `determinismJitter` | `number \| undefined` | Consistency across repeated calls (0.0-1.0) |
| `toolOutcomeCorrectness` | `number \| undefined` | Business logic validation score (0.0-1.0) |
| `meetsProfile` | `boolean` | `true` if all profile thresholds are met |
| `profile` | `string` | Name of the profile evaluated against |
| `failures` | `MetricFailure[] \| undefined` | Details of failed metric thresholds |

---

## MetricFailure Interface

Details of a single QoM metric that did not meet the profile threshold.

```typescript
interface MetricFailure {
  /** Name of the metric that failed */
  metric: string;
  /** Actual measured value */
  actual: number;
  /** Required threshold value */
  threshold: number;
}
```

#### Example

```typescript
// Inspecting QoM results on a received envelope
session.onMessage('org.calendar.Event.v1', (envelope) => {
  if (envelope.qomReport) {
    console.log('Schema Fidelity:', envelope.qomReport.schemaFidelity);
    console.log('Meets Profile:', envelope.qomReport.meetsProfile);

    if (envelope.qomReport.failures?.length) {
      for (const failure of envelope.qomReport.failures) {
        console.warn(
          `${failure.metric}: got ${failure.actual}, needed ${failure.threshold}`
        );
      }
    }
  }
});
```

---

## Wire Format

The envelope uses snake_case on the wire and camelCase in TypeScript:

| TypeScript Property | Wire Format | Example |
|--------------------|-------------|---------|
| `argsStype` | `args_stype` | `"org.calendar.CreateArgs.v1"` |
| `semHash` | `sem_hash` | `"b3:abc123..."` |
| `qomReport` | `qom_report` | `{ ... }` |

This mapping is handled automatically by `fromJSON()` and `toJSON()`.

---

## See Also

- [Validation](validation.md) - Schema validation for payloads
- [Hashing](hashing.md) - How `semHash` is computed
- [QoM](qom.md) - QoM profile definitions and evaluation
- [Session](session.md) - Sending and receiving envelopes
