# MPL Integration with MCP

This guide explains how the Meaning Protocol Layer (MPL) augments the Model Context Protocol (MCP) without changing its topology or transport primitives. It describes the handshake, message envelopes, metadata, and operational workflows required to add semantic contracts, QoM gates, and provenance to an existing MCP deployment.

## 1. Roles and Topology

- **MCP Client:** typically an LLM runtime (e.g., Claude) initiating a WebSocket/HTTP session.
- **MCP Server:** exposes tools and resources the client can invoke.
- **MPL Wrapper:** a proxy or SDK that performs the AI-ALPN handshake, injects semantic envelopes, and validates QoM. It can run as:
  - A **sidecar proxy** between the client and server.
  - An **SDK** embedded in the client or server runtime.
  - A **native extension** if the MCP implementation understands MPL headers directly.

```
LLM Runtime (MCP client) ──▶ MPL wrapper ──▶ MCP Server ──▶ Tools/Resources
```

## 2. Session Establishment and Handshake

1. **Transport setup:** identical to MCP (WebSocket or HTTP upgrade).
2. **AI-ALPN handshake:** immediately after the transport handshake, the MPL layer exchanges capability manifests.

### ClientHello (sent by MCP client / MPL wrapper)

```json
{
  "protocols": ["mcp/1.1", "mpl/0.1"],
  "models": ["gpt-x-2025.08"],
  "stypes": ["org.calendar.Event.v1", "agent.TaskPlan.v1"],
  "tools": [
    {"id":"calendar.read.v1","args_stype":"org.calendar.Query.v1"},
    {"id":"calendar.create.v1","args_stype":"org.calendar.Event.v1"}
  ],
  "policies": ["policy.ref#consent-basic-v1"],
  "profiles": ["qom-strict-argcheck"],
  "features": ["ext.qom.determinism@v1"]
}
```

### ServerSelect (sent by MCP server / MPL wrapper)

```json
{
  "selected": {
    "protocol": "mcp/1.1",
    "mpl_version": "0.1",
    "stypes": ["org.calendar.Event.v1"],
    "tools": ["calendar.read.v1","calendar.create.v1"],
    "policies": ["policy.ref#consent-basic-v1"],
    "profile": "qom-strict-argcheck"
  },
  "downgrades": [
    {"capability":"ext.qom.determinism@v1","reason":"unsupported"}
  ]
}
```

- The MPL wrapper logs downgrade events and exposes metrics (downgrade rate, unknown SType rate).
- If the server cannot satisfy the requested SType major versions or QoM profile, it must fail with a typed error (`E-NEGOTIATION-INCOMPATIBLE`).

## 3. Typed Calls over MCP

### Request Envelope

Wrap standard MCP tool calls with MPL metadata while preserving the original `call` structure:

```json
{
  "id": "uuid-1",
  "type": "call",
  "tool": "calendar.create",
  "args": {
    "stype": "org.calendar.Event.v1",
    "payload": {
      "title": "Design Review",
      "start": "2025-10-27T13:00:00Z",
      "end": "2025-10-27T13:30:00Z"
    },
    "profile": "qom-strict-argcheck",
    "sem_hash": "b3:c912...",
    "provenance": {
      "intent": "calendar.create.v1",
      "inputs_ref": ["ctx:plan.step#2"],
      "consent_ref": "consent://user123/v2025-06-01"
    }
  }
}
```

- The MCP server receives a standard `call` message but now has schema, provenance, and QoM context.
- Legacy MCP servers ignore extra fields if the MPL wrapper strips them at the boundary.

### Response Envelope

```json
{
  "id": "uuid-1",
  "type": "response",
  "role": "tool",
  "content": [
    {
      "stype": "org.calendar.Event.v1",
      "payload": {
        "eventId": "evt_123",
        "title": "Design Review",
        "start": "2025-10-27T13:00:00Z",
        "end": "2025-10-27T13:30:00Z"
      },
      "sem_hash": "b3:f481...",
      "qom_report": {
        "schema_fidelity": 1.0,
        "instruction_compliance": 0.98,
        "determinism_jitter": 0.95,
        "profile": "qom-strict-argcheck",
        "meets_profile": true,
        "artifacts": [
          {"type":"claims","ref":"cas://mpl/qom/evt_123/claims"}
        ]
      }
    }
  ]
}
```

