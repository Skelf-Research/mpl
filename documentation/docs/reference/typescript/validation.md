---
title: Validation
description: AJV-based JSON Schema validation for MPL payloads with support for draft 2020-12, format validation, and schema caching
---

# Validation

The validation module provides JSON Schema-based payload validation using [AJV](https://ajv.js.org/) (Another JSON Schema Validator). It supports JSON Schema draft 2020-12, format validation, schema caching, and integration with the SType registry.

---

## Import

```typescript
import {
  SchemaValidator,
  ValidationResult,
  ValidationError,
  SchemaValidationError,
} from '@mpl/sdk';
```

---

## ValidationError Interface

Describes a single validation failure.

```typescript
interface ValidationError {
  /** JSON Pointer path to the failing property */
  path: string;
  /** Human-readable error message */
  message: string;
  /** AJV validation keyword that failed (e.g., "type", "required", "format") */
  keyword?: string;
}
```

| Property | Type | Description |
|----------|------|-------------|
| `path` | `string` | JSON Pointer to the invalid value (e.g., `"/title"`, `"/attendees/0"`) |
| `message` | `string` | Description of what went wrong |
| `keyword` | `string \| undefined` | The JSON Schema keyword that triggered the error |

#### Common Keywords

| Keyword | Meaning | Example Error |
|---------|---------|---------------|
| `type` | Wrong data type | `"must be string"` |
| `required` | Missing required property | `"must have required property 'title'"` |
| `format` | Format validation failed | `"must match format \"date-time\""` |
| `minimum` | Number below minimum | `"must be >= 1"` |
| `maxLength` | String too long | `"must NOT have more than 100 characters"` |
| `enum` | Value not in allowed set | `"must be equal to one of the allowed values"` |
| `pattern` | Regex pattern not matched | `"must match pattern \"^[a-z]+$\""` |

---

## ValidationResult Interface

The result of validating a payload against a schema.

```typescript
interface ValidationResult {
  /** Whether the payload is valid against the schema */
  valid: boolean;
  /** List of validation errors (empty if valid) */
  errors: ValidationError[];
}
```

| Property | Type | Description |
|----------|------|-------------|
| `valid` | `boolean` | `true` if the payload passes all schema constraints |
| `errors` | `ValidationError[]` | Empty array when valid, detailed errors when invalid |

---

## SchemaValidator Class

Manages JSON Schema registration, compilation, and validation for SType payloads.

```typescript
class SchemaValidator {
  constructor();
  register(stype: string, schema: object | string): void;
  hasSchema(stype: string): boolean;
  validate(stype: string, payload: unknown): ValidationResult;
  validateOrThrow(stype: string, payload: unknown): void;
  registeredStypes(): string[];
}
```

---

### Constructor

```typescript
constructor()
```

Creates a new `SchemaValidator` with the following AJV configuration:

| Option | Value | Purpose |
|--------|-------|---------|
| `allErrors` | `true` | Report all errors, not just the first |
| `verbose` | `true` | Include data and schema in error objects |
| `strict` | `false` | Allow unknown keywords without errors |

The constructor also initializes `ajv-formats` for built-in format validation.

```typescript
const validator = new SchemaValidator();
```

---

### register()

Register a JSON Schema for an SType identifier.

```typescript
register(stype: string, schema: object | string): void
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `stype` | `string` | SType identifier to associate with this schema |
| `schema` | `object \| string` | JSON Schema object or JSON string |

The schema is compiled immediately upon registration. If the schema does not include a `$id` property, one is automatically set to `urn:stype:<stype>`.

#### Examples

**Register with object:**

```typescript
validator.register('org.calendar.Event.v1', {
  $schema: 'https://json-schema.org/draft/2020-12/schema',
  type: 'object',
  required: ['title', 'start'],
  properties: {
    title: {
      type: 'string',
      minLength: 1,
      maxLength: 200,
    },
    start: {
      type: 'string',
      format: 'date-time',
    },
    duration: {
      type: 'number',
      minimum: 1,
      description: 'Duration in minutes',
    },
    attendees: {
      type: 'array',
      items: { type: 'string', format: 'email' },
      maxItems: 100,
    },
    location: {
      type: 'string',
    },
    recurring: {
      type: 'boolean',
      default: false,
    },
  },
  additionalProperties: false,
});
```

**Register with JSON string:**

```typescript
import { readFileSync } from 'fs';

const schemaJson = readFileSync('schemas/event.v1.json', 'utf-8');
validator.register('org.calendar.Event.v1', schemaJson);
```

**Automatic $id assignment:**

```typescript
// Schema without $id - will be set to "urn:stype:org.calendar.Event.v1"
validator.register('org.calendar.Event.v1', {
  type: 'object',
  properties: { title: { type: 'string' } },
});
```

---

### hasSchema()

Check if a schema is registered for the given SType.

```typescript
hasSchema(stype: string): boolean
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `stype` | `string` | SType identifier to check |

**Returns:** `true` if a schema has been registered for this SType.

```typescript
if (validator.hasSchema('org.calendar.Event.v1')) {
  const result = validator.validate('org.calendar.Event.v1', payload);
}
```

---

### validate()

Validate a payload against the registered schema for an SType.

```typescript
validate(stype: string, payload: unknown): ValidationResult
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `stype` | `string` | SType whose schema to validate against |
| `payload` | `unknown` | The data to validate |

**Returns:** `ValidationResult` with `valid` flag and any errors.

**Throws:** `Error` if no schema is registered for the given SType.

#### Examples

**Valid payload:**

```typescript
const result = validator.validate('org.calendar.Event.v1', {
  title: 'Team Standup',
  start: '2024-01-15T10:00:00Z',
  duration: 30,
});

console.log(result.valid);   // true
console.log(result.errors);  // []
```

**Invalid payload:**

```typescript
const result = validator.validate('org.calendar.Event.v1', {
  title: '',           // minLength: 1
  start: 'not-a-date', // format: date-time
  duration: -5,         // minimum: 1
});

console.log(result.valid);  // false
console.log(result.errors);
// [
//   { path: '/title', message: 'must NOT have fewer than 1 characters', keyword: 'minLength' },
//   { path: '/start', message: 'must match format "date-time"', keyword: 'format' },
//   { path: '/duration', message: 'must be >= 1', keyword: 'minimum' },
// ]
```

**Missing required fields:**

```typescript
const result = validator.validate('org.calendar.Event.v1', {
  duration: 60,  // missing 'title' and 'start'
});

console.log(result.errors);
// [
//   { path: '/', message: "must have required property 'title'", keyword: 'required' },
//   { path: '/', message: "must have required property 'start'", keyword: 'required' },
// ]
```

---

### validateOrThrow()

Validate a payload and throw if invalid.

```typescript
validateOrThrow(stype: string, payload: unknown): void
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `stype` | `string` | SType whose schema to validate against |
| `payload` | `unknown` | The data to validate |

**Throws:** `SchemaValidationError` if validation fails.

```typescript
try {
  validator.validateOrThrow('org.calendar.Event.v1', {
    title: '',
    start: 'invalid',
  });
} catch (error) {
  if (error instanceof SchemaValidationError) {
    console.error('SType:', error.stype);
    console.error('Errors:', error.validationErrors);
  }
}
```

---

### registeredStypes()

Get all registered SType identifiers.

```typescript
registeredStypes(): string[]
```

**Returns:** Array of SType strings that have registered schemas.

```typescript
validator.register('org.calendar.Event.v1', eventSchema);
validator.register('org.calendar.Invite.v1', inviteSchema);

console.log(validator.registeredStypes());
// ["org.calendar.Event.v1", "org.calendar.Invite.v1"]
```

---

## SchemaValidationError Class

Thrown by `validateOrThrow()` when validation fails.

```typescript
class SchemaValidationError extends Error {
  readonly stype: string;
  readonly validationErrors: ValidationError[];

  constructor(stype: string, errors: ValidationError[]);
}
```

| Property | Type | Description |
|----------|------|-------------|
| `stype` | `string` | The SType that failed validation |
| `validationErrors` | `ValidationError[]` | Detailed list of failures |

The error message includes a formatted summary of all validation errors:

```typescript
try {
  validator.validateOrThrow('org.calendar.Event.v1', { title: 123 });
} catch (error) {
  console.error(error.message);
  // "Schema validation failed for org.calendar.Event.v1: /title: must be string"
}
```

---

## AJV Configuration

The validator uses AJV with the following setup:

### All Errors Mode

With `allErrors: true`, every constraint violation is reported. Without this, AJV would stop at the first error:

```typescript
// Reports ALL three errors, not just the first
const result = validator.validate('org.calendar.Event.v1', {
  title: 123,           // Error 1: wrong type
  start: 'not-a-date',  // Error 2: wrong format
  extra: 'field',       // Error 3: additional property
});
// result.errors.length === 3
```

### Format Validation

The `ajv-formats` plugin enables validation of standard string formats:

| Format | Example Valid Value |
|--------|-------------------|
| `date-time` | `"2024-01-15T10:00:00Z"` |
| `date` | `"2024-01-15"` |
| `time` | `"10:00:00Z"` |
| `email` | `"user@example.com"` |
| `uri` | `"https://example.com/path"` |
| `uuid` | `"550e8400-e29b-41d4-a716-446655440000"` |
| `ipv4` | `"192.168.1.1"` |
| `ipv6` | `"::1"` |

### Schema Caching

Schemas are compiled once during `register()` and the compiled validator function is cached. Subsequent `validate()` calls reuse the compiled function without re-parsing:

```typescript
// Compilation happens here (once)
validator.register('org.calendar.Event.v1', largeSchema);

// These use the cached compiled validator (fast)
validator.validate('org.calendar.Event.v1', payload1);
validator.validate('org.calendar.Event.v1', payload2);
validator.validate('org.calendar.Event.v1', payload3);
```

---

## Integration with Session

The `Session` class uses `SchemaValidator` internally for automatic payload validation:

```typescript
import { Session } from '@mpl/sdk';

const session = new Session({
  endpoint: 'ws://localhost:9443/ws',
  stypes: ['org.calendar.Event.v1'],
  autoValidate: true,  // Enables automatic validation
});

// Register schema on the session
session.registerSchema('org.calendar.Event.v1', {
  type: 'object',
  required: ['title', 'start'],
  properties: {
    title: { type: 'string' },
    start: { type: 'string', format: 'date-time' },
  },
});

await session.connect();

// This will throw SchemaFidelityError before sending
await session.send('org.calendar.Event.v1', {
  title: 123,  // Wrong type
});
```

---

## JSON Schema Draft 2020-12

The validator supports JSON Schema draft 2020-12 features:

```typescript
validator.register('org.medical.Diagnosis.v1', {
  $schema: 'https://json-schema.org/draft/2020-12/schema',
  type: 'object',
  required: ['condition', 'confidence'],
  properties: {
    condition: {
      type: 'string',
      minLength: 1,
    },
    confidence: {
      type: 'number',
      minimum: 0,
      maximum: 1,
    },
    evidence: {
      type: 'array',
      items: { type: 'string' },
      prefixItems: [
        { type: 'string', minLength: 10 },  // First item must be detailed
      ],
    },
  },
});
```

---

## See Also

- [Types](types.md) - MplEnvelope and payload structures
- [Session](session.md) - Automatic validation in sessions
- [Errors](errors.md) - SchemaFidelityError thrown by the proxy
- [QoM](qom.md) - Schema Fidelity as a QoM metric
