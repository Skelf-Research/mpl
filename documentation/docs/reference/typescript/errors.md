---
title: Errors
description: MPL error hierarchy with structured error codes, serialization, and handling patterns
---

# Errors

The MPL SDK defines a structured error hierarchy for different failure modes. All errors extend `MplError` and include an error code, message, and JSON serialization for wire transport.

---

## Import

```typescript
import {
  MplError,
  SchemaFidelityError,
  QomBreachError,
  NegotiationError,
  UnknownStypeError,
  ConnectionError,
  HashMismatchError,
  PolicyDeniedError,
} from '@mpl/sdk';
```

---

## Error Hierarchy

```
Error (built-in)
└── MplError
    ├── SchemaFidelityError
    ├── QomBreachError
    ├── UnknownStypeError
    ├── NegotiationError
    ├── ConnectionError
    ├── HashMismatchError
    └── PolicyDeniedError
```

---

## Error Codes

Every `MplError` carries a string `code` for programmatic handling:

| Code | Error Class | Description |
|------|-------------|-------------|
| `E-SCHEMA-FIDELITY` | `SchemaFidelityError` | Payload does not match the SType schema |
| `E-QOM-BREACH` | `QomBreachError` | QoM profile thresholds not met |
| `E-UNKNOWN-STYPE` | `UnknownStypeError` | SType not found in registry |
| `E-NEGOTIATION-FAILED` | `NegotiationError` | AI-ALPN handshake rejected |
| `E-CONNECTION` | `ConnectionError` | WebSocket or HTTP connection failed |
| `E-HASH-MISMATCH` | `HashMismatchError` | Semantic hash verification failed |
| `E-POLICY-DENIED` | `PolicyDeniedError` | Policy engine denied the operation |

---

## MplError

Base class for all MPL errors. Extends the built-in `Error` with a structured code and JSON serialization.

```typescript
class MplError extends Error {
  readonly code: string;

  constructor(code: string, message: string);
  toJSON(): Record<string, unknown>;
}
```

| Property | Type | Description |
|----------|------|-------------|
| `code` | `string` | Machine-readable error code |
| `message` | `string` | Human-readable error description (inherited from Error) |
| `name` | `string` | Always `"MplError"` |

### toJSON()

Serialize the error for wire transport or logging.

```typescript
toJSON(): Record<string, unknown>
```

**Returns:**

```json
{
  "error": "E-SCHEMA-FIDELITY",
  "message": "Payload does not match schema for org.calendar.Event.v1"
}
```

### Example

```typescript
try {
  await client.call('calendar.create', { title: 123 });
} catch (error) {
  if (error instanceof MplError) {
    console.error('Code:', error.code);
    console.error('Message:', error.message);
    console.error('JSON:', JSON.stringify(error.toJSON()));
  }
}
```

---

## SchemaFidelityError

Thrown when a payload fails JSON Schema validation against its declared SType.

```typescript
class SchemaFidelityError extends MplError {
  readonly stype: string;
  readonly validationErrors: Array<{ path: string; message: string }>;

  constructor(
    stype: string,
    validationErrors: Array<{ path: string; message: string }>,
  );
  toJSON(): Record<string, unknown>;
}
```

| Property | Type | Description |
|----------|------|-------------|
| `code` | `string` | `"E-SCHEMA-FIDELITY"` |
| `stype` | `string` | The SType that the payload failed to match |
| `validationErrors` | `Array<{ path: string; message: string }>` | Detailed validation failures |

### toJSON() Output

```json
{
  "error": "E-SCHEMA-FIDELITY",
  "message": "Payload does not match schema for org.calendar.Event.v1",
  "stype": "org.calendar.Event.v1",
  "validation_errors": [
    { "path": "/title", "message": "must be string" },
    { "path": "/start", "message": "must match format \"date-time\"" }
  ]
}
```

### Example

```typescript
import { SchemaFidelityError } from '@mpl/sdk';

try {
  await session.send('org.calendar.Event.v1', {
    title: 123,           // Should be string
    start: 'not-a-date',  // Should be date-time format
  });
} catch (error) {
  if (error instanceof SchemaFidelityError) {
    console.error(`Schema mismatch for ${error.stype}:`);
    for (const ve of error.validationErrors) {
      console.error(`  ${ve.path}: ${ve.message}`);
    }
    // "Schema mismatch for org.calendar.Event.v1:"
    // "  /title: must be string"
    // "  /start: must match format "date-time""
  }
}
```

---

## QomBreachError

Thrown when QoM metrics do not meet the required profile thresholds.

```typescript
class QomBreachError extends MplError {
  readonly profile: string;
  readonly metrics: Record<string, number>;
  readonly failures: Array<{ metric: string; actual: number; threshold: number }>;

  constructor(
    profile: string,
    metrics: Record<string, number>,
    failures: Array<{ metric: string; actual: number; threshold: number }>,
  );
  toJSON(): Record<string, unknown>;
}
```

