---
title: Errors
description: Reference for MPL error hierarchy - exception classes, error codes, and handling patterns
---

# Errors

The MPL SDK uses a structured exception hierarchy with unique error codes. All exceptions inherit from `MplError` and provide consistent access to error codes, messages, and detailed context.

```python
from mpl_sdk import (
    MplError,
    SchemaFidelityError,
    QomBreachError,
    NegotiationError,
    UnknownStypeError,
)
from mpl_sdk.errors import (
    HashMismatchError,
    ConnectionError,
    TimeoutError,
)
```

---

## Error Code Reference

| Code | Exception | Description |
|------|-----------|-------------|
| `E-SCHEMA-FIDELITY` | `SchemaFidelityError` | Payload does not conform to the declared SType schema |
| `E-QOM-BREACH` | `QomBreachError` | QoM metric fell below the profile threshold |
| `E-HANDSHAKE-FAILED` | `NegotiationError` | AI-ALPN capability negotiation failed |
| `E-UNKNOWN-STYPE` | `UnknownStypeError` | SType not found in the registry |
| `E-HASH-MISMATCH` | `HashMismatchError` | Semantic hash verification failed (payload tampered) |
| `E-CONNECTION-FAILED` | `ConnectionError` | Connection to MCP/A2A server failed |
| `E-TIMEOUT` | `TimeoutError` | Operation exceeded timeout |
| `E-MPL-UNKNOWN` | `MplError` | Generic/unclassified MPL error |

---

## MplError

```python
class MplError(Exception):
    code: str
    message: str
    details: dict
```

Base exception for all MPL errors. All other MPL exceptions inherit from this class.

### Constructor

```python
MplError(
    message: str,
    code: str | None = None,
    details: dict | None = None,
)
```

#### Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `message` | `str` | *(required)* | Human-readable error message |
| `code` | `str \| None` | `"E-MPL-UNKNOWN"` | Structured error code |
| `details` | `dict \| None` | `{}` | Additional context about the error |

### Attributes

| Attribute | Type | Description |
|-----------|------|-------------|
| `code` | `str` | Structured error code (e.g., `"E-SCHEMA-FIDELITY"`) |
| `message` | `str` | Human-readable error description |
| `details` | `dict` | Additional error context |

### Methods

#### to_dict()

```python
def to_dict(self) -> dict
```

Serialize the error to a dictionary for logging or transmission.

```python
try:
    ...
except MplError as e:
    error_data = e.to_dict()
    # {"code": "E-SCHEMA-FIDELITY", "message": "...", "details": {...}}
```

#### \_\_str\_\_()

Returns a formatted string: `"[E-SCHEMA-FIDELITY] Payload does not match schema"`

---

## SchemaFidelityError

```python
class SchemaFidelityError(MplError):
    stype: str
    validation_errors: list[dict]
```

Raised when a payload fails schema validation against its declared SType. Error code: `E-SCHEMA-FIDELITY`.

### Constructor

```python
SchemaFidelityError(
    message: str,
    stype: str,
    validation_errors: list[dict] | None = None,
)
```

### Attributes

| Attribute | Type | Description |
|-----------|------|-------------|
| `stype` | `str` | The SType that validation failed against |
| `validation_errors` | `list[dict]` | List of `{"path": "...", "message": "..."}` objects |

### Example

```python
from mpl_sdk.errors import SchemaFidelityError

try:
    result = await client.call("calendar.create", {
        "title": 123,  # Should be string
        # Missing required 'start' field
    })
except SchemaFidelityError as e:
    print(f"SType: {e.stype}")
    print(f"Code: {e.code}")  # "E-SCHEMA-FIDELITY"

    for error in e.validation_errors:
        print(f"  {error['path']}: {error['message']}")
    # Output:
    #   /title: 123 is not of type 'string'
    #   : 'start' is a required property
```

---

## QomBreachError

```python
class QomBreachError(MplError):
    metric: str
    expected: float
    actual: float
    profile: str | None
```

Raised when a QoM metric falls below the threshold defined by the active profile. Error code: `E-QOM-BREACH`.

### Constructor

```python
QomBreachError(
    message: str,
    metric: str,
    expected: float,
    actual: float,
    profile: str | None = None,
)
```

### Attributes

| Attribute | Type | Description |
|-----------|------|-------------|
| `metric` | `str` | Name of the failing metric (e.g., `"instruction_compliance"`) |
| `expected` | `float` | Required threshold value |
| `actual` | `float` | Actual measured value |
| `profile` | `str \| None` | Name of the QoM profile that was violated |

