---
title: Session
description: Reference for the advanced MPL Session API - full control over validation, QoM, and message routing
---

# Session

The `Session` class provides advanced MPL session management for typed communication with MCP/A2A servers. It handles WebSocket and HTTP connections, AI-ALPN capability negotiation, automatic schema validation, semantic hashing, and message routing.

```python
from mpl_sdk import Session, SessionConfig, NegotiatedCapabilities
```

---

## SessionConfig

```python
@dataclass
class SessionConfig:
    endpoint: str
    stypes: list[str] = field(default_factory=list)
    qom_profile: str | None = None
    registry_path: str | None = None
    timeout_ms: int = 30000
    auto_validate: bool = True
    auto_hash: bool = True
```

Configuration for an MPL session. Passed to the `Session` constructor to define connection parameters and behavior.

### Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `endpoint` | `str` | *(required)* | Server endpoint URL. Supports `ws://`, `wss://` for WebSocket or `http://`, `https://` for HTTP |
| `stypes` | `list[str]` | `[]` | List of SType identifiers this client supports. Used during AI-ALPN negotiation |
| `qom_profile` | `str \| None` | `None` | QoM profile name to enforce (e.g., `"qom-basic"`, `"qom-strict-argcheck"`) |
| `registry_path` | `str \| None` | `None` | Path to local SType registry directory. Defaults to `./registry` |
| `timeout_ms` | `int` | `30000` | Request timeout in milliseconds |
| `auto_validate` | `bool` | `True` | Automatically validate payloads against registered schemas |
| `auto_hash` | `bool` | `True` | Automatically compute BLAKE3 semantic hashes for payloads |

### Example

```python
config = SessionConfig(
    endpoint="ws://localhost:8080/mcp",
    stypes=[
        "org.calendar.Event.v1",
        "org.agent.TaskPlan.v1",
        "org.agent.TaskResult.v1",
    ],
    qom_profile="qom-strict-argcheck",
    registry_path="./my-registry",
    timeout_ms=60000,
    auto_validate=True,
    auto_hash=True,
)
```

---

## NegotiatedCapabilities

```python
@dataclass
class NegotiatedCapabilities:
    common_stypes: list[str]
    selected_profile: str | None
    server_extensions: dict[str, Any] = field(default_factory=dict)
```

Result of the AI-ALPN handshake negotiation between client and server.

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `common_stypes` | `list[str]` | STypes supported by both client and server. Only these types can be used for communication. |
| `selected_profile` | `str \| None` | The QoM profile agreed upon by both parties. `None` if no profile was negotiated. |
| `server_extensions` | `dict[str, Any]` | Additional capabilities advertised by the server (e.g., batch support, streaming). |

### Example

```python
async with Session(config) as session:
    caps = session.capabilities
    print(f"Common STypes: {caps.common_stypes}")
    print(f"Profile: {caps.selected_profile}")
    print(f"Extensions: {caps.server_extensions}")
```

---

## Session

### Constructor

```python
Session(config: SessionConfig)
```

Create a new MPL session. The session is not connected until `connect()` is called or the context manager is entered.

#### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `config` | `SessionConfig` | Session configuration |

---

### connect()

```python
async def connect(self) -> NegotiatedCapabilities
```

Establish connection and perform the AI-ALPN handshake. Automatically selects WebSocket or HTTP transport based on the endpoint URL scheme.

#### Returns

