---
title: Calendar Workflow Tutorial
description: Create and validate calendar events through the MPL proxy with full QoM reporting
---

# Calendar Workflow Tutorial

This tutorial walks you through creating and validating calendar events using the MPL proxy. You will learn how SType validation works, how the proxy rejects invalid payloads, and how to read QoM reports.

---

## Goal

By the end of this tutorial, you will:

- Send a valid calendar event through the MPL proxy
- Understand the QoM report attached to successful responses
- See how invalid payloads are rejected with descriptive errors
- Check validation metrics on the Prometheus endpoint

---

## Prerequisites

| Requirement | Version | Check Command |
|-------------|---------|---------------|
| MPL CLI | >= 0.5.0 | `mpl --version` |
| MPL Proxy | Running on `:9443` | `curl http://localhost:9443/health` |
| Python SDK | >= 0.3.0 | `pip show mpl-sdk` |
| TypeScript SDK | >= 0.3.0 | `npm list @mpl/sdk` |
| Registry | With `org.calendar.Event.v1` registered | `mpl schemas show org.calendar.Event.v1` |

---

## Step 1: Start the Proxy with the Registry

Start the MPL proxy pointing at your MCP server, with the schema registry loaded:

```bash
mpl proxy http://your-mcp-server:8080 \
  --schemas ./registry \
  --mode production
```

Verify it is running:

```bash
curl http://localhost:9443/health
```

Expected response:

```json
{
  "status": "healthy",
  "mode": "production",
  "schemas_loaded": 12,
  "uptime_seconds": 5
}
```

!!! info "Port Configuration"
    The proxy listens on port `9443` by default. Metrics are exposed on port `9100` at `/metrics`. The dashboard is available at `http://localhost:9080`.

---

## Step 2: Examine the Schema

The `org.calendar.Event.v1` SType defines the contract for calendar events. Let us inspect it:

```bash
mpl schemas show org.calendar.Event.v1
```

The schema requires three fields and allows two optional ones:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://mpl.dev/stypes/org/calendar/Event/v1/schema.json",
  "title": "Calendar Event",
  "description": "A calendar event with required time bounds and title.",
  "type": "object",
  "required": ["title", "start", "end"],
  "additionalProperties": false,
  "properties": {
    "title": {
      "type": "string",
      "minLength": 1,
      "maxLength": 500,
      "description": "Human-readable event title"
    },
    "start": {
      "type": "string",
      "format": "date-time",
      "description": "Event start time in ISO 8601 format"
    },
    "end": {
      "type": "string",
      "format": "date-time",
      "description": "Event end time in ISO 8601 format"
    },
    "timezone": {
      "type": "string",
      "description": "IANA timezone identifier (e.g., America/New_York)"
    },
    "attendees": {
      "type": "array",
      "items": {
        "type": "string",
        "format": "email"
      },
      "description": "Optional list of attendee email addresses"
    }
  }
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `title` | string | Yes | Event title (1-500 chars) |
| `start` | string (date-time) | Yes | ISO 8601 start time |
| `end` | string (date-time) | Yes | ISO 8601 end time |
| `timezone` | string | No | IANA timezone identifier |
| `attendees` | array of emails | No | Participant email addresses |

!!! warning "additionalProperties: false"
    The schema explicitly forbids undeclared fields. Any extra properties in the payload will cause validation failure.

---

## Step 3: Send a Valid Event

Send a properly structured calendar event through the proxy:

=== "Python"

    ```python
    from mpl_sdk import Client

    client = Client("http://localhost:9443")

    result = await client.call(
        "calendar.create",
        payload={
            "title": "Sprint Planning",
            "start": "2025-02-01T09:00:00Z",
            "end": "2025-02-01T10:00:00Z",
            "timezone": "America/New_York",
            "attendees": ["alice@example.com", "bob@example.com"]
        },
        headers={"X-MPL-SType": "org.calendar.Event.v1"}
    )

    print(f"Valid: {result.valid}")           # True
    print(f"QoM passed: {result.qom_passed}") # True
    print(f"Sem hash: {result.sem_hash}")     # Semantic hash of the payload
    print(f"Data: {result.data}")             # Response from the MCP server
    ```

=== "TypeScript"

    ```typescript
    import { MplClient } from '@mpl/sdk';

    const client = new MplClient('http://localhost:9443');

    const result = await client.call('calendar.create', {
      payload: {
        title: 'Sprint Planning',
        start: '2025-02-01T09:00:00Z',
        end: '2025-02-01T10:00:00Z',
        timezone: 'America/New_York',
        attendees: ['alice@example.com', 'bob@example.com'],
      },
      headers: { 'X-MPL-SType': 'org.calendar.Event.v1' },
    });

    console.log(`Valid: ${result.valid}`);         // true
    console.log(`QoM passed: ${result.qomPassed}`); // true
    console.log(`Sem hash: ${result.semHash}`);     // Semantic hash
    console.log(`Data:`, result.data);              // MCP server response
    ```