| Property | Type | Description |
|----------|------|-------------|
| `code` | `string` | `"E-QOM-BREACH"` |
| `profile` | `string` | Name of the profile that was not met |
| `metrics` | `Record<string, number>` | All measured metrics |
| `failures` | `Array<...>` | Metrics that fell below their thresholds |

### toJSON() Output

```json
{
  "error": "E-QOM-BREACH",
  "message": "QoM profile qom-strict-argcheck not met: instructionCompliance",
  "profile": "qom-strict-argcheck",
  "metrics": {
    "schemaFidelity": 1.0,
    "instructionCompliance": 0.72
  },
  "failures": [
    { "metric": "instructionCompliance", "actual": 0.72, "threshold": 0.95 }
  ]
}
```

### Example

```typescript
import { QomBreachError } from '@mpl/sdk';

try {
  const response = await processWithQom(payload);
} catch (error) {
  if (error instanceof QomBreachError) {
    console.error(`Profile "${error.profile}" not met`);
    for (const f of error.failures) {
      console.error(
        `  ${f.metric}: got ${f.actual}, needed >= ${f.threshold}`
      );
    }
  }
}
```

---

## NegotiationError

Thrown when the AI-ALPN handshake fails because client and server cannot agree on capabilities.

```typescript
class NegotiationError extends MplError {
  readonly clientStypes: string[];
  readonly serverStypes: string[];
  readonly reason?: string;

  constructor(
    clientStypes: string[],
    serverStypes: string[],
    reason?: string,
  );
  toJSON(): Record<string, unknown>;
}
```

| Property | Type | Description |
|----------|------|-------------|
| `code` | `string` | `"E-NEGOTIATION-FAILED"` |
| `clientStypes` | `string[]` | STypes offered by the client |
| `serverStypes` | `string[]` | STypes supported by the server |
| `reason` | `string \| undefined` | Server-provided reason for rejection |

### toJSON() Output

```json
{
  "error": "E-NEGOTIATION-FAILED",
  "message": "No common STypes between client and server",
  "client_stypes": ["org.calendar.Event.v1", "org.calendar.Invite.v1"],
  "server_stypes": ["org.finance.Transaction.v1"]
}
```

### Example

```typescript
import { NegotiationError } from '@mpl/sdk';

try {
  await session.connect();
} catch (error) {
  if (error instanceof NegotiationError) {
    console.error('Negotiation failed:', error.reason);
    console.error('We offered:', error.clientStypes);
    console.error('Server supports:', error.serverStypes);

    // Find what's missing
    const missing = error.clientStypes.filter(
      s => !error.serverStypes.includes(s)
    );
    console.error('Server missing:', missing);
  }
}
```

---

## UnknownStypeError

Thrown when an SType is referenced that does not exist in the registry.

```typescript
class UnknownStypeError extends MplError {
  readonly stype: string;

  constructor(stype: string);
  toJSON(): Record<string, unknown>;
}
```

| Property | Type | Description |
|----------|------|-------------|
| `code` | `string` | `"E-UNKNOWN-STYPE"` |
| `stype` | `string` | The unrecognized SType identifier |

### toJSON() Output

```json
{
  "error": "E-UNKNOWN-STYPE",
  "message": "Unknown SType: org.calendar.Event.v99",
  "stype": "org.calendar.Event.v99"
}
```

### Example

```typescript
import { UnknownStypeError } from '@mpl/sdk';

try {
  await client.send('org.calendar.Event.v99', payload);
} catch (error) {
  if (error instanceof UnknownStypeError) {
    console.error(`SType not found: ${error.stype}`);
    console.error('Check the registry or version number');
  }
}
```

---

## ConnectionError

Thrown when a WebSocket or HTTP connection cannot be established.

```typescript
class ConnectionError extends MplError {
  readonly endpoint: string;
  readonly cause?: string;

  constructor(endpoint: string, cause?: string);
  toJSON(): Record<string, unknown>;
}
```

| Property | Type | Description |
|----------|------|-------------|
| `code` | `string` | `"E-CONNECTION"` |
| `endpoint` | `string` | The URL that failed to connect |
| `cause` | `string \| undefined` | Underlying error message |

### toJSON() Output

```json
{
  "error": "E-CONNECTION",
  "message": "Failed to connect to ws://localhost:9443/ws",
  "endpoint": "ws://localhost:9443/ws",
  "cause": "ECONNREFUSED"
}
```

### Example

```typescript
import { ConnectionError } from '@mpl/sdk';

try {
  await session.connect();
} catch (error) {
  if (error instanceof ConnectionError) {
    console.error(`Cannot reach ${error.endpoint}`);
    if (error.cause) {
      console.error(`Reason: ${error.cause}`);
    }
    // Implement retry logic
    await retryWithBackoff(() => session.connect());
  }
}
```

---

## HashMismatchError

Thrown when verifying a semantic hash and the computed hash does not match the expected value.

```typescript
class HashMismatchError extends MplError {
  readonly expected: string;
  readonly actual: string;

  constructor(expected: string, actual: string);
  toJSON(): Record<string, unknown>;
}
```

| Property | Type | Description |
|----------|------|-------------|
| `code` | `string` | `"E-HASH-MISMATCH"` |
| `expected` | `string` | The hash that was expected |
| `actual` | `string` | The hash that was computed |

