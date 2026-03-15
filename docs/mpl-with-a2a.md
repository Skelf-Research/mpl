# MPL Integration with A2A

This document explains how the Meaning Protocol Layer (MPL) augments Agent-to-Agent (A2A) frameworks by adding negotiated semantics, QoM enforcement, and provenance without disrupting peer-to-peer routing or transport.

## 1. Topology and Roles

- **A2A peers:** autonomous agents that both send and receive messages, often over WebSocket, gRPC, or custom brokers.
- **Directory/registry (optional):** some A2A deployments provide a discovery service. MPL does not require it but can reuse advertised capability metadata.
- **MPL runtime:** embedded in each peer (SDK) or deployed as a sidecar proxy. It participates in handshake negotiation, wraps outgoing messages, and validates incoming payloads.

```
Agent A (with MPL) ⇄ Agent B (with MPL) ⇄ Agent C (with MPL)
```

## 2. Session Establishment

1. **Transport setup:** peers connect using the existing A2A mechanism (direct WebSocket, gRPC stream, or brokered channel).
2. **AI-ALPN handshake:** immediately after connection (or when a session topic is negotiated), peers exchange capability manifests.

### Peer Hello

```json
{
  "protocols": ["a2a/0.3", "mpl/0.1"],
  "roles": ["planner", "executor"],
  "stypes": [
    "agent.TaskPlan.v1",
    "org.calendar.Event.v1",
    "eval.RAGQuery.v1"
  ],
  "tools": [
    {"id":"calendar.read.v1","args_stype":"org.calendar.Query.v1"},
    {"id":"kb.search.v1","args_stype":"eval.RAGQuery.v1"}
  ],
  "policies": ["policy.ref#consent-basic-v1"],
  "profiles": ["qom-basic", "qom-strict-argcheck"],
  "features": ["ext.profile.degradation@v1"]
}
```

### Peer Select

```json
{
  "selected": {
    "protocol": "a2a/0.3",
    "mpl_version": "0.1",
    "stypes": ["agent.TaskPlan.v1", "org.calendar.Event.v1"],
    "tools": ["calendar.read.v1"],
    "profile": "qom-basic",
    "policy": "policy.ref#consent-basic-v1"
  },
  "downgrades": [
    {"capability": "kb.search.v1", "reason": "tool unavailable"}
  ]
}
```

- Handshake is symmetric: both peers send Hello/Select messages.
- Peers log downgrade reasons to monitor capability drift and align future sessions.

## 3. Typed Peer Messages

### Plan/Request Message

```json
{
  "id": "msg-42",
  "from": "agent://planner",
  "to": "agent://executor",
  "stype": "agent.TaskPlan.v1",
  "profile": "qom-basic",
  "payload": {
    "steps": [
      {
        "tool_id": "calendar.read.v1",
        "input_stype": "org.calendar.Query.v1",
        "output_stype": "org.calendar.Event.v1",
        "args": {"eventId": "evt_123"}
      }
    ]
  },
  "sem_hash": "b3:a812...",
  "provenance": {
    "intent": "task.plan.v1",
    "inputs_ref": ["ctx:discovery#1"],
    "policy_ref": "policy.ref#consent-basic-v1"
  }
}
```

### Tool Execution Response

```json
{
  "id": "msg-42",
  "from": "agent://executor",
  "to": "agent://planner",
  "stype": "org.calendar.Event.v1",
  "payload": {
    "eventId": "evt_123",
    "title": "Design Review",
    "start": "2025-10-27T13:00:00Z",
    "end": "2025-10-27T13:30:00Z"
  },
  "sem_hash": "b3:fa13...",
  "qom_report": {
    "schema_fidelity": 1.0,
    "instruction_compliance": 1.0,
    "profile": "qom-basic",
    "meets_profile": true
  }
}
```

- QoM reports travel with messages so peers can gate follow-up actions.
- If a peer cannot meet the agreed QoM level, it should respond with `E-QOM-BREACH` or propose profile degradation (`qom-basic` → `qom-lite`) via a structured control message.