### Example

```python
from mpl_sdk.errors import QomBreachError

try:
    response = await session.send("org.calendar.Event.v1", payload)
except QomBreachError as e:
    print(f"QoM breach in profile '{e.profile}'")
    print(f"  Metric: {e.metric}")
    print(f"  Expected: >= {e.expected}")
    print(f"  Actual:   {e.actual}")
    # Output:
    #   QoM breach in profile 'qom-strict-argcheck'
    #     Metric: instruction_compliance
    #     Expected: >= 0.97
    #     Actual:   0.85
```

---

## NegotiationError

```python
class NegotiationError(MplError):
    client_stypes: list[str]
    server_stypes: list[str]
    reason: str | None
```

Raised when AI-ALPN handshake fails due to incompatible capabilities. Error code: `E-HANDSHAKE-FAILED`.

### Constructor

```python
NegotiationError(
    message: str,
    client_stypes: list[str] | None = None,
    server_stypes: list[str] | None = None,
    reason: str | None = None,
)
```

### Attributes

| Attribute | Type | Description |
|-----------|------|-------------|
| `client_stypes` | `list[str]` | STypes offered by the client |
| `server_stypes` | `list[str]` | STypes supported by the server |
| `reason` | `str \| None` | Server-provided reason for negotiation failure |

### Example

```python
from mpl_sdk.errors import NegotiationError

try:
    async with Session(config) as session:
        ...
except NegotiationError as e:
    print(f"Negotiation failed: {e.reason}")
    print(f"  Client offers: {e.client_stypes}")
    print(f"  Server supports: {e.server_stypes}")

    # Find what's missing
    client_set = set(e.client_stypes)
    server_set = set(e.server_stypes)
    missing = client_set - server_set
    print(f"  Server missing: {missing}")
```

---

## UnknownStypeError

```python
class UnknownStypeError(MplError):
    stype: str
    registry_path: str | None
```

Raised when an SType is not found in the local registry. Error code: `E-UNKNOWN-STYPE`.

### Constructor

```python
UnknownStypeError(
    stype: str,
    registry_path: str | None = None,
)
```

### Attributes

| Attribute | Type | Description |
|-----------|------|-------------|
| `stype` | `str` | The SType identifier that was not found |
| `registry_path` | `str \| None` | Path to the registry that was searched |

### Example

```python
from mpl_sdk.errors import UnknownStypeError

try:
    response = await session.send("org.custom.Widget.v1", payload)
except UnknownStypeError as e:
    print(f"SType not found: {e.stype}")
    print(f"Registry searched: {e.registry_path}")
    # Suggest adding the schema
    from mpl_sdk import SType
    parsed = SType(e.stype)
    print(f"Expected at: {e.registry_path}/{parsed.registry_path()}/schema.json")
```

---

## HashMismatchError

```python
class HashMismatchError(MplError):
    expected: str
    actual: str
```

Raised when semantic hash verification fails, indicating the payload has been modified in transit. Error code: `E-HASH-MISMATCH`.

### Constructor

```python
HashMismatchError(
    expected: str,
    actual: str,
    stype: str | None = None,
)
```

### Attributes

| Attribute | Type | Description |
|-----------|------|-------------|
| `expected` | `str` | The hash that was declared (e.g., `"blake3:a1b2..."`) |
| `actual` | `str` | The hash computed from the received payload |

### Example

```python
from mpl_sdk.errors import HashMismatchError

try:
    envelope = receive_envelope()
    if not envelope.verify_hash():
        raise HashMismatchError(
            expected=envelope.sem_hash,
            actual=envelope.compute_hash(),
        )
except HashMismatchError as e:
    print(f"Payload integrity check failed!")
    print(f"  Expected: {e.expected[:24]}...")
    print(f"  Actual:   {e.actual[:24]}...")
    # Reject the message - possible tampering
```

---

## ConnectionError

```python
class ConnectionError(MplError):
    endpoint: str
    cause: str | None
```

Raised when connection to an MCP/A2A server fails. Error code: `E-CONNECTION-FAILED`.

### Constructor

```python
ConnectionError(
    message: str,
    endpoint: str,
    cause: str | None = None,
)
```

### Attributes

| Attribute | Type | Description |
|-----------|------|-------------|
| `endpoint` | `str` | The endpoint URL that failed to connect |
| `cause` | `str \| None` | Underlying cause of the connection failure |

### Example

