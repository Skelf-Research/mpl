# Plan: MPL Integration Simplification

## Executive Summary

Transform MPL from a complex enterprise tool into a developer-friendly platform with progressive disclosure. Add a web UI for schema management and observability. Reduce time-to-first-value from 30 minutes to 5 minutes.

---

## Goals

1. **Zero-config proxy**: One command to start, no YAML required
2. **Schema inference**: Learn schemas from traffic, approve via UI
3. **Simple modes**: Development vs Production (not profiles)
4. **Minimal SDK**: 3 exports instead of 20+
5. **Web UI**: Dashboard for schemas, traffic, quality metrics
6. **Updated docs**: Rewrite for progressive disclosure

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           MPL Platform                                   │
│                                                                          │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────────────────────┐ │
│  │   CLI        │   │   Proxy      │   │   Web UI                     │ │
│  │              │   │              │   │                              │ │
│  │ mpl proxy    │──▶│ Zero-config  │◀──│ Dashboard                    │ │
│  │ mpl schemas  │   │ Schema learn │   │ - Traffic view               │ │
│  │ mpl ui       │   │ Mode switch  │   │ - Schema approval            │ │
│  │              │   │              │   │ - QoM metrics                │ │
│  └──────────────┘   └──────────────┘   │ - Policy editor              │ │
│                            │           └──────────────────────────────┘ │
│                            ▼                                             │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                     Core Engine                                    │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌──────────┐ │   │
│  │  │ Schema      │  │ Traffic     │  │ QoM         │  │ Policy   │ │   │
│  │  │ Inference   │  │ Recorder    │  │ Evaluator   │  │ Engine   │ │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └──────────┘ │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                                                          │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                     Storage                                        │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐   │   │
│  │  │ Schemas     │  │ Traffic Log │  │ Metrics (Prometheus)    │   │   │
│  │  │ (JSON/YAML) │  │ (SQLite)    │  │                         │   │   │
│  │  └─────────────┘  └─────────────┘  └─────────────────────────┘   │   │
│  └──────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Implementation Plan

### Phase 1: Zero-Config Proxy (Week 1-2)

#### 1.1 CLI Defaults

**Current:**
```bash
mpl-proxy --config mpl-config.yaml  # 20+ lines of config required
```

**Target:**
```bash
mpl proxy http://mcp-server:8080  # Works immediately
```