=== "curl"

    ```bash
    curl -X POST http://localhost:9443/call \
      -H "Content-Type: application/json" \
      -H "X-MPL-SType: org.calendar.Event.v1" \
      -d '{
        "method": "calendar.create",
        "payload": {
          "title": "Sprint Planning",
          "start": "2025-02-01T09:00:00Z",
          "end": "2025-02-01T10:00:00Z",
          "timezone": "America/New_York",
          "attendees": ["alice@example.com", "bob@example.com"]
        }
      }'
    ```

### Successful Response

The proxy validates the payload, computes the QoM report, and forwards the request:

```json
{
  "success": true,
  "data": {
    "eventId": "evt_a1b2c3d4",
    "status": "created"
  },
  "mpl": {
    "stype": "org.calendar.Event.v1",
    "sem_hash": "sha256:9f86d08...",
    "qom_report": {
      "profile": "qom-strict-argcheck",
      "meets_profile": true,
      "metrics": {
        "schema_fidelity": {
          "score": 1.0,
          "details": { "validation_errors": [] }
        },
        "instruction_compliance": {
          "score": 1.0,
          "details": {
            "assertions_total": 3,
            "assertions_passed": 3,
            "failures": []
          }
        }
      },
      "evaluation_duration_ms": 4
    }
  }
}
```

!!! tip "The `sem_hash` Field"
    The semantic hash (`sem_hash`) is a SHA-256 hash of the canonicalized payload. It provides a tamper-evident fingerprint for audit trails. Two identical payloads will always produce the same hash, regardless of field ordering.

---

## Step 4: Send an Invalid Event (Missing Required Field)

Now send an event missing the required `end` field:

=== "Python"

    ```python
    try:
        result = await client.call(
            "calendar.create",
            payload={
                "title": "Sprint Planning",
                "start": "2025-02-01T09:00:00Z"
                # Missing "end" field!
            },
            headers={"X-MPL-SType": "org.calendar.Event.v1"}
        )
    except client.ValidationError as e:
        print(f"Error code: {e.code}")     # E-SCHEMA-FIDELITY
        print(f"Message: {e.message}")     # Schema validation failed
        print(f"Errors: {e.details}")      # Missing required field: end
    ```

=== "TypeScript"

    ```typescript
    try {
      const result = await client.call('calendar.create', {
        payload: {
          title: 'Sprint Planning',
          start: '2025-02-01T09:00:00Z',
          // Missing "end" field!
        },
        headers: { 'X-MPL-SType': 'org.calendar.Event.v1' },
      });
    } catch (error) {
      if (error instanceof MplValidationError) {
        console.log(`Error code: ${error.code}`);    // E-SCHEMA-FIDELITY
        console.log(`Message: ${error.message}`);    // Schema validation failed
        console.log(`Errors:`, error.details);       // Missing required field: end
      }
    }
    ```

=== "curl"

    ```bash
    curl -X POST http://localhost:9443/call \
      -H "Content-Type: application/json" \
      -H "X-MPL-SType: org.calendar.Event.v1" \
      -d '{
        "method": "calendar.create",
        "payload": {
          "title": "Sprint Planning",
          "start": "2025-02-01T09:00:00Z"
        }
      }'
    ```

### Error Response

The proxy rejects the request before it reaches the MCP server:

```json
{
  "success": false,
  "error": {
    "code": "E-SCHEMA-FIDELITY",
    "message": "Payload does not conform to org.calendar.Event.v1",
    "validation_errors": [
      {
        "path": "",
        "message": "required property 'end' is missing",
        "keyword": "required",
        "params": { "missingProperty": "end" }
      }
    ],
    "stype": "org.calendar.Event.v1",
    "qom_report": {
      "profile": "qom-strict-argcheck",
      "meets_profile": false,
      "metrics": {
        "schema_fidelity": {
          "score": 0.0,
          "details": {
            "validation_errors": ["required property 'end' is missing"]
          }
        }
      }
    }
  }
}
```

!!! note "Short-Circuit Behavior"
    When Schema Fidelity fails (score = 0.0), the proxy immediately rejects the message. No further QoM metrics are evaluated. The request never reaches the upstream MCP server.

---

## Step 5: Send an Event with Extra Fields

Try adding a field not declared in the schema:

