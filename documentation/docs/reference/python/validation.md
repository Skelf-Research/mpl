---
title: Validation
description: Reference for MPL schema validation - registering schemas, validating payloads, and handling validation errors
---

# Validation

Schema validation classes provided by the Rust core. These implement JSON Schema (draft 2020-12) validation for SType payloads, ensuring Schema Fidelity -- that every message conforms to its declared type contract.

```python
from mpl_sdk import SchemaValidator, ValidationResult
```

---

## SchemaValidator

```python
class SchemaValidator:
    def __init__(self): ...
```

A schema registry and validator. Stores JSON Schema definitions for STypes and validates payloads against them. Backed by the Rust `jsonschema` crate for high-performance validation.

### Constructor

```python
SchemaValidator()
```

Create a new, empty schema validator. Schemas must be registered before validation can occur.

```python
from mpl_sdk import SchemaValidator

validator = SchemaValidator()
```

---

### register()

```python
def register(self, stype: str, schema_json: str) -> None
```

Register a JSON Schema for an SType. The schema is compiled and cached for fast repeated validation.

#### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `stype` | `str` | SType identifier (e.g., `"org.calendar.Event.v1"`) |
| `schema_json` | `str` | JSON Schema definition as a JSON string |

#### Raises

| Exception | Condition |
|-----------|-----------|
| `SchemaError` | Invalid JSON Schema definition |

#### Example

```python
import json
from mpl_sdk import SchemaValidator

validator = SchemaValidator()

schema = {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "type": "object",
    "properties": {
        "title": {"type": "string", "minLength": 1},
        "start": {"type": "string", "format": "date-time"},
        "duration_minutes": {"type": "integer", "minimum": 1},
        "attendees": {
            "type": "array",
            "items": {"type": "string", "format": "email"},
        },
    },
    "required": ["title", "start"],
}

validator.register("org.calendar.Event.v1", json.dumps(schema))
```

---

### validate()

```python
def validate(self, stype: str, payload_json: str) -> ValidationResult
```

Validate a JSON payload against the registered schema for the given SType. Returns a result object with validation status and any errors.

#### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `stype` | `str` | SType identifier to validate against |
| `payload_json` | `str` | JSON string of the payload to validate |

#### Returns

