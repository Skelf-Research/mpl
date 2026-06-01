# First Validation

This guide walks you through validating your first payload against an SType schema, understanding what happens on success and failure.

---

## What You'll Learn

- How SType schemas define message contracts
- How to validate a payload via the proxy and SDK
- How to interpret validation results and errors

---

## Prerequisites

- MPL proxy running (see [Quick Start](quick-start.md))
- A registry with SType definitions (the default registry ships with 25+ types)

---

## Step 1: Explore Available STypes

The MPL registry comes pre-seeded with STypes. Let's look at a simple one:

```bash
# List available STypes
mpl schemas list

# Show the calendar event schema
mpl schemas show org.calendar.Event.v1
```

The `org.calendar.Event.v1` schema looks like:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "Calendar Event",
  "type": "object",
  "required": ["title", "start", "end"],
  "properties": {
    "title": { "type": "string", "minLength": 1 },
    "start": { "type": "string", "format": "date-time" },
    "end": { "type": "string", "format": "date-time" },
    "timezone": { "type": "string" },
    "attendees": {
      "type": "array",
      "items": { "type": "string", "format": "email" }
    }
  },
  "additionalProperties": false
}
```

---

## Step 2: Validate a Correct Payload

Send a valid payload through the proxy:

=== "curl"

    ```bash
    curl -X POST http://localhost:9443/validate \
      -H "Content-Type: application/json" \
      -d '{
        "stype": "org.calendar.Event.v1",
        "payload": {
          "title": "Team Standup",
          "start": "2025-01-15T09:00:00Z",
          "end": "2025-01-15T09:30:00Z"
        }
      }'
    ```

=== "Python"

    ```python
    from mpl_sdk import Client

    client = Client("http://localhost:9443")
    result = await client.send(
        stype="org.calendar.Event.v1",
        payload={
            "title": "Team Standup",
            "start": "2025-01-15T09:00:00Z",
            "end": "2025-01-15T09:30:00Z"
        }
    )
    print(result.valid)      # True
    print(result.qom_passed) # True
    ```

=== "TypeScript"

    ```typescript
    import { MplClient } from '@mpl/sdk';

    const client = new MplClient('http://localhost:9443');
    const result = await client.validate({
      stype: 'org.calendar.Event.v1',
      payload: {
        title: 'Team Standup',
        start: '2025-01-15T09:00:00Z',
        end: '2025-01-15T09:30:00Z',
      }
    });
    console.log(result.valid);     // true
    console.log(result.qomPassed); // true
    ```

**Expected response:**

```json
{
  "valid": true,
  "stype": "org.calendar.Event.v1",
  "qom_report": {
    "meets_profile": true,
    "metrics": {
      "schema_fidelity": 1.0
    }
  },
  "sem_hash": "blake3:7f2a..."
}
```

---

## Step 3: Validate an Invalid Payload

Now send a payload with errors — missing the required `end` field:

=== "curl"

    ```bash
    curl -X POST http://localhost:9443/validate \
      -H "Content-Type: application/json" \
      -d '{
        "stype": "org.calendar.Event.v1",
        "payload": {
          "title": "Team Standup",
          "start": "2025-01-15T09:00:00Z"
        }
      }'
    ```

=== "Python"

    ```python
    from mpl_sdk import Client
    from mpl_sdk.errors import SchemaFidelityError

    client = Client("http://localhost:9443")
    try:
        result = await client.send(
            stype="org.calendar.Event.v1",
            payload={
                "title": "Team Standup",
                "start": "2025-01-15T09:00:00Z"
                # Missing "end" field!
            }
        )
    except SchemaFidelityError as e:
        print(e.code)              # "E-SCHEMA-FIDELITY"
        print(e.validation_errors) # [{"path": "", "message": "'end' is required"}]
    ```

**Expected error response:**

```json
{
  "valid": false,
  "error": {
    "code": "E-SCHEMA-FIDELITY",
    "message": "Payload failed schema validation",
    "details": {
      "stype": "org.calendar.Event.v1",
      "errors": [
        {
          "path": "",
          "message": "'end' is a required property"
        }
      ]
    }
  }
}
```

---

## Step 4: Understand the QoM Report

When validation passes, the response includes a QoM report based on the negotiated profile:

```json
{
  "qom_report": {
    "meets_profile": true,
    "profile": "qom-basic",
    "metrics": {
      "schema_fidelity": 1.0
    },
    "failures": []
  }
}
```

| Field | Meaning |
|-------|---------|
| `meets_profile` | Whether all metric thresholds are met |
| `profile` | The QoM profile evaluated against |
| `metrics` | Individual metric scores |
| `failures` | List of metrics that failed thresholds |

With a stricter profile (`qom-strict-argcheck`), you'd also see `instruction_compliance`:

```json
{
  "metrics": {
    "schema_fidelity": 1.0,
    "instruction_compliance": 0.98
  }
}
```

---

## Step 5: Validate with the CLI

You can also validate directly from the command line:

```bash
# Validate a JSON file
mpl validate --stype org.calendar.Event.v1 event.json

# Validate inline JSON
mpl validate --stype org.calendar.Event.v1 \
  '{"title": "Meeting", "start": "2025-01-15T10:00:00Z", "end": "2025-01-15T11:00:00Z"}'
```

---

## What You've Learned

1. STypes define schema contracts for messages
2. The proxy validates payloads against registered schemas
3. Valid payloads receive a QoM report with metric scores
4. Invalid payloads receive typed errors with specific failure details
5. Schema Fidelity (SF = 1.0) means the payload fully conforms

---

## Next Steps

- [Concepts: STypes](../concepts/stypes.md) — Deep dive into Semantic Types
- [Concepts: QoM](../concepts/qom.md) — Understanding quality metrics
- [Tutorial: Calendar Workflow](../guides/tutorials/calendar-workflow.md) — Full workflow example
