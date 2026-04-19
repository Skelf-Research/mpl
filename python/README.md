# MPL SDK for Python

Python SDK for the Meaning Protocol Layer (MPL), enabling semantic type safety for AI agent communication.

## Installation

```bash
pip install mpl-sdk
```

## Quick Start

```python
import asyncio
from mpl_sdk import Session, SessionConfig, MplEnvelope

async def main():
    # Create session with typed communication
    config = SessionConfig(
        endpoint="ws://localhost:8080/mcp",
        stypes=["org.calendar.Event.v1", "org.agent.TaskPlan.v1"],
        qom_profile="qom-basic",
    )

    async with Session(config) as session:
        # Send typed payload
        response = await session.send(
            stype="org.calendar.Event.v1",
            payload={
                "title": "Team Meeting",
                "start": "2024-01-15T10:00:00Z",
                "end": "2024-01-15T11:00:00Z",
                "attendees": ["alice@example.com"]
            }
        )
        print(f"Response: {response.payload}")

asyncio.run(main())
```

## Features

### Semantic Types (STypes)

Parse and work with semantic type identifiers:

```python
from mpl_sdk import SType

stype = SType.parse("org.calendar.Event.v1")
print(stype.namespace)  # "org"
print(stype.domain)     # "calendar"
print(stype.name)       # "Event"
print(stype.version)    # 1
```

### Schema Validation

Validate payloads against JSON schemas:

```python
from mpl_sdk import SchemaValidator

schema = {
    "type": "object",
    "properties": {
        "title": {"type": "string"},
        "start": {"type": "string", "format": "date-time"}
    },
    "required": ["title", "start"]
}

validator = SchemaValidator(schema)
result = validator.validate({"title": "Meeting", "start": "2024-01-15T10:00:00Z"})
print(result.is_valid)  # True
```

### Semantic Hashing

Compute deterministic hashes for payloads:

```python
from mpl_sdk import canonicalize, semantic_hash, verify_hash

payload = {"b": 2, "a": 1}  # Key order doesn't matter
canonical = canonicalize(payload)
hash_value = semantic_hash(canonical)

# Verify integrity
assert verify_hash(canonical, hash_value)
```

### QoM Profiles

Evaluate Quality of Meaning metrics:

```python
from mpl_sdk import QomProfile, QomMetrics, QomEvaluation

profile = QomProfile(
    name="strict",
    metrics={"schema_fidelity": {"min": 1.0}}
)

metrics = QomMetrics(schema_fidelity=1.0, instruction_compliance=0.95)
evaluation = profile.evaluate(metrics)
print(evaluation.passed)  # True
```

## Error Handling

```python
from mpl_sdk import (
    MplError,
    SchemaFidelityError,
    QomBreachError,
    NegotiationError,
    UnknownStypeError,
)

try:
    result = validator.validate(invalid_payload)
except SchemaFidelityError as e:
    print(f"Validation failed: {e.validation_errors}")
except QomBreachError as e:
    print(f"QoM breach: {e.metric} = {e.actual}, expected >= {e.expected}")
```

## License

Apache-2.0
