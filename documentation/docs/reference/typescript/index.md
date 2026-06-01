---
title: TypeScript SDK
description: Complete reference for the MPL TypeScript SDK - type-safe AI agent communication with schema validation and QoM enforcement
---

# TypeScript SDK

The `@mpl/sdk` package provides a pure TypeScript implementation of the Meaning Protocol Layer, enabling type-safe communication between AI agents with built-in schema validation, semantic hashing, and Quality of Meaning (QoM) enforcement.

---

## Installation

```bash
npm install @mpl/sdk
```

!!! info "Requirements"
    - **Node.js** 18.0.0 or later
    - **TypeScript** 5.3+ (for full type inference support)

---

## Architecture

The SDK is built as a pure TypeScript library with minimal dependencies:

| Dependency | Version | Purpose |
|-----------|---------|---------|
| `ajv` | 8.12.0 | JSON Schema validation (draft 2020-12) |
| `ajv-formats` | 2.1.1 | Format validation (email, uri, date-time, etc.) |
| `ws` | 8.16.0 | WebSocket connections for session-based communication |

The build system uses **tsup** to produce three output formats:

| Format | File | Use Case |
|--------|------|----------|
| ESM | `dist/index.mjs` | Modern bundlers, Node.js with `"type": "module"` |
| CJS | `dist/index.js` | Legacy Node.js, CommonJS environments |
| Types | `dist/index.d.ts` | TypeScript type checking and IDE support |

---

## Two API Levels

The SDK exposes two levels of abstraction to accommodate different use cases:

### Simple API: Client

For the common case where you need to call tools through an MPL proxy with type safety:

```typescript
import { Client, Mode } from '@mpl/sdk';

const client = new Client('http://localhost:9443', {
  mode: Mode.Production,
});

// Type-safe tool calls with generics
interface CalendarEvent {
  id: string;
  title: string;
  start: string;
}

const result = await client.call<CalendarEvent>('calendar.create', {
  title: 'Team Standup',
  start: '2024-01-15T10:00:00Z',
});

console.log(result.data.id);    // TypeScript knows this is string
console.log(result.valid);       // Schema validation passed
console.log(result.qomPassed);   // QoM evaluation passed
```

### Advanced API: Session

For full control over connections, negotiation, validation, and QoM:

```typescript
import { Session, QomProfile, SchemaValidator } from '@mpl/sdk';

const session = new Session({
  endpoint: 'ws://localhost:9443/ws',
  stypes: ['org.calendar.Event.v1', 'org.calendar.Invite.v1'],
  qomProfile: 'qom-strict-argcheck',
  autoValidate: true,
  autoHash: true,
});

// Connect and negotiate capabilities
const capabilities = await session.connect();
console.log(capabilities.commonStypes);

// Send typed payloads
const response = await session.send('org.calendar.Event.v1', {
  title: 'Meeting',
  start: '2024-01-15T10:00:00Z',
});

// Listen for incoming messages
session.onMessage('org.calendar.Invite.v1', (envelope) => {
  console.log('Received invite:', envelope.payload);
});
```

---

## Module Structure

```
typescript/src/
├── index.ts                    # Public API exports
├── client.ts                   # Simple Client class
├── session/
│   └── session.ts              # Advanced Session class
├── types/
│   ├── stype.ts                # SType parsing and representation
│   ├── envelope.ts             # MplEnvelope message wrapper
│   └── qom.ts                  # QoM profiles and metrics
├── validation/
│   ├── schema-validator.ts     # AJV-based JSON Schema validation
│   └── hash.ts                 # Semantic hashing (BLAKE3/SHA-256)
└── errors/
    └── index.ts                # Error hierarchy
```

---

## Quick Start

### ESM Import

```typescript
import { Client, Mode, Session, SType, QomProfile } from '@mpl/sdk';
```

### CommonJS Require

```javascript
const { Client, Mode, Session, SType, QomProfile } = require('@mpl/sdk');
```

### Minimal Example

```typescript
import { Client } from '@mpl/sdk';

async function main() {
  const client = new Client('http://localhost:9443');

  // Check proxy health
  const health = await client.health();
  console.log('Proxy status:', health.status);

  // Discover capabilities
  const caps = await client.capabilities();
  console.log('Supported STypes:', caps.stypes);

  // Call a tool
  const result = await client.call('calendar.list', {
    from: '2024-01-01',
    to: '2024-12-31',
  });

  if (result.valid && result.qomPassed) {
    console.log('Events:', result.data);
  }
}

main().catch(console.error);
```

---

## TypeScript Generics

The SDK uses TypeScript generics throughout to provide type-safe payloads without runtime overhead:

```typescript
// Define your domain types
interface WeatherForecast {
  location: string;
  temperature: number;
  conditions: string;
  humidity: number;
}

// Generic parameter flows through to result.data
const result = await client.call<WeatherForecast>('weather.forecast', {
  location: 'San Francisco',
});

// Full type inference - no casting needed
const temp: number = result.data.temperature;
const conditions: string = result.data.conditions;
```

---

## Reference Pages

| Page | Description |
|------|-------------|
| [Client](client.md) | Simple client for tool calls and typed payloads |
| [Session](session.md) | Advanced session management with WebSocket and AI-ALPN |
| [Types](types.md) | Core type definitions: SType, MplEnvelope, Provenance |
| [Validation](validation.md) | AJV-based schema validation |
| [QoM](qom.md) | Quality of Meaning profiles and evaluation |
| [Errors](errors.md) | Error hierarchy and handling patterns |
| [Hashing](hashing.md) | Semantic hashing and payload canonicalization |

---

## Version

The current SDK version is exported as a constant:

```typescript
import { VERSION } from '@mpl/sdk';
console.log(VERSION); // "0.1.0"
```
