---
title: Python SDK
description: Reference documentation for the MPL Python SDK - typed, validated AI agent communication
---

# Python SDK

The MPL Python SDK provides typed, validated communication for AI agents. Built on a high-performance Rust core with Python bindings via PyO3, it offers both a simple client for quick integration and an advanced session API for full control.

---

## Installation

```bash
pip install mpl-sdk
```

### Requirements

| Requirement | Version |
|-------------|---------|
| Python | 3.10+ |
| aiohttp | >= 3.9.0 |
| websockets | >= 12.0 |

---

## Architecture

The SDK is a Python wrapper over a compiled Rust core, accessed through PyO3 bindings:

```
┌─────────────────────────────────────────────┐
│  Your Application                           │
├─────────────────────────────────────────────┤
│  Python API Layer                           │
│  (client.py, session.py, errors.py)         │
├─────────────────────────────────────────────┤
│  Rust Core (mpl_sdk._mpl_core)             │
│  SType, MplEnvelope, SchemaValidator,       │
│  QomProfile, canonicalize, semantic_hash    │
└─────────────────────────────────────────────┘
```

The Rust core handles all performance-critical operations: schema validation, semantic hashing (BLAKE3), JSON canonicalization, and QoM evaluation. The Python layer provides ergonomic async APIs for network communication and session management.

---

## Two API Levels

### Simple Client (Recommended)

For 80% of use cases. Provides a minimal, intuitive interface for calling tools through an MPL proxy.

```python
from mpl_sdk import Client, Mode

async with Client("http://localhost:9443") as client:
    result = await client.call("calendar.create", {
        "title": "Team Standup",
        "start": "2024-01-15T10:00:00Z",
        "duration_minutes": 15
    })
    print(result.data)       # Response payload
    print(result.valid)      # Schema validation passed
    print(result.qom_passed) # QoM profile met
```

### Advanced Session

For full control over validation, QoM profiles, WebSocket communication, and message routing.

```python
from mpl_sdk import Session, SessionConfig

config = SessionConfig(
    endpoint="ws://localhost:8080/mcp",
    stypes=["org.calendar.Event.v1", "org.agent.TaskPlan.v1"],
    qom_profile="qom-strict-argcheck",
    registry_path="./registry",
)

async with Session(config) as session:
    capabilities = session.capabilities
    print(f"Common STypes: {capabilities.common_stypes}")

    response = await session.send(
        stype="org.calendar.Event.v1",
        payload={"title": "Meeting", "start": "2024-01-15T10:00:00Z"}
    )
```

---

## Module Structure

| Module | Description | Key Exports |
|--------|-------------|-------------|
| [`client`](client.md) | Simple client API | `Client`, `Mode`, `CallResult`, `typed` |
| [`session`](session.md) | Advanced session management | `Session`, `SessionConfig`, `NegotiatedCapabilities` |
| [`types`](types.md) | Core type classes (Rust bindings) | `SType`, `MplEnvelope` |
| [`validation`](validation.md) | Schema validation | `SchemaValidator`, `ValidationResult` |
| [`qom`](qom.md) | Quality of Meaning evaluation | `QomMetrics`, `QomProfile`, `QomEvaluation` |
| [`errors`](errors.md) | Error hierarchy | `MplError`, `SchemaFidelityError`, `QomBreachError`, ... |
| [`hashing`](hashing.md) | Semantic hashing functions | `canonicalize`, `semantic_hash`, `verify_hash` |

---

## Quick Example

A complete example demonstrating the simple client with error handling:

```python
import asyncio
from mpl_sdk import Client, Mode, MplError, SchemaFidelityError

async def main():
    async with Client("http://localhost:9443", mode=Mode.PRODUCTION) as client:
        # Check server health
        health = await client.health()
        print(f"Server status: {health['status']}")

        # Discover capabilities
        caps = await client.capabilities()
        print(f"Supported STypes: {caps['stypes']}")

        try:
            # Make a typed tool call
            result = await client.call(
                "calendar.create",
                {
                    "title": "Architecture Review",
                    "start": "2024-03-01T14:00:00Z",
                    "duration_minutes": 60,
                    "attendees": ["alice@example.com", "bob@example.com"],
                },
                stype="org.calendar.Event.v1",
            )

            if result.valid and result.qom_passed:
                print(f"Event created: {result.data['id']}")
            else:
                print("Warning: response did not pass validation")

        except SchemaFidelityError as e:
            print(f"Schema validation failed for {e.stype}:")
            for error in e.validation_errors:
                print(f"  {error['path']}: {error['message']}")

        except MplError as e:
            print(f"MPL error [{e.code}]: {e.message}")

asyncio.run(main())
```

---

## Imports

All public symbols are available from the top-level package:

```python
# Simple API
from mpl_sdk import Client, Mode, CallResult, typed

# Advanced API
from mpl_sdk import Session, SessionConfig, NegotiatedCapabilities

# Core types (from Rust bindings)
from mpl_sdk import SType, MplEnvelope

# Validation
from mpl_sdk import SchemaValidator, ValidationResult

# QoM
from mpl_sdk import QomMetrics, QomProfile, QomEvaluation, MetricFailure

# Hashing functions
from mpl_sdk import canonicalize, semantic_hash, verify_hash

# Errors
from mpl_sdk import (
    MplError,
    SchemaFidelityError,
    QomBreachError,
    NegotiationError,
    UnknownStypeError,
)
```

---

## Next Steps

- [Client API](client.md) -- Get started with the simple client
- [Session API](session.md) -- Full session management for advanced use cases
- [Types](types.md) -- Core type system reference
- [Errors](errors.md) -- Error handling patterns
