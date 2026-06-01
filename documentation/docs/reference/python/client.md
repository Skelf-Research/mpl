---
title: Client
description: Reference for the simple MPL Client API - the recommended interface for 80% of use cases
---

# Client

The `Client` class provides a minimal, user-friendly interface for MPL. It handles HTTP communication with an MPL proxy, automatic schema validation feedback, and QoM evaluation results.

```python
from mpl_sdk import Client, Mode, CallResult, typed
```

---

## Client

### Constructor

```python
Client(
    endpoint: str,
    mode: Mode = Mode.DEVELOPMENT,
    timeout: float = 30.0,
)
```

Creates a new MPL client that communicates with an MPL proxy server.

#### Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `endpoint` | `str` | *(required)* | MPL proxy URL (e.g., `"http://localhost:9443"`) |
| `mode` | `Mode` | `Mode.DEVELOPMENT` | Operating mode controlling error behavior |
| `timeout` | `float` | `30.0` | Request timeout in seconds |

#### Example

```python
# Development mode (default) - logs errors, doesn't raise
client = Client("http://localhost:9443")

# Production mode - raises on validation failures
client = Client(
    "http://localhost:9443",
    mode=Mode.PRODUCTION,
    timeout=60.0,
)
```

---

### call()

```python
async def call(
    self,
    tool_or_stype: str | Type[T],
    arguments: dict,
    *,
    stype: str | None = None,
) -> CallResult
```

Call a tool through the MPL proxy. The request is wrapped in a JSON-RPC 2.0 envelope and sent to the proxy, which handles validation and QoM evaluation.

#### Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `tool_or_stype` | `str \| Type[T]` | *(required)* | Tool name (e.g., `"calendar.create"`) or a typed class with `_tool_name` attribute |
| `arguments` | `dict` | *(required)* | Tool arguments as a dictionary |
| `stype` | `str \| None` | `None` | Override SType for the request. Sent as `X-MPL-SType` header |

#### Returns