```python
from mpl_sdk.errors import ConnectionError

try:
    async with Session(config) as session:
        ...
except ConnectionError as e:
    print(f"Cannot connect to {e.endpoint}")
    print(f"Cause: {e.cause}")
    # Implement retry logic
```

---

## TimeoutError

```python
class TimeoutError(MplError):
    timeout_ms: int
    operation: str | None
```

Raised when an operation exceeds its configured timeout. Error code: `E-TIMEOUT`.

### Constructor

```python
TimeoutError(
    message: str,
    timeout_ms: int,
    operation: str | None = None,
)
```

### Attributes

| Attribute | Type | Description |
|-----------|------|-------------|
| `timeout_ms` | `int` | The timeout value in milliseconds that was exceeded |
| `operation` | `str \| None` | Description of the operation that timed out |

### Example

```python
from mpl_sdk.errors import TimeoutError

try:
    result = await client.call("slow.operation", {"data": large_payload})
except TimeoutError as e:
    print(f"Operation '{e.operation}' timed out after {e.timeout_ms}ms")
    # Consider increasing timeout or breaking into smaller operations
```

---

## Error Handling Patterns

### Comprehensive Try/Except

```python
from mpl_sdk import Client, Mode, MplError
from mpl_sdk.errors import (
    SchemaFidelityError,
    QomBreachError,
    ConnectionError,
    TimeoutError,
)

async def resilient_call(client: Client, tool: str, args: dict) -> dict | None:
    """Make a tool call with comprehensive error handling."""
    try:
        result = await client.call(tool, args)
        return result.data

    except SchemaFidelityError as e:
        # Payload doesn't match schema - fix the data
        logger.error("Schema error for %s: %s", e.stype, e.validation_errors)
        return None

    except QomBreachError as e:
        # Quality threshold not met - may want to retry
        logger.warning(
            "QoM breach: %s = %.2f (need %.2f)",
            e.metric, e.actual, e.expected,
        )
        return None

    except TimeoutError as e:
        # Operation too slow - consider retry with longer timeout
        logger.warning("Timeout after %dms on %s", e.timeout_ms, e.operation)
        return None

    except ConnectionError as e:
        # Server unreachable - check infrastructure
        logger.error("Cannot reach %s: %s", e.endpoint, e.cause)
        return None

    except MplError as e:
        # Catch-all for any other MPL error
        logger.error("MPL error [%s]: %s", e.code, e.message)
        return None
```

### Retry with Backoff

```python
import asyncio
from mpl_sdk import Client, MplError
from mpl_sdk.errors import ConnectionError, TimeoutError

async def call_with_retry(
    client: Client,
    tool: str,
    args: dict,
    max_retries: int = 3,
    base_delay: float = 1.0,
) -> dict:
    """Call with exponential backoff retry on transient errors."""
    last_error = None

    for attempt in range(max_retries):
        try:
            result = await client.call(tool, args)
            return result.data

        except (ConnectionError, TimeoutError) as e:
            last_error = e
            delay = base_delay * (2 ** attempt)
            logger.warning(
                "Attempt %d/%d failed (%s), retrying in %.1fs",
                attempt + 1, max_retries, e.code, delay,
            )
            await asyncio.sleep(delay)

        except MplError:
            # Non-retryable errors - raise immediately
            raise

    raise last_error
```

### Error Logging and Monitoring

```python
import json
import logging
from mpl_sdk import MplError

logger = logging.getLogger("mpl")

def log_mpl_error(e: MplError, context: dict | None = None):
    """Structured logging for MPL errors."""
    log_data = {
        **e.to_dict(),
        "context": context or {},
    }
    logger.error("MPL error: %s", json.dumps(log_data))

# Usage
try:
    result = await client.call("calendar.create", event_data)
except MplError as e:
    log_mpl_error(e, context={
        "tool": "calendar.create",
        "user_id": current_user.id,
    })
    raise
```

### Development vs Production Error Handling

```python
from mpl_sdk import Client, Mode, CallResult

async def handle_result(result: CallResult, mode: Mode) -> dict:
    """Handle call results differently based on mode."""
    if mode == Mode.DEVELOPMENT:
        # Log issues but continue
        if not result.valid:
            print(f"DEV WARNING: Validation failed, data may be incorrect")
        if not result.qom_passed:
            print(f"DEV WARNING: QoM threshold not met")
        return result.data

    else:
        # Production: strict checking
        if not result.valid:
            raise ValueError("Invalid response in production mode")
        if not result.qom_passed:
            raise ValueError("QoM requirements not met in production")
        return result.data
```