## 4. Discovery and Capability Sharing

- MPL registries complement A2A capability advertisements by standardizing SType URNs and tool descriptors.
- Peers can publish MPL manifests to the A2A discovery service so that new agents know which STypes, policies, and profiles are supported.
- When peers encounter unrecognized STypes, they respond with `E-UNKNOWN-STYPE` to trigger adapter negotiation or schema registration.

## 5. Policy and Consent

- Policies negotiated in the handshake (e.g., consent/basic, pii/mask) must be enforced in every message.
- Include `consent_ref` or `redaction_plan_id` in the provenance block when sharing user-linked data.
- Violations produce typed errors (`E-POLICY-DENIED`) that include remediation hints.

## 6. Operational Patterns

### Peer SDK Adoption

1. Wrap the A2A send/receive loops with the MPL SDK.
2. On connection, perform AI-ALPN handshake and cache results.
3. For outgoing messages, attach `stype`, `profile`, `sem_hash`, provenance.
4. For incoming messages, validate schema/QoM before passing to business logic.
5. Emit telemetry events (QoM metrics, downgrade detections, policy violations).

### Sidecar Proxy Adoption

1. Deploy sidecar proxies alongside each agent.
2. Proxies handle handshake, envelope augmentation, QoM validation, policy enforcement.
3. Optional translation layer strips MPL fields for legacy peers.
4. This model enables gradual rollout in heterogeneous multi-agent systems.

## 7. Failure & Retry Semantics

- **Schema failure:** reject with `E-SCHEMA-FIDELITY` and include the validation error path; peer can repair using the registry schema.
- **QoM breach:** respond with `E-QOM-BREACH`; optionally send `control.degrade` message proposing a lower profile.
- **Policy breach:** respond with `E-POLICY-DENIED`; include required consent scope or redaction instructions.
- **Unknown SType/tool:** respond with `E-UNKNOWN-STYPE` or `E-UNKNOWN-TOOL`; include registry hint or adapter suggestion.

Peers should distinguish semantic failures from transport errors to avoid ambiguous retries.

## 8. Observability and Governance

- **Telemetry:** track QoM pass rate, downgrade frequency, unknown SType rate, policy denial count, semantic hash mismatch.
- **Registry updates:** synchronize with central MPL registry to stay aligned on SType versions and tool descriptors.
- **Conformance:** run MPL-aware test suites (schema vectors, jitter harness) when releasing new agent capabilities.
- **Security:** optionally sign `sem_hash` values for provenance; store QoM reports for audit.

## 9. Migration Checklist

1. **Inventory STypes:** map agent message schemas and tool payloads to MPL STypes; register missing definitions.
2. **Instrument handshake:** add AI-ALPN negotiation to the peer connection phase.
3. **Enhance envelopes:** attach MPL metadata to all A2A messages.
4. **Implement QoM checks:** validate incoming messages against negotiated profiles; produce structured reports (see `docs/qom-evaluation-engine.md`).
5. **Update error handling:** emit MPL typed errors and handle them deterministically.
6. **Log provenance:** record semantic hashes, consent references, and QoM outcomes for each message.
7. **Educate peers:** document expected STypes and policies in the discovery layer so third-party agents can interoperate.

By embedding MPL inside A2A peers, teams keep their existing coordination fabric while gaining typed meaning, negotiated capabilities, and measurable semantic quality across multi-agent workflows.

## 10. Developer Interfaces

- **Registry & profile tooling:** use the MPL CLI to scaffold STypes, tool descriptors, and QoM profiles that peers advertise via the discovery service (refer to `docs/implementation-guide.md#10-developer-workflow--interfaces`).
- **Agent SDKs:** developers integrate MPL by wrapping `send()` / `receive()` primitives with SDK helpers that automatically negotiate, attach envelopes, and raise typed exceptions when QoM or policy checks fail.
- **Config-driven sidecars:** platform teams can run MPL sidecar proxies with declarative configs to enforce organization-wide policies while letting agent developers focus on business logic.