[`CallResult`](#callresult) containing the response data, validation status, and QoM result.

#### Raises

| Exception | Condition |
|-----------|-----------|
| [`MplError`](errors.md#mplerror) | Request failed (production mode only) |
| [`SchemaFidelityError`](errors.md#schemafidelityerror) | Validation failed (production mode only) |

#### Examples

**Basic tool call:**

```python
async with Client("http://localhost:9443") as client:
    result = await client.call("calendar.create", {
        "title": "Team Standup",
        "start": "2024-01-15T10:00:00Z",
        "duration_minutes": 15,
    })
    print(result.data)  # {"id": "evt-123", "title": "Team Standup", ...}
```

**With explicit SType:**

```python
result = await client.call(
    "calendar.create",
    {"title": "Meeting", "start": "2024-01-15T10:00:00Z"},
    stype="org.calendar.Event.v1",
)
print(result.stype)  # "org.calendar.Event.v1"
```

**Production mode with error handling:**

```python
client = Client("http://localhost:9443", mode=Mode.PRODUCTION)

try:
    result = await client.call("calendar.create", {
        "title": "Meeting",
        # Missing required 'start' field
    })
except SchemaFidelityError as e:
    print(f"Validation failed: {e.validation_errors}")
except MplError as e:
    print(f"Call failed: {e.code} - {e.message}")
```

---

### send()

```python
async def send(
    self,
    stype: str,
    payload: dict,
) -> CallResult
```

Send a typed payload directly without the JSON-RPC wrapper. Use this for non-tool payloads or direct MPL communication between agents.

#### Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `stype` | `str` | *(required)* | SType identifier (e.g., `"org.calendar.Event.v1"`) |
| `payload` | `dict` | *(required)* | The payload data matching the SType schema |

#### Returns

[`CallResult`](#callresult) with the response.

#### Raises

| Exception | Condition |
|-----------|-----------|
| [`MplError`](errors.md#mplerror) | Request failed |

#### Example

```python
async with Client("http://localhost:9443") as client:
    result = await client.send(
        stype="org.calendar.Event.v1",
        payload={
            "title": "Architecture Review",
            "start": "2024-03-01T14:00:00Z",
            "duration_minutes": 60,
            "attendees": ["alice@example.com"],
        },
    )
    print(result.valid)      # True
    print(result.qom_passed) # True
```

---

### health()

```python
async def health(self) -> dict
```

Check the MPL proxy health status.

#### Returns

Dictionary with health information from the proxy server.

#### Example

```python
async with Client("http://localhost:9443") as client:
    health = await client.health()
    print(health)
    # {"status": "healthy", "version": "0.1.0", "uptime_seconds": 3600}
```

---

### capabilities()

```python
async def capabilities(self) -> dict
```

Get the proxy's advertised capabilities including supported STypes, QoM profiles, and server extensions.

#### Returns

Dictionary with capability information.

#### Example

```python
async with Client("http://localhost:9443") as client:
    caps = await client.capabilities()
    print(caps["stypes"])    # ["org.calendar.Event.v1", ...]
    print(caps["profiles"])  # ["qom-basic", "qom-strict-argcheck"]
```

---

### close()

```python
async def close(self) -> None
```

Close the client and release all resources. This is called automatically when using the client as a context manager.

#### Example

```python
client = Client("http://localhost:9443")
try:
    result = await client.call("calendar.create", {"title": "Meeting"})
finally:
    await client.close()
```

---

### Context Manager

The client supports `async with` for automatic resource cleanup:

```python
async with Client("http://localhost:9443") as client:
    result = await client.call("calendar.create", {"title": "Meeting"})
# client.close() is called automatically
```

This is equivalent to:

```python
client = Client("http://localhost:9443")
try:
    await client._ensure_session()
    result = await client.call("calendar.create", {"title": "Meeting"})
finally:
    await client.close()
```

---

## Mode

```python
class Mode(Enum):
    DEVELOPMENT = "development"
    PRODUCTION = "production"
```

Controls how the client handles validation failures and errors.

| Mode | Behavior |
|------|----------|
| `DEVELOPMENT` | Log validation errors but do not raise exceptions. Returns `CallResult` with `valid=False` on failures. Suitable for development and testing. |
| `PRODUCTION` | Raise exceptions on validation failures. Ensures only valid responses are processed. Recommended for production deployments. |

### Example

```python
# Development: tolerant, logs issues
dev_client = Client("http://localhost:9443", mode=Mode.DEVELOPMENT)
result = await dev_client.call("tool", {"bad": "data"})
if not result.valid:
    print("Validation issue detected, but call continued")

# Production: strict, raises errors
prod_client = Client("http://localhost:9443", mode=Mode.PRODUCTION)
try:
    result = await prod_client.call("tool", {"bad": "data"})
except MplError as e:
    print(f"Blocked invalid call: {e}")
```

---

## CallResult

```python
@dataclass
class CallResult:
    data: Any
    stype: str | None = None
    valid: bool = True
    qom_passed: bool = True
```

Result from a tool call or send operation.

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `data` | `Any` | The response payload. In success cases, this is the tool result. On JSON-RPC errors (development mode), this contains the error object. |
| `stype` | `str \| None` | SType of the response, extracted from the `X-MPL-SType` response header. `None` if the server did not declare a type. |
| `valid` | `bool` | Whether schema validation passed. `True` by default; `False` if the proxy detected validation errors. |
| `qom_passed` | `bool` | Whether QoM evaluation passed. Derived from the `X-MPL-QoM-Result` response header. |

### Example

```python
result = await client.call("calendar.create", {"title": "Meeting"})

# Access response data
event_id = result.data["id"]

# Check validation status
if not result.valid:
    print("Warning: response did not match expected schema")

# Check QoM status
if not result.qom_passed:
    print("Warning: QoM profile thresholds not met")

# Check response SType
if result.stype:
    print(f"Response typed as: {result.stype}")
```

---

## typed()

```python
def typed(stype: str | None = None) -> Callable
```

Decorator to mark a function as typed with MPL. The decorated function will have its arguments validated against the specified SType schema when called through the MPL runtime.

#### Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `stype` | `str \| None` | `None` | SType identifier to associate with the function. If `None`, the SType is inferred from type hints. |

#### Examples

**With explicit SType:**

```python
@typed("org.calendar.Event.v1")
async def create_event(payload: dict) -> dict:
    return {"id": "event-123", **payload}
```

**Without parentheses (auto-infer from type hints):**

```python
@typed
async def create_event(event: dict) -> dict:
    return {"id": "event-123", **event}
```

**Using with Client:**

```python
@typed("org.calendar.Event.v1")
async def handle_calendar_event(payload: dict) -> dict:
    # This function's input/output will be validated against
    # the org.calendar.Event.v1 schema
    return {
        "id": generate_id(),
        "title": payload["title"],
        "start": payload["start"],
        "created_at": datetime.utcnow().isoformat(),
    }
```

---

## Complete Examples

### Basic Workflow

```python
import asyncio
from mpl_sdk import Client, Mode

async def main():
    async with Client("http://localhost:9443") as client:
        # Step 1: Verify connectivity
        health = await client.health()
        assert health["status"] == "healthy"

        # Step 2: Create an event
        result = await client.call("calendar.create", {
            "title": "Sprint Planning",
            "start": "2024-01-22T09:00:00Z",
            "duration_minutes": 60,
            "attendees": ["team@example.com"],
        })
        event_id = result.data["id"]

        # Step 3: Retrieve the event
        result = await client.call("calendar.get", {
            "id": event_id,
        })
        print(f"Event: {result.data['title']}")

asyncio.run(main())
```

### Production Error Handling

```python
import asyncio
import logging
from mpl_sdk import Client, Mode, MplError, SchemaFidelityError

logger = logging.getLogger(__name__)

async def create_event_safe(client: Client, event_data: dict) -> dict | None:
    """Create an event with comprehensive error handling."""
    try:
        result = await client.call(
            "calendar.create",
            event_data,
            stype="org.calendar.Event.v1",
        )

        if not result.qom_passed:
            logger.warning("QoM threshold not met for event creation")

        return result.data

    except SchemaFidelityError as e:
        logger.error(
            "Schema validation failed for %s: %s",
            e.stype,
            e.validation_errors,
        )
        return None

    except MplError as e:
        logger.error("MPL error [%s]: %s", e.code, e.message)
        return None

async def main():
    async with Client("http://localhost:9443", mode=Mode.PRODUCTION) as client:
        event = await create_event_safe(client, {
            "title": "Architecture Review",
            "start": "2024-03-01T14:00:00Z",
            "duration_minutes": 90,
        })
        if event:
            print(f"Created event: {event['id']}")

asyncio.run(main())
```

### Multiple Sequential Calls

```python
import asyncio
from mpl_sdk import Client

async def schedule_meeting_series(client: Client, series: list[dict]):
    """Schedule multiple meetings and collect results."""
    results = []
    for meeting in series:
        result = await client.call("calendar.create", meeting)
        if result.valid:
            results.append(result.data)
        else:
            print(f"Skipped invalid meeting: {meeting['title']}")
    return results

async def main():
    meetings = [
        {"title": "Monday Standup", "start": "2024-01-22T09:00:00Z", "duration_minutes": 15},
        {"title": "Tuesday Standup", "start": "2024-01-23T09:00:00Z", "duration_minutes": 15},
        {"title": "Wednesday Standup", "start": "2024-01-24T09:00:00Z", "duration_minutes": 15},
    ]

    async with Client("http://localhost:9443") as client:
        created = await schedule_meeting_series(client, meetings)
        print(f"Created {len(created)} meetings")

asyncio.run(main())
```
