# MPL Integration Simplification Analysis

## Problem Statement

MPL's current integration model has significant friction that will slow adoption:

1. **Cognitive Load**: 6+ new concepts before getting value (STypes, QoM, profiles, handshakes, envelopes, provenance)
2. **Configuration Explosion**: 20+ config options across transport, mpl, observability, routing, limits
3. **Schema Management Overhead**: Manual creation of SType definitions with specific directory structure
4. **Profile Confusion**: 4 profiles with unclear selection criteria
5. **SDK Surface Area**: 20+ exported types overwhelming for simple use cases

---

## Root Cause Analysis

### The Core Tension

MPL is designed for **enterprise governance** but presented as a **developer tool**. These audiences have different needs:

| Enterprise Governance | Developer Tool |
|----------------------|----------------|
| Completeness over simplicity | Simplicity over completeness |
| Configuration flexibility | Convention over configuration |
| Explicit control | Implicit defaults |
| Audit everything | Get things working |

**Current state**: We've optimized for enterprise completeness, creating friction for developer adoption.

**Insight**: Developers adopt first, enterprises follow. We need to flip the priority.

---

## Proposed Simplification Principles

### 1. Progressive Disclosure

```
Level 0: Zero-config → Instant value
Level 1: Add schemas → Schema validation
Level 2: Add assertions → Quality checks
Level 3: Add policies → Enterprise governance
```

Users should get value at Level 0 without understanding Levels 1-3.

### 2. Convention over Configuration

Every config option should have a sensible default. Users configure only what they want to change.

### 3. Schema Inference

Schemas should be **discovered**, not **declared**. Observe traffic, infer types, let users refine.

### 4. Simple Mental Model

Replace profiles with modes:
- **Development**: Log everything, block nothing
- **Production**: Enforce validation, block violations

---

## Concrete Proposals

### Proposal 1: Zero-Config Proxy

**Current experience:**
```yaml
# mpl-config.yaml (20+ lines required)
transport:
  listen: 0.0.0.0:9443
  upstream: localhost:8080
  protocol: http
mpl:
  registry: ./registry
  mode: transparent
  required_profile: qom-basic
  enforce_schema: true
  enforce_assertions: true
observability:
  metrics_port: 9100
  log_format: json
  log_level: info
```

**Proposed experience:**
```bash
# Just works - one command
mpl proxy http://mcp-server:8080

# Listens on :9443
# Metrics on :9100
# Development mode (logs, doesn't block)
# Auto-discovers schemas from traffic
```

**Implementation:**
- All config has sensible defaults
- CLI args override defaults
- Config file only for advanced use cases

### Proposal 2: Schema Inference Mode

**Current experience:**
```bash
# Must manually create:
mkdir -p registry/stypes/org/calendar/Event/v1
echo '{"type": "object", ...}' > schema.json
# 50+ lines of JSON Schema
# Must know naming conventions
# Must understand directory structure
```

**Proposed experience:**
```bash
# Step 1: Observe traffic
mpl proxy http://mcp-server:8080 --learn

# Step 2: Generate schemas from observed payloads
mpl schemas generate
# Creates: ./schemas/org.calendar.Event.v1.json (inferred)
# Creates: ./schemas/org.finance.Trade.v1.json (inferred)

# Step 3: Review and approve
mpl schemas approve  # Interactive review
# or
mpl schemas approve --all  # Accept all inferred schemas

# Step 4: Enforce
mpl proxy http://mcp-server:8080 --schemas ./schemas
```

**Benefits:**
- No upfront schema authoring
- Schemas match real traffic
- Review workflow builds confidence
- Progressive path to enforcement

### Proposal 3: Simple Modes Instead of Profiles

**Current experience:**
```yaml
required_profile: qom-basic  # or qom-strict-argcheck? qom-outcome? qom-comprehensive?
# User doesn't know which to choose
# Must read docs to understand differences
```

