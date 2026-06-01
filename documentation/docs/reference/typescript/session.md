---
title: Session
description: Advanced session management with WebSocket connections, AI-ALPN negotiation, and bidirectional typed messaging
---

# Session

The `Session` class provides advanced, stateful communication over WebSocket with support for AI-ALPN handshake negotiation, automatic schema validation, semantic hashing, and QoM profile enforcement.

---

## Import

```typescript
import { Session, SessionConfig, NegotiatedCapabilities, SendOptions } from '@mpl/sdk';
```

---

## SessionConfig Interface

```typescript
interface SessionConfig {
  /** Server endpoint URL (ws:// or wss://) */
  endpoint: string;
  /** List of STypes this session supports */
  stypes?: string[];
  /** QoM profile to enforce */
  qomProfile?: string;
  /** Path to local registry for schema resolution */
  registryPath?: string;
  /** Request timeout in milliseconds. Default: 30000 */
  timeoutMs?: number;
  /** Auto-validate payloads against registered schemas. Default: true */
  autoValidate?: boolean;
  /** Auto-compute semantic hashes for outgoing payloads. Default: true */
  autoHash?: boolean;
}
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `endpoint` | `string` | *required* | WebSocket URL (`ws://` or `wss://`) |
| `stypes` | `string[]` | `[]` | STypes advertised during handshake |
| `qomProfile` | `string` | `undefined` | QoM profile name to negotiate and enforce |
| `registryPath` | `string` | `undefined` | Local filesystem path to SType registry |
| `timeoutMs` | `number` | `30000` | Timeout for connection, handshake, and requests |
| `autoValidate` | `boolean` | `true` | Validate outgoing payloads against registered schemas |
| `autoHash` | `boolean` | `true` | Compute semantic hashes for outgoing envelopes |

---

## NegotiatedCapabilities Interface

Result of the AI-ALPN handshake, describing the intersection of client and server capabilities.

```typescript
interface NegotiatedCapabilities {
  /** STypes supported by both client and server */
  commonStypes: string[];
  /** QoM profile selected for this session */
  selectedProfile?: string;
  /** Additional server extensions */
  serverExtensions: Record<string, unknown>;
}
```

| Property | Type | Description |
|----------|------|-------------|
| `commonStypes` | `string[]` | Intersection of client and server SType lists |
| `selectedProfile` | `string \| undefined` | The QoM profile agreed upon during handshake |
| `serverExtensions` | `Record<string, unknown>` | Server-specific extensions and features |

---

## SendOptions Interface

Per-message options that override session defaults.

```typescript
interface SendOptions {
  /** Override autoValidate for this message */
  validate?: boolean;
  /** Override autoHash for this message */
  computeHash?: boolean;
}
```

| Property | Type | Description |
|----------|------|-------------|
| `validate` | `boolean` | If set, overrides `SessionConfig.autoValidate` |
| `computeHash` | `boolean` | If set, overrides `SessionConfig.autoHash` |

---

## Session Class

### Constructor

```typescript
class Session {
  constructor(config: SessionConfig);
}
```

Creates a new session instance. Does not initiate a connection until `connect()` is called.

#### Example

```typescript
const session = new Session({
  endpoint: 'ws://localhost:9443/ws',
  stypes: [
    'org.calendar.Event.v1',
    'org.calendar.Invite.v1',
    'org.calendar.Reminder.v1',
  ],
  qomProfile: 'qom-strict-argcheck',
  timeoutMs: 15000,
  autoValidate: true,
  autoHash: true,
});
```

---

### connect()

Establish a WebSocket connection and perform the AI-ALPN handshake.

```typescript
async connect(): Promise<NegotiatedCapabilities>
```

**Returns:** `Promise<NegotiatedCapabilities>` - The negotiated capabilities.

**Throws:**

- `ConnectionError` - If the WebSocket connection fails
- `NegotiationError` - If the AI-ALPN handshake is rejected

#### AI-ALPN Handshake

The handshake follows the AI-ALPN protocol:

1. Client sends a `ai-alpn-hello` message with its STypes and QoM profiles
2. Server responds with the intersection of capabilities
3. Session stores the negotiated capabilities