**Tasks:**
- [ ] Add `mpl` unified CLI entry point (combines proxy, cli, schemas)
- [ ] Implement sensible defaults for all config options
- [ ] Auto-detect protocol (HTTP/WebSocket) from upstream URL
- [ ] Default listen on :9443, metrics on :9100
- [ ] Default mode: development (log only, don't block)
- [ ] Create default data directory (~/.mpl/)

**Files to modify:**
- `crates/mplx/src/main.rs` - Add unified entry point
- `crates/mpl-proxy/src/config.rs` - Enhance defaults
- `crates/mpl-proxy/src/main.rs` - Simplify startup

#### 1.2 Mode Abstraction

**Tasks:**
- [ ] Add `Mode` enum: `Development`, `Production`
- [ ] Map modes to profiles internally
- [ ] Add `--mode` CLI flag
- [ ] Development: logs violations, doesn't block, schema learning ON
- [ ] Production: enforces validation, blocks violations, learning OFF

**Mode mapping:**
```rust
pub enum Mode {
    Development,  // qom-basic, transparent, learn=true
    Production,   // qom-strict-argcheck, strict, learn=false
}
```

#### 1.3 Data Directory Structure

```
~/.mpl/
├── config.yaml           # Optional overrides
├── schemas/              # Learned/approved schemas
│   ├── org.calendar.Event.v1.json
│   └── org.finance.Trade.v1.json
├── traffic.db            # SQLite traffic log
├── assertions/           # Instruction compliance rules
└── policies/             # Access control policies
```

---

### Phase 2: Schema Inference Engine (Week 3-4)

#### 2.1 Traffic Recording

**Tasks:**
- [ ] Create traffic recorder middleware
- [ ] Store requests/responses in SQLite
- [ ] Index by endpoint, timestamp, SType (if present)
- [ ] Configurable retention (default 7 days)
- [ ] Privacy: option to hash/redact sensitive fields

**Schema:**
```sql
CREATE TABLE traffic (
    id INTEGER PRIMARY KEY,
    timestamp DATETIME,
    endpoint TEXT,
    method TEXT,
    stype TEXT,           -- Declared or inferred
    request_payload JSON,
    response_payload JSON,
    validation_result TEXT,  -- pass/fail/skip
    latency_ms INTEGER
);
```

#### 2.2 Schema Inference Algorithm

**Tasks:**
- [ ] Implement JSON Schema inference from payloads
- [ ] Merge schemas from multiple samples
- [ ] Detect required vs optional fields
- [ ] Infer types, formats, enums
- [ ] Generate human-readable schema names

**Algorithm:**
```
1. Group payloads by endpoint
2. For each endpoint:
   a. Sample N payloads (default 100)
   b. Infer schema from each
   c. Merge schemas (union of fields)
   d. Mark field as required if present in >90% of samples
   e. Infer type from value patterns
3. Generate SType name from endpoint path
4. Output draft schema for approval
```

#### 2.3 CLI Commands

```bash
# Start proxy with learning enabled (default in dev mode)
mpl proxy http://mcp-server:8080 --learn

# Generate schemas from recorded traffic
mpl schemas generate
# Output:
# Generated 5 schemas:
#   org.calendar.Event.v1 (47 samples)
#   org.calendar.Query.v1 (123 samples)
#   org.finance.Trade.v1 (89 samples)
#   ...

# List pending schemas
mpl schemas list --status pending

# Approve schema (interactive)
mpl schemas approve
# Shows diff, asks for confirmation

# Approve all
mpl schemas approve --all

# Export to registry format
mpl schemas export ./registry
```

---

### Phase 3: Web UI (Week 5-8)

#### 3.1 Technology Stack

- **Frontend**: React + TypeScript + Tailwind CSS
- **Backend**: Axum (same as proxy, shared process)
- **State**: SQLite + in-memory cache
- **Charts**: Recharts or similar

#### 3.2 UI Components

**Dashboard (Home)**
```
┌─────────────────────────────────────────────────────────────────┐
│  MPL Dashboard                                    [Dev Mode ▼]  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │ Requests    │  │ Schema      │  │ QoM Pass    │              │
│  │ 12,456      │  │ Fidelity    │  │ Rate        │              │
│  │ +23% ↑      │  │ 98.7%       │  │ 94.2%       │              │
│  └─────────────┘  └─────────────┘  └─────────────┘              │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  Traffic Over Time                                        │   │
│  │  [===========================================] Chart      │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  Recent Violations                                        │   │
│  │  ┌────────────────────────────────────────────────────┐  │   │
│  │  │ 10:23:45  POST /api/trade  Schema Fidelity Failed  │  │   │
│  │  │ 10:22:12  POST /api/event  Missing required: end   │  │   │
│  │  └────────────────────────────────────────────────────┘  │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

**Schema Management**
```
┌─────────────────────────────────────────────────────────────────┐
│  Schemas                                      [+ Generate New]  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────┬────────────────────────┬─────────┬─────────┬────────┐ │
│  │ ○   │ SType                  │ Status  │ Samples │ Action │ │
│  ├─────┼────────────────────────┼─────────┼─────────┼────────┤ │
│  │ ●   │ org.calendar.Event.v1  │ Active  │ 1,234   │ [Edit] │ │
│  │ ●   │ org.calendar.Query.v1  │ Active  │ 567     │ [Edit] │ │
│  │ ○   │ org.finance.Trade.v1   │ Pending │ 89      │[Approve]│ │
│  │ ○   │ org.finance.Quote.v1   │ Pending │ 45      │[Approve]│ │
│  └─────┴────────────────────────┴─────────┴─────────┴────────┘ │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

**Schema Editor**
```
┌─────────────────────────────────────────────────────────────────┐
│  Edit: org.calendar.Event.v1                    [Save] [Cancel] │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────────────────────┬───────────────────────────────┐│
│  │ Schema (JSON)               │ Sample Payloads               ││
│  │                             │                               ││
│  │ {                           │ {                             ││
│  │   "type": "object",         │   "title": "Team Standup",   ││
│  │   "properties": {           │   "start": "2025-01-15T09:00"││
│  │     "title": {              │   "end": "2025-01-15T09:30"  ││
│  │       "type": "string"      │ }                             ││
│  │     },                      │                               ││
│  │     "start": {              │ {                             ││
│  │       "type": "string",     │   "title": "Project Review", ││
│  │       "format": "date-time" │   "start": "2025-01-16T14:00"││
│  │     },                      │   "end": "2025-01-16T15:00"  ││
│  │     ...                     │ }                             ││
│  │   },                        │                               ││
│  │   "required": ["title",     │                               ││
│  │     "start", "end"]         │                               ││
│  │ }                           │                               ││
│  └─────────────────────────────┴───────────────────────────────┘│
│                                                                  │
│  Validation: ✓ 47/47 samples pass                               │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

**Traffic Explorer**
```
┌─────────────────────────────────────────────────────────────────┐
│  Traffic                                    [Filter] [Export]   │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Filters: [Endpoint ▼] [Status ▼] [SType ▼] [Last 1 hour ▼]    │
│                                                                  │
│  ┌────────┬──────────────┬────────────────────┬────────┬──────┐│
│  │ Time   │ Endpoint     │ SType              │ Status │ ms   ││
│  ├────────┼──────────────┼────────────────────┼────────┼──────┤│
│  │ 10:23  │ POST /event  │ org.calendar.Event │ ✓ Pass │ 12   ││
│  │ 10:22  │ POST /trade  │ org.finance.Trade  │ ✗ Fail │ 8    ││
│  │ 10:21  │ GET /quote   │ org.finance.Quote  │ ✓ Pass │ 5    ││
│  │ 10:20  │ POST /event  │ org.calendar.Event │ ✓ Pass │ 15   ││
│  └────────┴──────────────┴────────────────────┴────────┴──────┘│
│                                                                  │
│  ─────────────────────────────────────────────────────────────  │
│  Request Details (selected row)                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │ Request:                    Response:                       │ │
│  │ { "title": "Meeting", ...}  { "id": "evt-123", ...}        │ │
│  │                                                             │ │
│  │ Validation: Schema Fidelity FAILED                         │ │
│  │ Error: Missing required field "end"                        │ │
│  └────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

**Settings**
```
┌─────────────────────────────────────────────────────────────────┐
│  Settings                                                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Mode                                                            │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ ○ Development - Log violations, don't block                 ││
│  │ ● Production  - Enforce validation, block violations        ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                  │
│  Upstream                                                        │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ http://mcp-server:8080                              [Test]  ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                  │
│  Schema Learning                                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ [✓] Enable automatic schema inference                       ││
│  │ [ ] Auto-approve schemas with >95% sample coverage          ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                  │
│  Traffic Retention                                               │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ Keep traffic logs for: [7 days ▼]                           ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

#### 3.3 API Endpoints

```
GET  /api/stats              - Dashboard statistics
GET  /api/traffic            - Traffic log (paginated)
GET  /api/traffic/:id        - Single traffic entry
GET  /api/schemas            - List schemas
GET  /api/schemas/:stype     - Get schema
POST /api/schemas/:stype     - Create/update schema
POST /api/schemas/generate   - Trigger schema generation
POST /api/schemas/:stype/approve - Approve pending schema
GET  /api/metrics            - QoM metrics summary
GET  /api/settings           - Current settings
PUT  /api/settings           - Update settings
```

#### 3.4 File Structure

```
crates/mpl-ui/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── api/
│   │   ├── mod.rs
│   │   ├── stats.rs
│   │   ├── traffic.rs
│   │   ├── schemas.rs
│   │   └── settings.rs
│   └── handlers.rs
└── frontend/
    ├── package.json
    ├── src/
    │   ├── App.tsx
    │   ├── pages/
    │   │   ├── Dashboard.tsx
    │   │   ├── Schemas.tsx
    │   │   ├── SchemaEditor.tsx
    │   │   ├── Traffic.tsx
    │   │   └── Settings.tsx
    │   └── components/
    │       ├── Layout.tsx
    │       ├── StatsCard.tsx
    │       ├── TrafficTable.tsx
    │       └── SchemaViewer.tsx
    └── dist/                 # Built frontend (embedded in binary)
```

---

### Phase 4: Minimal SDK (Week 9-10)

#### 4.1 Python SDK Simplification

**New simple API:**
```python
# mpl/__init__.py
from .client import Client
from .mode import Mode
from .decorators import typed

__all__ = ['Client', 'Mode', 'typed']
```

**Client class:**
```python
class Client:
    """Simple MPL client - wraps HTTP calls with validation."""

    def __init__(
        self,
        proxy_url: str = "http://localhost:9443",
        mode: Mode = Mode.DEVELOPMENT,
    ):
        self.proxy_url = proxy_url
        self.mode = mode

    async def call(
        self,
        endpoint: str,
        payload: dict,
        stype: str | None = None,  # Auto-inferred if not provided
    ) -> dict:
        """Make a validated call through the proxy."""
        ...

    def validate(self, stype: str, payload: dict) -> ValidationResult:
        """Validate payload against schema (no network call)."""
        ...
```

**Decorator:**
```python
def typed(stype: str | None = None):
    """Decorator for type-safe handlers."""
    def decorator(func):
        @wraps(func)
        async def wrapper(*args, **kwargs):
            # Validate input
            # Call function
            # Validate output
            return result
        return wrapper
    return decorator

# Usage
@typed("org.calendar.Event.v1")
async def create_event(payload: dict) -> dict:
    ...
```

#### 4.2 TypeScript SDK Simplification

**New simple API:**
```typescript
// index.ts
export { Client } from './client';
export { Mode } from './mode';
export { typed } from './decorators';

// Advanced (submodule)
export * as advanced from './advanced';
```

**Client class:**
```typescript
export class Client {
  constructor(
    proxyUrl: string = 'http://localhost:9443',
    options?: { mode?: Mode }
  ) {}

  async call<T = unknown>(
    endpoint: string,
    payload: unknown,
    stype?: string
  ): Promise<T> {}

  validate(stype: string, payload: unknown): ValidationResult {}
}
```

---

### Phase 5: Documentation Rewrite (Week 11-12)

#### 5.1 New Documentation Structure

```
docs/
├── README.md                    # Quick start (5 min)
├── getting-started.md           # Expanded quick start
├── guides/
│   ├── zero-config.md           # Level 0: Just proxy
│   ├── schema-inference.md      # Level 1: Learn schemas
│   ├── quality-enforcement.md   # Level 2: QoM checks
│   └── enterprise-governance.md # Level 3: Policies
├── ui/
│   ├── dashboard.md
│   ├── schemas.md
│   └── traffic.md
├── sdk/
│   ├── python.md
│   └── typescript.md
├── reference/
│   ├── cli.md
│   ├── config.md
│   ├── api.md
│   └── errors.md
└── advanced/
    ├── protocol-architecture.md
    ├── qom-profiles.md
    ├── policy-engine.md
    └── native-integration.md
```

#### 5.2 New README.md

```markdown
# MPL - Semantic Governance for AI Agents

Add validation, observability, and governance to your AI agents in 5 minutes.

## Quick Start

```bash
# Install
curl -sSL https://mpl.dev/install.sh | bash

# Start proxy (points to your MCP server)
mpl proxy http://mcp-server:8080

# Open dashboard
open http://localhost:9443
```

That's it. Your MCP traffic now has:
- ✅ Request/response logging
- ✅ Schema inference (learns from traffic)
- ✅ Quality metrics (Prometheus on :9100)

## Next Steps

| Goal | Guide | Time |
|------|-------|------|
| Enforce schemas | [Schema Inference](docs/guides/schema-inference.md) | 30 min |
| Add quality checks | [Quality Enforcement](docs/guides/quality-enforcement.md) | 1 hour |
| Enterprise governance | [Enterprise Guide](docs/guides/enterprise-governance.md) | 1 day |

## Dashboard

![Dashboard Screenshot](docs/images/dashboard.png)

- **Traffic view**: See all requests/responses
- **Schema management**: Approve inferred schemas
- **Quality metrics**: Monitor validation pass rates

## SDK (Optional)

For programmatic access:

```python
from mpl import Client

client = Client("http://localhost:9443")
result = await client.call("calendar.create", {"title": "Meeting", ...})
```

## Documentation

- [Full Documentation](docs/README.md)
- [CLI Reference](docs/reference/cli.md)
- [Configuration](docs/reference/config.md)

## License

Apache 2.0
```

---

## Timeline Summary

| Week | Phase | Deliverables |
|------|-------|--------------|
| 1-2 | Zero-Config | CLI defaults, mode abstraction, data directory |
| 3-4 | Schema Inference | Traffic recording, inference algorithm, CLI |
| 5-6 | Web UI (Core) | Dashboard, traffic explorer |
| 7-8 | Web UI (Schemas) | Schema management, editor, approval flow |
| 9-10 | Minimal SDK | Python/TypeScript simplification |
| 11-12 | Documentation | Full rewrite, progressive disclosure |

---

## Success Criteria

| Metric | Current | Target | Measurement |
|--------|---------|--------|-------------|
| Time to first proxy | ~30 min | <5 min | User testing |
| Config lines (basic) | 20+ | 0 | Code review |
| Concepts to understand | 6+ | 1 | Doc review |
| SDK imports (basic) | 10+ | 1 | API surface |
| User satisfaction | - | >80% | Survey |

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| UI complexity | Delays | Start with dashboard only, iterate |
| Schema inference accuracy | Bad UX | Manual override, approval flow |
| Performance (traffic logging) | Latency | Async writes, sampling option |
| Backward compatibility | Existing users | Keep advanced APIs, just add simple layer |

---

## Open Questions

1. **UI framework**: React vs Svelte vs Vue? (Recommendation: React for ecosystem)
2. **Embedded vs separate**: UI in same binary or separate service? (Recommendation: Embedded for simplicity)
3. **Storage**: SQLite vs PostgreSQL for traffic? (Recommendation: SQLite for zero-config, PG option for scale)
4. **Schema format**: JSON Schema vs simplified DSL? (Recommendation: JSON Schema with visual editor)

---

## Next Steps

1. Review and approve this plan
2. Create GitHub issues for each phase
3. Begin Phase 1 implementation