### toJSON() Output

```json
{
  "error": "E-HASH-MISMATCH",
  "message": "Semantic hash mismatch",
  "expected": "b3:a1b2c3d4e5f6...",
  "actual": "b3:f6e5d4c3b2a1..."
}
```

### Example

```typescript
import { HashMismatchError, verifyHash } from '@mpl/sdk';

function verifyEnvelopeIntegrity(envelope: MplEnvelope): void {
  if (envelope.semHash) {
    const isValid = verifyHash(envelope.payload, envelope.semHash);
    if (!isValid) {
      throw new HashMismatchError(
        envelope.semHash,
        semanticHash(envelope.payload),
      );
    }
  }
}

try {
  verifyEnvelopeIntegrity(receivedEnvelope);
} catch (error) {
  if (error instanceof HashMismatchError) {
    console.error('Payload was tampered with!');
    console.error(`Expected: ${error.expected}`);
    console.error(`Got: ${error.actual}`);
  }
}
```

---

## PolicyDeniedError

Thrown when the policy engine denies an operation.

```typescript
class PolicyDeniedError extends MplError {
  readonly policy: string;
  readonly reason: string;
  readonly remediation?: string;

  constructor(policy: string, reason: string, remediation?: string);
  toJSON(): Record<string, unknown>;
}
```

| Property | Type | Description |
|----------|------|-------------|
| `code` | `string` | `"E-POLICY-DENIED"` |
| `policy` | `string` | Name of the policy that blocked the operation |
| `reason` | `string` | Why the policy was triggered |
| `remediation` | `string \| undefined` | Suggested action to resolve the issue |

### toJSON() Output

```json
{
  "error": "E-POLICY-DENIED",
  "message": "Policy data-residency denied: Payload contains PII routed to non-EU region",
  "policy": "data-residency",
  "reason": "Payload contains PII routed to non-EU region",
  "remediation": "Use eu-west-1 endpoint for PII-containing payloads"
}
```

### Example

```typescript
import { PolicyDeniedError } from '@mpl/sdk';

try {
  await client.send('org.medical.Diagnosis.v1', sensitivePayload);
} catch (error) {
  if (error instanceof PolicyDeniedError) {
    console.error(`Policy "${error.policy}" denied this request`);
    console.error(`Reason: ${error.reason}`);
    if (error.remediation) {
      console.log(`Fix: ${error.remediation}`);
    }
  }
}
```

---

## Error Handling Patterns

### Comprehensive Error Handling

```typescript
import {
  MplError,
  SchemaFidelityError,
  QomBreachError,
  NegotiationError,
  ConnectionError,
  HashMismatchError,
  PolicyDeniedError,
} from '@mpl/sdk';

async function robustCall(session: Session, stype: string, payload: Record<string, unknown>) {
  try {
    return await session.send(stype, payload);
  } catch (error) {
    if (error instanceof SchemaFidelityError) {
      // Fix the payload and retry
      console.error('Validation errors:', error.validationErrors);
      throw error;

    } else if (error instanceof QomBreachError) {
      // Quality issue - log and potentially retry with different prompt
      console.warn('QoM breach:', error.failures);
      throw error;

    } else if (error instanceof NegotiationError) {
      // Incompatible server - cannot proceed
      console.error('Server incompatible:', error.reason);
      throw error;

    } else if (error instanceof ConnectionError) {
      // Network issue - retry with backoff
      console.warn('Connection lost, retrying...');
      await session.connect();
      return await session.send(stype, payload);

    } else if (error instanceof HashMismatchError) {
      // Integrity violation - do not trust the data
      console.error('Data integrity compromised!');
      throw error;

    } else if (error instanceof PolicyDeniedError) {
      // Policy blocked - check remediation
      console.error(`Policy ${error.policy}: ${error.remediation}`);
      throw error;

    } else if (error instanceof MplError) {
      // Unknown MPL error
      console.error('MPL error:', error.code, error.message);
      throw error;

    } else {
      // Non-MPL error (network timeout, etc.)
      throw error;
    }
  }
}
```

### Type Guard Pattern

```typescript
function isMplError(error: unknown): error is MplError {
  return error instanceof MplError;
}

function isRetryable(error: unknown): boolean {
  if (!isMplError(error)) return false;
  return error.code === 'E-CONNECTION';
}
```

### JSON Serialization for Logging

All errors support `toJSON()` for structured logging:

```typescript
try {
  await session.send(stype, payload);
} catch (error) {
  if (error instanceof MplError) {
    // Structured log entry
    logger.error({
      ...error.toJSON(),
      timestamp: new Date().toISOString(),
      requestId: currentRequestId,
    });
  }
}
```

---

## See Also

- [Client](client.md) - How errors are thrown in production mode
- [Session](session.md) - Errors during connection and messaging
- [Validation](validation.md) - Schema validation that produces SchemaFidelityError
- [QoM](qom.md) - Profile evaluation that produces QomBreachError
- [Hashing](hashing.md) - Hash verification that produces HashMismatchError