[`NegotiatedCapabilities`](#negotiatedcapabilities) containing the intersection of client and server capabilities.

#### Raises

| Exception | Condition |
|-----------|-----------|
| [`ConnectionError`](errors.md#connectionerror) | Connection to the server failed |
| [`NegotiationError`](errors.md#negotiationerror) | AI-ALPN handshake failed (incompatible capabilities) |

#### Handshake Flow

```
Client                          Server
  │                               │
  │──── ai-alpn-hello ──────────▶│
  │     {stypes, qom_profiles}   │
  │                               │
  │◀─── ai-alpn-response ────────│
  │     {common_stypes,           │
  │      selected_profile,        │
  │      extensions}              │
  │                               │
```

#### Example

```python
session = Session(config)
try:
    capabilities = await session.connect()
    if "org.calendar.Event.v1" in capabilities.common_stypes:
        print("Calendar events supported!")
finally:
    await session.close()
```

---

### send()

```python
async def send(
    self,
    stype: str,
    payload: dict,
    validate: bool | None = None,
    compute_hash: bool | None = None,
) -> MplEnvelope
```

Send a typed payload to the server. Optionally validates the payload against the SType schema and computes a semantic hash before sending.

#### Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `stype` | `str` | *(required)* | SType identifier (e.g., `"org.calendar.Event.v1"`) |
| `payload` | `dict` | *(required)* | The payload data matching the SType schema |
| `validate` | `bool \| None` | `None` | Override `auto_validate` setting for this call. `None` uses the session config value. |
| `compute_hash` | `bool \| None` | `None` | Override `auto_hash` setting for this call. `None` uses the session config value. |

#### Returns

[`MplEnvelope`](types.md#mplenvelope) containing the server response.

#### Raises

| Exception | Condition |
|-----------|-----------|
| [`SchemaFidelityError`](errors.md#schemafidelityerror) | Payload fails schema validation (when validation is enabled) |
| [`ConnectionError`](errors.md#connectionerror) | Session is not connected |

#### Example

```python
async with Session(config) as session:
    response = await session.send(
        stype="org.calendar.Event.v1",
        payload={
            "title": "Architecture Review",
            "start": "2024-03-01T14:00:00Z",
            "duration_minutes": 60,
            "attendees": ["alice@example.com", "bob@example.com"],
        },
    )

    print(f"Response SType: {response.stype}")
    print(f"Payload: {response.get_payload()}")
    print(f"Hash: {response.sem_hash}")
```

**Skip validation for a single call:**

```python
response = await session.send(
    stype="org.calendar.Event.v1",
    payload=raw_data,
    validate=False,    # Skip schema check
    compute_hash=True, # Still compute hash
)
```

---

### listen()

```python
async def listen(self) -> None
```

Start listening for incoming messages on a WebSocket connection. Dispatches received messages to handlers registered with `on_message()`. Runs indefinitely until the connection is closed.

!!! warning "WebSocket Only"
    This method requires a WebSocket connection. It will raise `ConnectionError` if the session was established over HTTP.

#### Raises

| Exception | Condition |
|-----------|-----------|
| [`ConnectionError`](errors.md#connectionerror) | Session is not using WebSocket transport |

#### Example

```python
async with Session(config) as session:
    @session.on_message("org.agent.TaskPlan.v1")
    async def handle_task(envelope: MplEnvelope):
        plan = envelope.get_payload()
        print(f"Received task: {plan['title']}")

    # This blocks until the connection is closed
    await session.listen()
```

---

### on_message()

```python
def on_message(self, stype: str) -> Callable
```

Decorator to register a handler for incoming messages of a specific SType. Multiple handlers can be registered for different STypes.

#### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `stype` | `str` | SType identifier to listen for |

#### Example

```python
session = Session(config)

@session.on_message("org.calendar.Event.v1")
async def handle_event(envelope: MplEnvelope):
    event = envelope.get_payload()
    print(f"Calendar event: {event['title']}")

@session.on_message("org.agent.TaskPlan.v1")
async def handle_task(envelope: MplEnvelope):
    plan = envelope.get_payload()
    print(f"Task plan: {plan['objective']}")

await session.connect()
await session.listen()
```

---

### close()

```python
async def close(self) -> None
```

Close the session and release all resources. Closes both WebSocket and HTTP connections. Safe to call multiple times.

#### Example

```python
session = Session(config)
await session.connect()
try:
    response = await session.send("org.calendar.Event.v1", {...})
finally:
    await session.close()
```

---

### Properties

#### is_connected

```python
@property
def is_connected(self) -> bool
```

Returns `True` if the session is currently connected and the AI-ALPN handshake completed successfully.

#### capabilities

```python
@property
def capabilities(self) -> NegotiatedCapabilities | None
```

Returns the negotiated capabilities, or `None` if not yet connected.

---

### Context Manager

The session supports `async with` for automatic connection and cleanup:

```python
async with Session(config) as session:
    # session.connect() was called automatically
    response = await session.send("org.calendar.Event.v1", {...})
# session.close() is called automatically
```

---

## Complete Examples

### WebSocket Session with Message Handlers

```python
import asyncio
from mpl_sdk import Session, SessionConfig, MplEnvelope

async def main():
    config = SessionConfig(
        endpoint="ws://localhost:8080/mcp",
        stypes=[
            "org.calendar.Event.v1",
            "org.agent.TaskPlan.v1",
            "org.agent.TaskResult.v1",
        ],
        qom_profile="qom-strict-argcheck",
        registry_path="./registry",
    )

    session = Session(config)

    @session.on_message("org.agent.TaskPlan.v1")
    async def handle_task_plan(envelope: MplEnvelope):
        plan = envelope.get_payload()
        print(f"Received task plan: {plan['objective']}")

        # Respond with task result
        await session.send(
            stype="org.agent.TaskResult.v1",
            payload={
                "plan_id": plan["id"],
                "status": "completed",
                "output": {"summary": "Task completed successfully"},
            },
        )

    @session.on_message("org.calendar.Event.v1")
    async def handle_calendar_event(envelope: MplEnvelope):
        event = envelope.get_payload()
        print(f"Calendar event: {event['title']} at {event['start']}")

    async with session:
        print(f"Connected! Common STypes: {session.capabilities.common_stypes}")
        await session.listen()

asyncio.run(main())
```

### HTTP Session with Validation

```python
import asyncio
from mpl_sdk import Session, SessionConfig
from mpl_sdk.errors import SchemaFidelityError, NegotiationError

async def main():
    config = SessionConfig(
        endpoint="http://localhost:8080/mcp",
        stypes=["org.calendar.Event.v1"],
        qom_profile="qom-basic",
        registry_path="./registry",
        auto_validate=True,
        auto_hash=True,
    )

    try:
        async with Session(config) as session:
            # Check what was negotiated
            caps = session.capabilities
            print(f"Profile: {caps.selected_profile}")
            print(f"Common STypes: {caps.common_stypes}")

            # Send validated payload
            response = await session.send(
                stype="org.calendar.Event.v1",
                payload={
                    "title": "Team Sync",
                    "start": "2024-01-15T10:00:00Z",
                    "duration_minutes": 30,
                },
            )

            print(f"Response hash: {response.sem_hash}")
            print(f"Response payload: {response.get_payload()}")

    except NegotiationError as e:
        print(f"Handshake failed: {e.reason}")
        print(f"  Client STypes: {e.client_stypes}")
        print(f"  Server STypes: {e.server_stypes}")

    except SchemaFidelityError as e:
        print(f"Validation failed for {e.stype}:")
        for err in e.validation_errors:
            print(f"  {err['path']}: {err['message']}")

asyncio.run(main())
```

### Multiple SType Listeners with Graceful Shutdown

```python
import asyncio
import signal
from mpl_sdk import Session, SessionConfig, MplEnvelope

async def main():
    config = SessionConfig(
        endpoint="ws://localhost:8080/mcp",
        stypes=[
            "org.calendar.Event.v1",
            "org.agent.TaskPlan.v1",
            "org.agent.TaskResult.v1",
            "org.agent.Heartbeat.v1",
        ],
        qom_profile="qom-basic",
    )

    session = Session(config)
    shutdown_event = asyncio.Event()

    @session.on_message("org.agent.Heartbeat.v1")
    async def handle_heartbeat(envelope: MplEnvelope):
        # Respond to server heartbeats
        await session.send("org.agent.Heartbeat.v1", {"status": "alive"})

    @session.on_message("org.agent.TaskPlan.v1")
    async def handle_task(envelope: MplEnvelope):
        plan = envelope.get_payload()
        # Process the task...
        await session.send("org.agent.TaskResult.v1", {
            "plan_id": plan["id"],
            "status": "completed",
        })

    # Graceful shutdown on SIGINT
    loop = asyncio.get_event_loop()
    loop.add_signal_handler(signal.SIGINT, shutdown_event.set)

    async with session:
        print("Session connected, listening for messages...")
        listen_task = asyncio.create_task(session.listen())

        await shutdown_event.wait()
        print("Shutting down...")
        listen_task.cancel()

asyncio.run(main())
```