=== "Python"

    ```python
    try:
        result = await client.call(
            "calendar.create",
            payload={
                "title": "Sprint Planning",
                "start": "2025-02-01T09:00:00Z",
                "end": "2025-02-01T10:00:00Z",
                "priority": "high"  # Not in schema!
            },
            headers={"X-MPL-SType": "org.calendar.Event.v1"}
        )
    except client.ValidationError as e:
        print(f"Error: {e.code}")  # E-SCHEMA-FIDELITY
        print(f"Details: {e.details}")
    ```

=== "TypeScript"

    ```typescript
    try {
      const result = await client.call('calendar.create', {
        payload: {
          title: 'Sprint Planning',
          start: '2025-02-01T09:00:00Z',
          end: '2025-02-01T10:00:00Z',
          priority: 'high',  // Not in schema!
        },
        headers: { 'X-MPL-SType': 'org.calendar.Event.v1' },
      });
    } catch (error) {
      console.log(`Error: ${error.code}`);  // E-SCHEMA-FIDELITY
      console.log(`Details:`, error.details);
    }
    ```

=== "curl"

    ```bash
    curl -X POST http://localhost:9443/call \
      -H "Content-Type: application/json" \
      -H "X-MPL-SType: org.calendar.Event.v1" \
      -d '{
        "method": "calendar.create",
        "payload": {
          "title": "Sprint Planning",
          "start": "2025-02-01T09:00:00Z",
          "end": "2025-02-01T10:00:00Z",
          "priority": "high"
        }
      }'
    ```

### Error Response

```json
{
  "success": false,
  "error": {
    "code": "E-SCHEMA-FIDELITY",
    "message": "Payload does not conform to org.calendar.Event.v1",
    "validation_errors": [
      {
        "path": "",
        "message": "property 'priority' is not allowed",
        "keyword": "additionalProperties",
        "params": { "additionalProperty": "priority" }
      }
    ]
  }
}
```

!!! warning "Why additionalProperties: false Matters"
    Without `additionalProperties: false`, an agent could inject arbitrary data into payloads that bypasses the governance layer. This strict validation ensures every field in transit is explicitly declared and schema-checked.

---

## Step 6: Check Metrics

The MPL proxy exposes Prometheus metrics on port `9100`. Query the validation statistics:

=== "curl"

    ```bash
    curl http://localhost:9100/metrics | grep mpl_
    ```

=== "Python"

    ```python
    import httpx

    response = httpx.get("http://localhost:9100/metrics")
    for line in response.text.splitlines():
        if line.startswith("mpl_"):
            print(line)
    ```

### Expected Metrics

```prometheus
# HELP mpl_validations_total Total number of schema validations performed
# TYPE mpl_validations_total counter
mpl_validations_total{stype="org.calendar.Event.v1",result="pass"} 1
mpl_validations_total{stype="org.calendar.Event.v1",result="fail"} 2

# HELP mpl_qom_score QoM metric scores
# TYPE mpl_qom_score histogram
mpl_qom_score{metric="schema_fidelity",stype="org.calendar.Event.v1"} 1.0
mpl_qom_score{metric="instruction_compliance",stype="org.calendar.Event.v1"} 1.0

# HELP mpl_validation_duration_seconds Time spent on schema validation
# TYPE mpl_validation_duration_seconds histogram
mpl_validation_duration_seconds{stype="org.calendar.Event.v1",quantile="0.99"} 0.004

# HELP mpl_qom_breaches_total Total QoM profile breaches
# TYPE mpl_qom_breaches_total counter
mpl_qom_breaches_total{profile="qom-strict-argcheck",reason="schema_fidelity"} 2
```

!!! tip "Grafana Dashboard"
    Import the MPL Grafana dashboard from `dashboards/mpl-overview.json` for pre-built visualizations of validation rates, QoM scores, and breach trends.

---

## What You Learned

In this tutorial, you:

1. **Started the proxy** with a schema registry in production mode
2. **Inspected the schema** for `org.calendar.Event.v1` and understood required vs optional fields
3. **Sent a valid event** and received a QoM report with semantic hash
4. **Sent an invalid event** (missing field) and saw the `E-SCHEMA-FIDELITY` error
5. **Sent an event with extra fields** and saw `additionalProperties` enforcement
6. **Checked metrics** to see validation statistics exposed for monitoring

---

## Next Steps

- **[RAG with QoM Tutorial](rag-workflow.md)** -- Learn how groundedness metrics work for RAG pipelines
- **[Multi-Agent Workflow](multi-agent.md)** -- See typed communication between agents
- **[Creating a Custom SType](custom-stype.md)** -- Design your own semantic type
- **[QoM Concepts](../../concepts/qom.md)** -- Deep dive into all six quality metrics