```typescript
// Handshake message sent by the client:
{
  "type": "ai-alpn-hello",
  "version": "1.0",
  "stypes": ["org.calendar.Event.v1", "org.calendar.Invite.v1"],
  "qom_profiles": ["qom-strict-argcheck"]
}

// Server response:
{
  "type": "ai-alpn-hello-ack",
  "common_stypes": ["org.calendar.Event.v1"],
  "selected_profile": "qom-strict-argcheck",
  "extensions": { "streaming": true }
}
```

#### Example

```typescript
try {
  const capabilities = await session.connect();
  console.log('Common STypes:', capabilities.commonStypes);
  console.log('Profile:', capabilities.selectedProfile);
  console.log('Extensions:', capabilities.serverExtensions);
} catch (error) {
  if (error instanceof NegotiationError) {
    console.error('Negotiation failed:', error.reason);
    console.error('Client offered:', error.clientStypes);
    console.error('Server supports:', error.serverStypes);
  }
}
```

---

### send()

Send a typed payload to the server and wait for a response.

```typescript
async send(
  stype: string,
  payload: Record<string, unknown>,
  options?: SendOptions,
): Promise<MplEnvelope>
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `stype` | `string` | SType identifier for the payload |
| `payload` | `Record<string, unknown>` | The payload data |
| `options` | `SendOptions` | Per-message overrides |

**Returns:** `Promise<MplEnvelope>` - The server's response envelope.

**Throws:**

- `ConnectionError` - If not connected
- `SchemaFidelityError` - If validation is enabled and the payload fails schema check
- `Error` - If the request times out

#### Behavior

1. If `autoValidate` (or `options.validate`) is `true` and a schema is registered for the SType, the payload is validated before sending
2. If `autoHash` (or `options.computeHash`) is `true`, a semantic hash is computed and attached to the envelope
3. The envelope is sent over WebSocket and the method waits for a matching response (by envelope ID)
4. The response timeout is controlled by `SessionConfig.timeoutMs`

#### Examples

**Basic send:**

```typescript
const response = await session.send('org.calendar.Event.v1', {
  title: 'Team Standup',
  start: '2024-01-15T10:00:00Z',
  duration: 30,
});

console.log(response.id);       // UUID of the response
console.log(response.payload);  // Server's response data
console.log(response.semHash);  // Semantic hash of response
```

**With per-message options:**

```typescript
// Skip validation and hashing for this specific message
const response = await session.send(
  'org.calendar.Event.v1',
  { title: 'Draft Event' },
  { validate: false, computeHash: false },
);
```

**With error handling:**

```typescript
import { SchemaFidelityError, ConnectionError } from '@mpl/sdk';

try {
  const response = await session.send('org.calendar.Event.v1', {
    title: 123,  // Wrong type - should be string
  });
} catch (error) {
  if (error instanceof SchemaFidelityError) {
    console.error('Validation failed for:', error.stype);
    error.validationErrors.forEach(err => {
      console.error(`  ${err.path}: ${err.message}`);
    });
  } else if (error instanceof ConnectionError) {
    console.error('Not connected:', error.endpoint);
  }
}
```

---

### onMessage()

Register a handler for incoming messages of a specific SType.

```typescript
onMessage(stype: string, handler: (envelope: MplEnvelope) => void | Promise<void>): void
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `stype` | `string` | SType to listen for |
| `handler` | `(envelope: MplEnvelope) => void \| Promise<void>` | Callback invoked when a matching message arrives |

Handlers are stored per-SType. Registering a new handler for the same SType replaces the previous one.

#### Example

```typescript
// Listen for calendar invites
session.onMessage('org.calendar.Invite.v1', (envelope) => {
  console.log('New invite from:', envelope.provenance?.intent);
  console.log('Event:', envelope.payload.title);
  console.log('Hash:', envelope.semHash);
});

// Listen for reminders
session.onMessage('org.calendar.Reminder.v1', async (envelope) => {
  await processReminder(envelope.payload);
});
```

---

### registerSchema()