**Proposed experience:**
```bash
# Mode-based (simple)
mpl proxy http://mcp-server:8080 --mode development  # Log only
mpl proxy http://mcp-server:8080 --mode production   # Enforce

# Profile-based (advanced, optional)
mpl proxy http://mcp-server:8080 --profile qom-strict-argcheck
```

**Mapping:**
| Mode | Profile | Behavior |
|------|---------|----------|
| `development` | `qom-basic` | Log violations, don't block |
| `production` | `qom-strict-argcheck` | Enforce SF=1.0, IC>=0.97 |

**Benefits:**
- Clear intent (development vs production)
- No profile confusion for common cases
- Advanced users can still use profiles directly

### Proposal 4: Minimal SDK API

**Current Python experience:**
```python
from mpl_sdk import (
    Session, SessionConfig, SType, MplEnvelope,
    QomProfile, QomMetrics, SchemaValidator,
    # ... 15+ more imports
)

session = Session(SessionConfig(
    endpoint="ws://localhost:9443/ws",
    stypes=["org.calendar.Event.v1", "org.calendar.Query.v1"],
    qom_profile=QomProfile.STRICT_ARGCHECK,
    registry_path="./registry",
))

async with session:
    response = await session.send(
        stype="org.calendar.Event.v1",
        payload={...}
    )
    if response.qom_report.meets_profile:
        ...
```

**Proposed experience:**
```python
from mpl import Client

# Simple - wraps existing calls
client = Client("http://localhost:9443")
result = await client.call("calendar.create", {"title": "Meeting", ...})

# That's it. Validation happens automatically.
# Errors are raised as exceptions.
# No SType/QoM/Profile knowledge needed.
```

**Advanced (when needed):**
```python
from mpl import Client, Mode

client = Client(
    "http://localhost:9443",
    mode=Mode.PRODUCTION,  # or Mode.DEVELOPMENT
)

# Type-safe (optional)
from mpl.types import CalendarEvent  # Auto-generated from schemas
result = await client.call(CalendarEvent, {"title": "Meeting", ...})
```

### Proposal 5: Decorator-Based Server Integration

**Current experience:**
```python
from mpl.sdk import defineTool, MPLServer

@defineTool(
    id="calendar.create.v1",
    args_stype="org.calendar.Event.v1",
    returns_stype="org.calendar.Event.v1",
    profile="qom-strict-argcheck"
)
async def create_event(payload):
    ...

server = MPLServer(tools=[create_event], registry="./registry")
```

**Proposed experience:**
```python
from mpl import typed

@typed  # That's it - schema inferred from type hints
async def create_event(event: CalendarEvent) -> CalendarEvent:
    ...

# Or with explicit SType (when needed)
@typed("org.calendar.Event.v1")
async def create_event(payload: dict) -> dict:
    ...
```

### Proposal 6: Clear Value Ladder Documentation

Instead of complex integration-modes.md, create a simple progression:

```
┌─────────────────────────────────────────────────────────────────┐
│  LEVEL 0: ZERO CONFIG (5 minutes)                               │
│  mpl proxy http://mcp-server:8080                               │
│  → Traffic logging, metrics, no validation                      │
├─────────────────────────────────────────────────────────────────┤
│  LEVEL 1: SCHEMA VALIDATION (30 minutes)                        │
│  mpl proxy http://mcp-server:8080 --learn                       │
│  mpl schemas generate && mpl schemas approve                    │
│  mpl proxy http://mcp-server:8080 --schemas ./schemas           │
│  → Schema fidelity enforcement                                  │
├─────────────────────────────────────────────────────────────────┤
│  LEVEL 2: QUALITY CHECKS (1 hour)                               │
│  Add assertions.yaml for instruction compliance                 │
│  mpl proxy http://mcp-server:8080 --mode production             │
│  → QoM enforcement with SF + IC                                 │
├─────────────────────────────────────────────────────────────────┤
│  LEVEL 3: ENTERPRISE GOVERNANCE (1 day)                         │
│  Add policies.yaml for access control                           │
│  Configure audit logging, provenance, signatures                │
│  → Full enterprise governance                                   │
└─────────────────────────────────────────────────────────────────┘
```

