---
title: Client
description: Simple MPL client for type-safe tool calls and typed payload communication
---

# Client

The `Client` class provides a simple, high-level interface for calling tools through an MPL proxy. It handles JSON-RPC framing, timeout management, QoM header inspection, and TypeScript generics for type-safe responses.

---

## Import

```typescript
import { Client, Mode, ClientOptions, CallResult, typed } from '@mpl/sdk';
```

---

## Mode Enum

Operating mode that controls error handling behavior.

```typescript
enum Mode {
  /** Log validation errors but don't fail requests. */
  Development = 'development',
  /** Enforce validation and fail on errors. */
  Production = 'production',
}
```

| Value | Behavior |
|-------|----------|
| `Mode.Development` | Returns errors in `result.data` without throwing. Default mode. |
| `Mode.Production` | Throws `MplError` on JSON-RPC errors or validation failures. |

---

## ClientOptions Interface

```typescript
interface ClientOptions {
  /** Operating mode (development or production). Default: 'development' */
  mode?: Mode;
  /** Request timeout in milliseconds. Default: 30000 */
  timeout?: number;
  /** Custom headers to include in all requests. */
  headers?: Record<string, string>;
}
```

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `mode` | `Mode` | `Mode.Development` | Controls whether errors throw or return |
| `timeout` | `number` | `30000` | Request timeout in milliseconds |
| `headers` | `Record<string, string>` | `{}` | Custom headers sent with every request |

---

## CallResult Interface

```typescript
interface CallResult<T = unknown> {
  /** The response payload, typed via generic parameter. */
  data: T;
  /** SType of the response, if returned by the proxy. */
  stype?: string;
  /** Whether schema validation passed. */
  valid: boolean;
  /** Whether QoM evaluation passed. */
  qomPassed: boolean;
}
```

| Property | Type | Description |
|----------|------|-------------|
| `data` | `T` | Response payload. Type is inferred from the generic parameter. |
| `stype` | `string \| undefined` | SType identifier from the `X-MPL-SType` response header |
| `valid` | `boolean` | `true` if the response was valid (HTTP 2xx, no JSON-RPC error) |
| `qomPassed` | `boolean` | `true` if the `X-MPL-QoM-Result` header was `"pass"` |

---

## Client Class

### Constructor

```typescript
class Client {
  constructor(endpoint: string, options?: ClientOptions);
}
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `endpoint` | `string` | MPL proxy URL (e.g., `"http://localhost:9443"`) |
| `options` | `ClientOptions` | Optional configuration |

The constructor normalizes the endpoint by stripping trailing slashes.

---

### call()

Call a tool through the MPL proxy using JSON-RPC 2.0 framing.

```typescript
async call<T = unknown>(
  tool: string,
  args: Record<string, unknown>,
  stype?: string,
): Promise<CallResult<T>>
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `tool` | `string` | Tool name (e.g., `"calendar.create"`) |
| `args` | `Record<string, unknown>` | Tool arguments |
| `stype` | `string` | Optional SType sent as `X-MPL-SType` header |

**Returns:** `Promise<CallResult<T>>` - The typed result.

**Throws:** `MplError` in production mode if the tool call fails.

#### JSON-RPC Request Format

The client wraps calls in standard JSON-RPC 2.0:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "calendar.create",
    "arguments": { "title": "Meeting" }
  }
}
```

#### Examples

**Basic call:**

```typescript
const client = new Client('http://localhost:9443');
const result = await client.call('calendar.create', {
  title: 'Team Standup',
  start: '2024-01-15T10:00:00Z',
});
console.log(result.data);
```

**With TypeScript generics:**

```typescript
interface CalendarEvent {
  id: string;
  title: string;
  start: string;
  attendees: string[];
}

const result = await client.call<CalendarEvent>('calendar.create', {
  title: 'Team Standup',
  start: '2024-01-15T10:00:00Z',
  attendees: ['alice@example.com'],
});

// Full type safety - TypeScript knows the shape of result.data
console.log(result.data.id);         // string
console.log(result.data.attendees);  // string[]
```

**With explicit SType:**

```typescript
const result = await client.call<CalendarEvent>(
  'calendar.create',
  { title: 'Meeting', start: '2024-01-15T10:00:00Z' },
  'org.calendar.Event.v1',
);
```

---

### send()

Send a typed payload directly without JSON-RPC wrapping. Use this for non-tool payloads or direct MPL communication.

```typescript
async send<T = unknown>(
  stype: string,
  payload: Record<string, unknown>,
): Promise<CallResult<T>>
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `stype` | `string` | SType identifier (e.g., `"org.calendar.Event.v1"`) |
| `payload` | `Record<string, unknown>` | The payload data |