Register a JSON Schema for payload validation.

```typescript
registerSchema(stype: string, schema: object | string): void
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `stype` | `string` | SType identifier to associate with the schema |
| `schema` | `object \| string` | JSON Schema object or JSON string |

Schemas are used by `send()` when `autoValidate` is enabled.

#### Example

```typescript
session.registerSchema('org.calendar.Event.v1', {
  type: 'object',
  required: ['title', 'start'],
  properties: {
    title: { type: 'string', minLength: 1 },
    start: { type: 'string', format: 'date-time' },
    duration: { type: 'number', minimum: 1 },
    attendees: {
      type: 'array',
      items: { type: 'string', format: 'email' },
    },
  },
});

// Now send() will validate before sending
const response = await session.send('org.calendar.Event.v1', {
  title: '',  // Will throw SchemaFidelityError (minLength: 1)
});
```

---

### close()

Close the WebSocket connection and clean up resources.

```typescript
async close(): Promise<void>
```

After closing, `isConnected` returns `false` and any pending requests are not resolved.

#### Example

```typescript
try {
  await session.connect();
  // ... use session ...
} finally {
  await session.close();
}
```

---

### isConnected (getter)

Check if the session has an active connection.

```typescript
get isConnected(): boolean
```

**Returns:** `true` if connected and handshake completed, `false` otherwise.

---

### capabilities (getter)

Get the negotiated capabilities from the handshake.

```typescript
get capabilities(): NegotiatedCapabilities | undefined
```

**Returns:** The negotiated capabilities, or `undefined` if not yet connected.

---

## Complete Example

```typescript
import {
  Session,
  MplEnvelope,
  SchemaFidelityError,
  NegotiationError,
  ConnectionError,
} from '@mpl/sdk';

async function main() {
  const session = new Session({
    endpoint: 'wss://mpl.example.com/ws',
    stypes: [
      'org.calendar.Event.v1',
      'org.calendar.Invite.v1',
    ],
    qomProfile: 'qom-strict-argcheck',
    timeoutMs: 10000,
  });

  // Register schemas for validation
  session.registerSchema('org.calendar.Event.v1', {
    type: 'object',
    required: ['title', 'start'],
    properties: {
      title: { type: 'string', minLength: 1 },
      start: { type: 'string', format: 'date-time' },
    },
  });

  try {
    // Connect and negotiate
    const caps = await session.connect();
    console.log('Connected! Common STypes:', caps.commonStypes);

    // Set up message handlers
    session.onMessage('org.calendar.Invite.v1', (envelope) => {
      console.log('Received invite:', envelope.payload);
    });

    // Send a validated, hashed payload
    const response = await session.send('org.calendar.Event.v1', {
      title: 'Architecture Review',
      start: '2024-03-01T14:00:00Z',
    });

    console.log('Response:', response.payload);
    console.log('Verified hash:', response.semHash);

  } catch (error) {
    if (error instanceof NegotiationError) {
      console.error('Cannot negotiate:', error.reason);
    } else if (error instanceof SchemaFidelityError) {
      console.error('Invalid payload:', error.validationErrors);
    } else if (error instanceof ConnectionError) {
      console.error('Connection failed:', error.cause);
    }
  } finally {
    await session.close();
  }
}

main();
```

---

## Session vs Client

| Feature | Client | Session |
|---------|--------|---------|
| Transport | HTTP (fetch) | WebSocket |
| Connection | Stateless (per-request) | Stateful (persistent) |
| Negotiation | None | AI-ALPN handshake |
| Bidirectional | No | Yes (onMessage handlers) |
| Validation | Proxy-side only | Client-side + proxy-side |
| Hashing | Proxy-side only | Client-side (auto) |
| QoM Profiles | Header inspection only | Full enforcement |
| Use Case | Simple tool calls | Multi-message workflows |

---

## See Also

- [Client](client.md) - Simple stateless client
- [Types](types.md) - MplEnvelope and related types
- [Validation](validation.md) - Schema validation details
- [Hashing](hashing.md) - Semantic hash computation
- [Errors](errors.md) - Error types thrown by Session