---

## Implementation Roadmap

### Phase 1: Zero-Config Experience (Week 1-2)

1. **CLI defaults**: Every proxy option has a sensible default
2. **Single-command start**: `mpl proxy <upstream>` works out of box
3. **Auto-detect protocol**: HTTP/WebSocket/gRPC inferred from upstream
4. **Default observability**: Metrics on :9100, structured logs to stdout

### Phase 2: Schema Inference (Week 3-4)

1. **Traffic observation**: Record payloads by endpoint
2. **Schema generation**: Infer JSON Schema from observed payloads
3. **Interactive approval**: CLI workflow to review/approve schemas
4. **Incremental learning**: Update schemas as new payloads observed

### Phase 3: Simple Modes (Week 5)

1. **Mode abstraction**: development/production map to profiles
2. **Clear documentation**: What each mode does
3. **Easy switching**: `--mode` flag on CLI

### Phase 4: Minimal SDK (Week 6-8)

1. **Python `Client` class**: Simple wrapper, minimal API
2. **TypeScript `Client` class**: Matching simplicity
3. **Auto-generated types**: From schemas to language types
4. **Decorator integration**: `@typed` for servers

---

## Metrics for Success

| Metric | Current | Target |
|--------|---------|--------|
| Time to first proxy | ~30 min | <5 min |
| Config lines for basic use | 20+ | 0 |
| Concepts to understand | 6+ | 1 (proxy) |
| SDK imports for basic use | 10+ | 1 |
| Docs pages to read | 5+ | 1 |

---

## Key Insight

**The best integration is invisible integration.**

Users shouldn't have to understand MPL to get value from MPL. The proxy should "just work" and provide:
- Traffic visibility (what's being called)
- Payload validation (when schemas exist)
- Quality metrics (when assertions exist)
- Policy enforcement (when policies exist)

Each layer adds value without requiring understanding of the layers below.

---

## Appendix: API Surface Comparison

### Current TypeScript Exports (22 items)
```typescript
// Types (8)
SType, STypeParseError, STypeComponents
MplEnvelope, MplEnvelopeOptions, Provenance, QomReport
QomMetrics, QomProfile, QomProfileConfig, QomEvaluation, MetricFailure, MetricThreshold

// Validation (4)
SchemaValidator, SchemaValidationError, ValidationResult, ValidationError
canonicalize, semanticHash, verifyHash

// Session (4)
Session, SessionConfig, NegotiatedCapabilities, SendOptions

// Errors (7)
MplError, SchemaFidelityError, QomBreachError, UnknownStypeError
NegotiationError, ConnectionError, HashMismatchError, PolicyDeniedError
```

### Proposed Minimal Exports (5 items)
```typescript
// Primary API
Client        // Simple client wrapper
Mode          // development | production

// Errors (when needed)
MplError      // Base error
ValidationError
PolicyError

// Advanced (re-export from submodules)
import { Session, QomProfile, ... } from 'mpl/advanced'
```

### Current Python Classes (10+)
```python
PySType, PySchemaValidator, PyQomMetrics, PyQomProfile
PyMplEnvelope, PyValidationResult, PyClientHello, PyServerSelect
...
```

### Proposed Minimal Classes (3)
```python
Client    # Simple client
typed     # Decorator for servers
Mode      # development | production

# Advanced (when needed)
from mpl.advanced import Session, QomProfile, ...
```

---

## Next Steps

1. **Validate with users**: Test these proposals with 3-5 design partners
2. **Prioritize**: Rank proposals by impact/effort
3. **Prototype**: Build zero-config experience first
4. **Iterate**: Refine based on feedback

The goal is not to remove features, but to **hide complexity** until it's needed.