[`ValidationResult`](#validationresult) containing the validation outcome.

#### Raises

| Exception | Condition |
|-----------|-----------|
| `ValueError` | No schema registered for the given SType |

#### Example

```python
import json
from mpl_sdk import SchemaValidator

validator = SchemaValidator()
validator.register("org.calendar.Event.v1", json.dumps(schema))

# Valid payload
result = validator.validate(
    "org.calendar.Event.v1",
    json.dumps({"title": "Meeting", "start": "2024-01-15T10:00:00Z"}),
)
print(result.valid)   # True
print(result.errors)  # []

# Invalid payload (missing required field)
result = validator.validate(
    "org.calendar.Event.v1",
    json.dumps({"duration_minutes": 30}),
)
print(result.valid)   # False
print(result.errors)  # [{"path": "", "message": "'title' is a required property"}, ...]
```

---

### validate_or_raise()

```python
def validate_or_raise(self, stype: str, payload_json: str) -> None
```

Validate a payload and raise an exception if validation fails. A convenience method that combines `validate()` with automatic error raising.

#### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `stype` | `str` | SType identifier to validate against |
| `payload_json` | `str` | JSON string of the payload to validate |

#### Raises

| Exception | Condition |
|-----------|-----------|
| [`SchemaFidelityError`](errors.md#schemafidelityerror) | Payload does not match the schema |
| `ValueError` | No schema registered for the given SType |

#### Example

```python
from mpl_sdk import SchemaValidator
from mpl_sdk.errors import SchemaFidelityError

validator = SchemaValidator()
validator.register("org.calendar.Event.v1", json.dumps(schema))

try:
    validator.validate_or_raise(
        "org.calendar.Event.v1",
        json.dumps({"title": 123}),  # title should be string
    )
except SchemaFidelityError as e:
    print(f"Validation failed for {e.stype}")
    for error in e.validation_errors:
        print(f"  {error['path']}: {error['message']}")
```

---

## ValidationResult

```python
@dataclass
class ValidationResult:
    valid: bool
    errors: list[dict]
```

The outcome of a schema validation check.

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `valid` | `bool` | `True` if the payload matches the schema, `False` otherwise |
| `errors` | `list[dict]` | List of validation error objects. Empty when `valid` is `True`. |

### Error Object Format

Each error in the `errors` list is a dictionary with the following keys:

| Key | Type | Description |
|-----|------|-------------|
| `path` | `str` | JSON Pointer path to the failing field (e.g., `"/attendees/0"`, `""` for root) |
| `message` | `str` | Human-readable description of the validation failure |

### Example

```python
result = validator.validate("org.calendar.Event.v1", payload_json)

if result.valid:
    print("Payload is valid!")
else:
    print(f"Found {len(result.errors)} validation errors:")
    for error in result.errors:
        path = error["path"] or "(root)"
        print(f"  [{path}] {error['message']}")
```

---

## Complete Examples

### Loading Schemas from Registry Files

```python
import json
import os
from mpl_sdk import SchemaValidator, SType

def load_registry(registry_path: str, stypes: list[str]) -> SchemaValidator:
    """Load schemas from a local registry directory."""
    validator = SchemaValidator()

    for stype_id in stypes:
        stype = SType(stype_id)
        schema_file = os.path.join(
            registry_path,
            stype.registry_path(),
            "schema.json",
        )

        if not os.path.exists(schema_file):
            raise FileNotFoundError(
                f"Schema not found for {stype_id}: {schema_file}"
            )

        with open(schema_file) as f:
            schema_json = f.read()

        validator.register(stype_id, schema_json)
        print(f"Registered schema: {stype_id}")

    return validator

# Usage
validator = load_registry("./registry", [
    "org.calendar.Event.v1",
    "org.agent.TaskPlan.v1",
    "org.agent.TaskResult.v1",
])
```

### Validating Request and Response

```python
import json
from mpl_sdk import SchemaValidator
from mpl_sdk.errors import SchemaFidelityError

validator = SchemaValidator()
validator.register("org.calendar.Event.v1", json.dumps(event_schema))
validator.register("org.calendar.EventResponse.v1", json.dumps(response_schema))

async def validated_call(client, tool: str, args: dict, stype: str) -> dict:
    """Make a tool call with request and response validation."""
    payload_json = json.dumps(args)

    # Validate request
    try:
        validator.validate_or_raise(stype, payload_json)
    except SchemaFidelityError as e:
        raise ValueError(
            f"Invalid request for {stype}: {e.validation_errors}"
        )

    # Make the call
    result = await client.call(tool, args, stype=stype)

    # Validate response
    if result.stype:
        response_result = validator.validate(
            result.stype,
            json.dumps(result.data),
        )
        if not response_result.valid:
            print(f"Warning: response validation errors: {response_result.errors}")

    return result.data
```

### Batch Validation

```python
import json
from mpl_sdk import SchemaValidator

validator = SchemaValidator()
validator.register("org.calendar.Event.v1", json.dumps(schema))

events = [
    {"title": "Meeting A", "start": "2024-01-15T10:00:00Z"},
    {"title": "", "start": "2024-01-15T11:00:00Z"},      # Empty title
    {"title": "Meeting C"},                                # Missing start
    {"title": "Meeting D", "start": "not-a-date"},         # Invalid format
]

valid_events = []
for i, event in enumerate(events):
    result = validator.validate(
        "org.calendar.Event.v1",
        json.dumps(event),
    )
    if result.valid:
        valid_events.append(event)
    else:
        print(f"Event {i} invalid:")
        for error in result.errors:
            print(f"  {error['path'] or '(root)'}: {error['message']}")

print(f"\n{len(valid_events)}/{len(events)} events passed validation")
```

### Custom Validation Logic

```python
import json
from mpl_sdk import SchemaValidator, ValidationResult

class EnhancedValidator:
    """Validator with custom business rules on top of schema validation."""

    def __init__(self):
        self._schema_validator = SchemaValidator()
        self._custom_rules: dict[str, list] = {}

    def register(self, stype: str, schema_json: str):
        self._schema_validator.register(stype, schema_json)

    def add_rule(self, stype: str, rule_fn):
        """Add a custom validation rule for an SType."""
        self._custom_rules.setdefault(stype, []).append(rule_fn)

    def validate(self, stype: str, payload_json: str) -> ValidationResult:
        # Run schema validation first
        result = self._schema_validator.validate(stype, payload_json)
        if not result.valid:
            return result

        # Run custom rules
        payload = json.loads(payload_json)
        custom_errors = []
        for rule_fn in self._custom_rules.get(stype, []):
            error = rule_fn(payload)
            if error:
                custom_errors.append(error)

        if custom_errors:
            return ValidationResult(valid=False, errors=custom_errors)

        return result

# Usage
validator = EnhancedValidator()
validator.register("org.calendar.Event.v1", json.dumps(schema))

# Add business rule: events must be in the future
def must_be_future(payload):
    from datetime import datetime, timezone
    start = datetime.fromisoformat(payload.get("start", ""))
    if start < datetime.now(timezone.utc):
        return {"path": "/start", "message": "Event start must be in the future"}
    return None

validator.add_rule("org.calendar.Event.v1", must_be_future)
```