**Returns:** `Promise<CallResult<T>>` - The typed result.

**Throws:** `MplError` if the request fails.

The payload is sent as a raw POST to the `/mcp` endpoint with the SType declared in the `X-MPL-SType` header.

#### Example

```typescript
interface Notification {
  recipientId: string;
  message: string;
  priority: 'low' | 'medium' | 'high';
}

const result = await client.send<Notification>('com.acme.Notification.v1', {
  recipientId: 'user-456',
  message: 'Your report is ready',
  priority: 'medium',
});

console.log(result.valid);     // true if HTTP 2xx
console.log(result.qomPassed); // true if QoM check passed
```

---

### health()

Check the proxy health status.

```typescript
async health(): Promise<{ status: string; version: string }>
```

**Returns:** Health status object with `status` and `version` fields.

#### Example

```typescript
const health = await client.health();
console.log(health.status);   // "ok"
console.log(health.version);  // "0.1.0"
```

---

### capabilities()

Discover proxy capabilities including supported STypes and QoM profiles.

```typescript
async capabilities(): Promise<{
  version: string;
  stypes: string[];
  profiles: string[];
  capabilities: Record<string, boolean>;
}>
```

**Returns:** Object describing the proxy's supported features.

#### Example

```typescript
const caps = await client.capabilities();
console.log(caps.stypes);
// ["org.calendar.Event.v1", "org.calendar.Invite.v1", ...]

console.log(caps.profiles);
// ["qom-basic", "qom-strict-argcheck", "qom-outcome"]

console.log(caps.capabilities);
// { validation: true, hashing: true, provenance: true }
```

---

## Production Mode

In production mode, the client throws errors instead of returning them:

```typescript
import { Client, Mode, MplError } from '@mpl/sdk';

const client = new Client('http://localhost:9443', {
  mode: Mode.Production,
  timeout: 10000,
});

try {
  const result = await client.call<CalendarEvent>('calendar.create', {
    title: 'Meeting',
  });
  // Only reaches here if call succeeded
  console.log(result.data);
} catch (error) {
  if (error instanceof MplError) {
    console.error('MPL error:', error.message);
    console.error('Code:', error.code);
  }
}
```

!!! warning "Development vs Production"
    In **development mode** (default), JSON-RPC errors are returned in `result.data` with `result.valid = false`. This allows inspection without interrupting flow.

    In **production mode**, any JSON-RPC error throws an `MplError`, ensuring failures are never silently ignored.

---

## Custom Headers

Use custom headers for authentication, tracing, or tenant identification:

```typescript
const client = new Client('http://localhost:9443', {
  headers: {
    'Authorization': 'Bearer eyJhbGciOiJSUzI1NiIs...',
    'X-Tenant-ID': 'acme-corp',
    'X-Request-Trace': 'trace-789',
  },
});
```

---

## Timeout Handling

Requests that exceed the timeout are aborted via `AbortController`:

```typescript
const client = new Client('http://localhost:9443', {
  timeout: 5000,  // 5 second timeout
});

try {
  const result = await client.call('slow.operation', { data: '...' });
} catch (error) {
  // MplError with message "Request failed: AbortError: ..."
  console.error(error.message);
}
```

---

## @typed Decorator

The `typed` decorator marks methods as MPL-typed for runtime metadata:

```typescript
import { typed } from '@mpl/sdk';

class CalendarService {
  @typed('org.calendar.Event.v1')
  async createEvent(payload: CalendarEvent): Promise<CalendarEvent> {
    return { id: 'event-123', ...payload };
  }
}
```

!!! note "Decorator Behavior"
    The `@typed` decorator attaches an `_mplStype` property to the method for middleware inspection. Actual validation happens at the proxy level, not in the decorator itself.

---

## See Also

- [Session](session.md) - Advanced session-based communication with WebSocket
- [Errors](errors.md) - Error types thrown by the client
- [Types](types.md) - Core type definitions used in responses