- If QoM checks fail, respond with a typed error:

```json
{
  "id": "uuid-1",
  "error": "E-QOM-BREACH",
  "hint": "determinism_jitter 0.81 < 0.95",
  "metrics": {"determinism_jitter": 0.81}
}
```

The MPL wrapper surfaces errors to the orchestrator so it can retry, degrade the profile, or escalate.

## 4. Tool Metadata

MCP servers publish tool descriptors. MPL extends them with semantic metadata:

```json
{
  "name": "calendar.create",
  "description": "Create a calendar event.",
  "input_schema": { "$ref": "https://registry.mpl.dev/stypes/org/calendar/Event/v1" },
  "output_schema": { "$ref": "https://registry.mpl.dev/stypes/org/calendar/Event/v1" },
  "mpl": {
    "id": "calendar.create.v1",
    "args_stype": "org.calendar.Event.v1",
    "returns_stype": "org.calendar.Event.v1",
    "policies": ["policy.ref#consent-basic-v1"],
    "profiles": ["qom-strict-argcheck"],
    "features": ["recurrence"],
    "impl": {"url": "https://api.example.com/v1/calendar/event", "type": "http"}
  }
}
```

- Registrations in the MPL schema registry ensure consistent SType definitions.
- Handshake can advertise tool IDs and negotiated feature subsets (`recurrence`, `attendee_roles`, etc.).

## 5. Adoption Paths

### Sidecar Proxy (fastest path)

1. Deploy proxy between MCP client and server.
2. Proxy performs handshake, injects envelopes, validates QoM, and logs provenance.
3. Optionally strip MPL fields for downstream components that are not MPL-aware.

### Client/Server SDK (clean integration)

1. Wrap the MCP client with MPL SDK to send typed calls and validate responses.
2. Wrap the MCP server to enforce schema/QoM before passing payloads to tool handlers.
3. Expose typed errors and QoM data via observability hooks.

### Native Support (long-term)

1. Extend MCP implementations to speak MPL without proxies.
2. Offer provider-signed semantic hashes and QoM reports.
3. Align roadmap with MCP governance bodies to keep specs interoperable.

## 6. Migration Checklist

1. **Schema inventory:** map existing tool inputs/outputs to STypes; register them with the MPL registry.
2. **Handshake support:** add AI-ALPN exchange after MCP session establishment.
3. **Envelope injection:** start sending `stype`, `profile`, `sem_hash`, and `provenance` fields in MCP calls/responses.
4. **Validation pipeline:** run schema validation and QoM checks server-side; expose client-side assertions (see `docs/qom-evaluation-engine.md`).
5. **Telemetry:** capture QoM metrics, downgrade events, unknown SType rate, semantic hash mismatches.
6. **Policy integration:** enforce negotiated policies (consent, redaction) inside the MPL wrapper.
7. **Error handling:** map typed MPL errors to retry/repair workflows rather than generic MCP errors.

## 7. Operational Considerations

- **Observability:** export QoM pass rate, downgrade rate, schema validation failures, and semantic checksum drift to existing monitoring stacks.
- **Version control:** use the registry’s semver rules to manage SType updates; warn clients via handshake when deprecated versions appear.
- **Testing:** add MPL-aware conformance tests (positive/negative schema vectors, jitter sampling) to MCP server CI before publishing new tools.
- **Security & compliance:** sign semantic hashes when provenance must be tamper-evident; store QoM reports alongside audit logs.

## 8. Developer Interfaces

- **Registry CLI:** developers add or update STypes/tools using `mpl-registry` commands (see `docs/implementation-guide.md#101-registry-management`). This ensures MCP tool schemas stay consistent with the MPL registry.
- **SDK surface:** the MCP-facing SDK exposes typed `call` helpers, QoM assertions, and telemetry hooks (`session.on("downgrade", ...)`) so developers integrate MPL without hand-rolling validation logic.
- **Proxy config:** configuration-driven proxy deployments let platform teams enforce policies and QoM levels centrally while application developers focus on tool handlers.

By layering MPL on MCP in this way, teams retain their existing transports and tool ecosystems while gaining explicit semantic contracts, negotiated capabilities, and measurable quality guarantees.
