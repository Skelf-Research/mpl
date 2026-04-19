# Calendar Workflow Tutorial

This tutorial demonstrates how to use MPL for a typed calendar event creation workflow.

## Overview

You'll learn how to:
1. Define a calendar event SType
2. Validate event payloads
3. Send typed requests through the proxy
4. Handle validation errors

## Prerequisites

- MPL proxy running (`docker compose up -d`)
- Python 3.10+

## Step 1: Check the SType

The `org.calendar.Event.v1` SType is already defined in the registry:

```bash
mpl-cli validate \
  --stype org.calendar.Event.v1 \
  --payload '{"title": "Test", "start": "2024-01-15T10:00:00Z", "end": "2024-01-15T11:00:00Z"}' \
  --registry ./registry
```

## Step 2: Run the Example

```bash
cd examples/tutorials/calendar-workflow
pip install -r requirements.txt
python calendar_client.py
```

## Step 3: Understand the Code

See `calendar_client.py` for a complete example of:
- Connecting to the proxy
- Sending typed events
- Handling validation responses

## What You'll See

```
Creating event: Team Standup
Response: 201 Created
Event ID: evt-abc123
Schema Fidelity: 1.0

Creating invalid event (missing end time)...
Validation Error: Missing required property: end
Schema Fidelity: 0.0
```

## Next Steps

- Try the [RAG Workflow Tutorial](../rag-workflow/README.md)
- Read about [QoM Profiles](../../../docs/qom-evaluation-engine.md)
