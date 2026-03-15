# Calendar Workflow Example

This example demonstrates a complete MPL workflow for creating and managing calendar events. It includes all necessary artifacts: SType schemas, tool descriptors, QoM profiles, and sample request/response payloads.

## Overview

The workflow shows:
1. **AI-ALPN handshake** negotiating capabilities, STypes, tools, and QoM profile
2. **Typed tool call** to create a calendar event with full MPL envelope
3. **QoM validation** ensuring Schema Fidelity and Instruction Compliance
4. **Typed response** with QoM report and provenance

## Files

```
calendar-workflow/
├── README.md                           # This file
├── stypes/
│   ├── Event.v1.json                   # Event SType schema
│   └── Query.v1.json                   # Query SType schema
├── tools/
│   ├── calendar.create.v1.json         # Create tool descriptor
│   └── calendar.read.v1.json           # Read tool descriptor
├── profiles/
│   └── qom-strict-argcheck.json        # QoM profile for validation
├── requests/
│   ├── 01-handshake-client-hello.json  # Capability negotiation
│   ├── 02-handshake-server-select.json # Server response
│   └── 03-create-event-request.json    # Typed tool call
└── responses/
    ├── 01-create-event-success.json    # Successful response with QoM
    └── 02-create-event-qom-breach.json # QoM breach example
```

## Workflow Steps

### 1. AI-ALPN Handshake

Client proposes capabilities:
```bash
cat requests/01-handshake-client-hello.json
```

Server selects compatible subset:
```bash
cat requests/02-handshake-server-select.json
```

### 2. Create Calendar Event

Client sends typed request:
```bash
cat requests/03-create-event-request.json
```

Server validates schema and QoM, then responds:
```bash
cat responses/01-create-event-success.json
```

### 3. QoM Breach Handling

Example of QoM validation failure:
```bash
cat responses/02-create-event-qom-breach.json
```

## Running the Example

### Using the MPL SDK (Python)

```python
from mpl.sdk import Session

# Establish session with handshake
session = Session.connect(
    transport="wss://mcp.example.com",
    stypes=["org.calendar.Event.v1"],
    tools=["calendar.create.v1"],
    profile="qom-strict-argcheck"
)

# Create event with typed payload
response = session.call(
    tool="calendar.create.v1",
    payload={
        "eventId": "evt_001",
        "title": "Design Review",
        "start": "2025-10-27T13:00:00Z",
        "end": "2025-10-27T13:30:00Z",
        "attendees": ["alice@example.com", "bob@example.com"]
    }
)

# Validate response
assert response.qom_report.meets_profile
print(f"Event created: {response.payload['eventId']}")
```

### Using the MPL Proxy

```bash
# Start MPL proxy
mpl-proxy start \
  --upstream http://mcp-server:8080 \
  --registry https://registry.mpl.dev \
  --profile qom-strict-argcheck

# Send request via proxy
curl -X POST http://localhost:9443/tools/calendar.create \
  -H "Content-Type: application/json" \
  -H "Semantic-Type: org.calendar.Event.v1" \
  -d @requests/03-create-event-request.json
```

## Validation

### Schema Validation

```bash
# Validate request against SType schema
mpl-validate \
  --schema stypes/Event.v1.json \
  --payload requests/03-create-event-request.json
```

### QoM Evaluation

```bash
# Run QoM checks
mpl-qom evaluate \
  --profile profiles/qom-strict-argcheck.json \
  --payload requests/03-create-event-request.json \
  --response responses/01-create-event-success.json
```

## Key Concepts Demonstrated

1. **Semantic Types (STypes):** `org.calendar.Event.v1` declares event structure
2. **Tool Descriptors:** `calendar.create.v1` specifies input/output STypes
3. **QoM Profile:** `qom-strict-argcheck` enforces SF=1.0, IC≥0.97
4. **Provenance:** tracks intent, inputs, consent across workflow
5. **Typed Errors:** distinguishes schema failures from QoM breaches
6. **Semantic Hashes:** detects payload tampering or drift

## Extension Points

- Add recurrence support via feature flags
- Implement attendee role constraints (organizer vs. participant)
- Add policy enforcement (consent for personal data)
- Demonstrate jitter checks for determinism validation
- Show adapter usage for Event.v1 → Event.v2 migration

## References

- `docs/protocol-architecture.md` - Core MPL architecture
- `docs/mpl-with-mcp.md` - MCP integration details
- `docs/qom-evaluation-engine.md` - QoM metrics and enforcement
- `GLOSSARY.md` - Term definitions
