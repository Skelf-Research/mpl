# MPL Integration Modes: Decision Guide

This document consolidates the recommended integration paths for adopting MPL, when to use each mode, and migration strategies from plain MCP/A2A to MPL-enabled deployments.

---

## MPL's Ecosystem Strategy

MPL offers **three integration modes** in priority order for ecosystem adoption:

1. **Sidecar Proxy** (PRIMARY) - Universal adoption path, zero code changes, works everywhere
2. **Native Integration** (ECOSYSTEM) - MCP/A2A vendors add first-class MPL support
3. **SDK** (POWER USERS) - Optional for fine-grained control and stateful assertions

**Strategic rationale:**
- **Proxy gets everyone started immediately** (30 min, zero friction)
- **Native integration by vendors** means clients don't need SDKs at all
- **SDK is for specialized use cases**, not the default path

This ordering prioritizes **ecosystem-wide adoption** over individual developer convenience.

---

## Quick Decision Matrix

| Your Situation | Recommended Mode | Time to First Typed Call | Effort |
|----------------|------------------|--------------------------|--------|
| **Anyone evaluating MPL** | Sidecar Proxy | <30 min | Minimal |
| **Legacy system, can't modify code** | Sidecar Proxy | <30 min | Low |
| **Regulated enterprise, compliance-first** | Sidecar Proxy | 30 min | Low |
| **MCP/A2A server vendor** | Native Integration | 1-2 weeks | High |
| **Need stateful assertions (e.g., position limits)** | SDK (Python/TS) | <1 hour | Medium |
| **High-performance requirement** | Native Integration | 2-4 weeks | High |
| **New greenfield agent** | Sidecar Proxy (start) | <30 min | Low |

---

## The Three Integration Modes

### Mode 1: Sidecar Proxy (Zero-Code Integration)

**Architecture:**
```
┌─────────────┐         ┌─────────────────┐         ┌──────────────┐
│   Client    │────────>│   MPL Proxy     │────────>│ MCP/A2A      │
│ (unchanged) │         │  (validates,    │         │ Server       │
│             │<────────│   enriches)     │<────────│ (unchanged)  │
└─────────────┘         └─────────────────┘         └──────────────┘
                              │
                              ├─ Schema validation (SF)
                              ├─ Assertion checks (IC)
                              ├─ QoM reporting
                              ├─ Provenance logging
                              └─ Semantic hashing
```

**What it does:**
- Intercepts MCP WebSocket/HTTP or A2A gRPC traffic
- Performs AI-ALPN handshake negotiation
- Wraps payloads in MPL envelopes
- Validates schemas and runs QoM checks
- Strips MPL headers if downstream is legacy
- Exports telemetry (Prometheus, structured logs)

**Configuration (YAML):**
```yaml
transport:
  listen: 0.0.0.0:9443
  upstream: mcp-server.internal:8080
  protocol: websocket  # or http, grpc

mpl:
  registry: https://github.com/mpl/registry/raw/main
  mode: transparent  # or strict (reject non-MPL)

  # QoM enforcement
  required_profile: qom-basic
  enforce_schema: true
  enforce_assertions: true

  # Policy
  policy_engine: false  # MVP: disabled

observability:
  metrics_port: 9100
  metrics_format: prometheus
  logs: stdout
  log_format: json
  log_level: info

# Optional: route specific STypes to specific upstreams
routing:
  - stype_pattern: "org.calendar.*"
    upstream: calendar-service:8080
  - stype_pattern: "org.trading.*"
    upstream: trading-service:8080
```

**Deployment:**
```bash
# Docker Compose
docker run -d \
  -p 9443:9443 \
  -p 9100:9100 \
  -v ./mpl-config.yaml:/etc/mpl/config.yaml \
  mpl/proxy:latest

# Kubernetes (Sidecar pattern)
apiVersion: v1
kind: Pod
metadata:
  name: agent-with-mpl
spec:
  containers:
  - name: mcp-server
    image: my-mcp-server:latest
    ports:
    - containerPort: 8080
  - name: mpl-proxy
    image: mpl/proxy:latest
    ports:
    - containerPort: 9443
    - containerPort: 9100
    volumeMounts:
    - name: mpl-config
      mountPath: /etc/mpl
  volumes:
  - name: mpl-config
    configMap:
      name: mpl-config
```

**Pros:**
- ✅ **Zero code changes** to existing services
- ✅ **Fastest time to value** (<30 min setup)
- ✅ **Works with any language/framework**
- ✅ **Centralized policy enforcement**
- ✅ **Easy rollback** (remove proxy)
- ✅ **Kubernetes-native** (sidecar pattern)

**Cons:**
- ⚠️ **Extra network hop** (~5-15ms latency)
- ⚠️ **Limited context** (can't access application state for stateful assertions)
- ⚠️ **Operational overhead** (another component to manage)
- ⚠️ **Less flexible** than SDK integration

**When to use:**
- Legacy systems you can't modify
- Rapid evaluation/POC
- Compliance requirement without engineering bandwidth
- Multi-language environment (proxy works for all)
- Kubernetes deployments (leverage sidecar pattern)

**MVP timeline:** Week 11-12 (see `mvp-scope.md`)

---

### Mode 2: SDK Integration (Code-Level Integration)

**Architecture:**
```
┌──────────────────────────────────────┐
│  Your Application                    │
│  ┌────────────────────────────────┐  │
│  │  MPL SDK                       │  │
│  │  ├─ Schema validation          │  │
│  │  ├─ QoM checks                 │  │
│  │  ├─ Provenance logging         │  │
│  │  └─ Typed error handling       │  │
│  └────────────────────────────────┘  │
│         │                             │
│         ├─> MCP/A2A transport         │
└─────────┼───────────────────────────┘
          │
          └──────> MCP/A2A Server
```

**Client SDK (Python example):**
```python
from mpl.sdk import Session

# Connect with MPL handshake
session = Session.connect(
    transport="wss://mcp.example.com",
    stypes=["org.calendar.Event.v1", "org.calendar.Query.v1"],
    tools=["calendar.create.v1", "calendar.read.v1"],
    profile="qom-strict-argcheck",
    registry="https://github.com/mpl/registry/raw/main"
)

# Make typed call
response = await session.call(
    tool="calendar.create.v1",
    payload={
        "title": "Team Standup",
        "start": "2025-11-06T09:00:00Z",
        "end": "2025-11-06T09:30:00Z"
    }
)

# Automatic validation
assert response.qom_report.schema_fidelity == 1.0
assert response.qom_report.meets_profile

# Typed error handling
try:
    await session.call(...)
except MPLSchemaError as e:
    print(f"Schema violation: {e.field_path} - {e.message}")
except MPLQoMBreach as e:
    print(f"QoM breach: {e.metric} = {e.actual} (expected {e.threshold})")
```

**Server SDK (Python example):**
```python
from mpl.sdk import defineTool, MPLServer

@defineTool(
    id="calendar.create.v1",
    args_stype="org.calendar.Event.v1",
    returns_stype="org.calendar.Event.v1",
    profile="qom-strict-argcheck"
)
async def create_event(payload):
    # Payload already validated by SDK before this runs
    # Assertions already checked

    event = await calendar_db.insert({
        "title": payload["title"],
        "start": payload["start"],
        "end": payload["end"]
    })

    # Return is auto-validated before sending
    return event

# Run server
server = MPLServer(
    tools=[create_event],
    registry="https://github.com/mpl/registry/raw/main"
)
server.run(host="0.0.0.0", port=8080)
```

**TypeScript SDK (example):**
```typescript
import { Session, defineTool } from '@mpl/sdk';

// Client
const session = await Session.connect({
  transport: 'wss://mcp.example.com',
  stypes: ['org.calendar.Event.v1'],
  tools: ['calendar.create.v1'],
  profile: 'qom-strict-argcheck'
});

const response = await session.call({
  tool: 'calendar.create.v1',
  payload: { title: 'Meeting', start: '...', end: '...' }
});

// Server
export const createEvent = defineTool({
  id: 'calendar.create.v1',
  argsStype: 'org.calendar.Event.v1',
  returnsStype: 'org.calendar.Event.v1',
  handler: async ({ payload }) => {
    const event = await calendarAPI.create(payload);
    return event;
  }
});
```

**Telemetry Hooks:**
```python
from mpl.sdk import Session

session = Session.connect(
    ...,
    telemetry={
        'on_qom_result': lambda report: metrics.record(report),
        'on_downgrade': lambda event: alerts.send(event),
        'on_schema_failure': lambda error: log.error(error)
    }
)
```

**Pros:**
- ✅ **Fine-grained control** (access to application state)
- ✅ **Stateful assertions** (e.g., position limits using current balance)
- ✅ **Lower latency** (no proxy hop)
- ✅ **Better developer experience** (typed errors, IDE autocomplete)
- ✅ **Telemetry hooks** (custom observability)
- ✅ **Portable** (same code works across deployments)

**Cons:**
- ⚠️ **Code changes required** (refactor to use SDK)
- ⚠️ **Language-specific** (need SDK per language)
- ⚠️ **Slower adoption** (engineering work)
- ⚠️ **Dependency management** (SDK versioning)

**When to use:**
- New greenfield agents
- High-performance requirements
- Need stateful assertions
- Strong typing requirements
- Long-term MPL commitment

**MVP timeline:** Week 9-10 (see `mvp-scope.md`)

---

### Mode 3: Native Integration (Transport-Level Integration)

**Architecture:**
```
┌─────────────────────────────────────┐
│  MCP/A2A Server (MPL-native)        │
│  ┌───────────────────────────────┐  │
│  │  Built-in MPL Support         │  │
│  │  ├─ Handshake negotiation     │  │
│  │  ├─ Envelope parsing          │  │
│  │  ├─ Schema validation         │  │
│  │  ├─ QoM evaluation            │  │
│  │  └─ Provenance generation     │  │
│  └───────────────────────────────┘  │
│         │                            │
│         └─> Tool handlers            │
└─────────────────────────────────────┘
```

**Example: MCP Server with Native MPL (Python):**
```python
from mcp import MCPServer
from mpl import MPLExtension

server = MCPServer()

# Enable MPL extension
mpl = MPLExtension(
    registry="https://github.com/mpl/registry/raw/main",
    qom_profile="qom-strict-argcheck",
    enforce_schema=True,
    enforce_assertions=True
)
server.register_extension(mpl)

# Tools automatically get MPL envelope wrapping
@server.tool("calendar.create.v1")
def create_event(args: dict) -> dict:
    # MCP + MPL handle handshake, validation, QoM, provenance
    return calendar_db.insert(args)

server.run()
```

**Example: A2A Peer with Native MPL (Go):**
```go
import (
    "github.com/a2a/protocol"
    "github.com/mpl/go-sdk"
)

func main() {
    peer := a2a.NewPeer(a2a.Config{
        ID: "agent://executor",
    })

    // Enable MPL middleware
    peer.Use(mpl.Middleware{
        Registry: "https://github.com/mpl/registry/raw/main",
        Profile:  "qom-strict-argcheck",
    })

    // Register MPL-aware handler
    peer.HandleTool("trading.execute.v1", func(req mpl.Request) (mpl.Response, error) {
        // Request already validated and enriched with provenance
        result := executeTrade(req.Payload)
        return mpl.Response{Payload: result}, nil
    })

    peer.Start()
}
```

**Pros:**
- ✅ **Best performance** (no proxy, minimal SDK overhead)
- ✅ **Cleanest architecture** (MPL is first-class)
- ✅ **Full protocol support** (all MPL features)
- ✅ **Vendor differentiation** (MPL-native server = premium)
- ✅ **Ecosystem standardization** (encourages adoption)

**Cons:**
- ⚠️ **High implementation effort** (weeks to months)
- ⚠️ **Transport coupling** (need to implement per transport)
- ⚠️ **Maintenance burden** (keep up with MPL spec changes)
- ⚠️ **Limited to your ecosystem** (only helps if you're a platform)

**When to use:**
- You're an MCP/A2A server vendor
- Building a managed agent platform
- Performance is critical (high-frequency trading, real-time systems)
- Want to offer MPL as differentiator

**MVP timeline:** Not in Phase 1 MVP; recommended for Phase 2+ or vendor partnerships

---

## Adoption Strategies

### Strategy 1: Proxy-First (RECOMMENDED for 90% of adopters)

**The default path: Start with proxy, stay with proxy unless you need specialized features.**

**Phase 1: Deployment (Week 1)**
```
Day 1: Deploy sidecar proxy (Docker/K8s)
       Configure for existing MCP/A2A server
       Point clients to proxy endpoint

Day 2: Define 10-20 STypes in registry
       Create QoM profile (qom-basic)
       Enable schema validation

Day 3: Add assertions (Instruction Compliance)
       Configure telemetry (Prometheus)
       Set up monitoring dashboards

Day 4-5: Validation testing
         Present to stakeholders
         Get Compliance/Risk approval
```

**Phase 2: Ecosystem Integration (Months 2-6)**
```
Month 2-3: Work with MCP/A2A vendors
           Share STypes and QoM profiles
           Encourage native MPL support

Month 4-6: As vendors add native support:
           - Clients benefit automatically (no changes)
           - Proxy becomes optional (backward compat)
           - Performance improves (no proxy hop)
```

**Phase 3: Specialized Use Cases (As Needed)**
```
IF you need stateful assertions:
  → Use SDK for specific services
  → Keep proxy for everything else

IF vendor adds native MPL:
  → Deprecate proxy gradually
  → No client changes needed
```

**Why this works:**
- ✅ Zero friction adoption (<30 min)
- ✅ Works with ANY client/server (no code changes)
- ✅ As ecosystem matures, you benefit automatically
- ✅ SDK only if you need specialized features
- ✅ Future-proof (works with native MPL servers)

---

### Strategy 2: Vendor-Led Ecosystem Play

**For MCP/A2A server vendors who want to drive ecosystem adoption:**

**Phase 1: Add Native MPL Support (Quarter 1-2)**
```
Q1: Spec review, architecture design
    Implement handshake, envelope parsing
    Add schema validation (SF)

Q2: Implement QoM evaluation (IC)
    Add provenance generation
    Beta release with design partners
```

**Phase 2: Go-to-Market (Quarter 3)**
```
- Release "MPL-native" server
- Publish marketing: "Zero-config MPL compliance"
- Clients get MPL benefits automatically
- Differentiation vs. competitors
```

**Phase 3: Ecosystem Benefits (Quarter 4+)**
```
- Other vendors add MPL support
- STypes become cross-vendor standard
- Network effects kick in
- Your early adoption = market leadership
```

**Why this works:**
- ✅ Vendors drive adoption (users don't need to do anything)
- ✅ Network effects (more vendors → more value)
- ✅ Differentiation (first-mover advantage)
- ✅ Clients get MPL "for free"

---

### Strategy 3: SDK for Power Users (Specialized Use Cases)

**Use SDK ONLY when you need:**
- Stateful assertions (access to application state)
- Custom telemetry hooks
- Fine-grained error handling
- Performance optimization (bypass proxy)

**When to use:**
```
┌─────────────────────────────────────────────┐
│  Typical Deployment (Proxy)                 │
│  ✅ 90% of use cases                        │
│  - Legacy systems                           │
│  - Compliance requirements                  │
│  - Fast POC                                 │
└─────────────────────────────────────────────┘

┌─────────────────────────────────────────────┐
│  Specialized (SDK)                          │
│  ⚡ 10% of use cases                        │
│  - Trading systems (position limits)        │
│  - Custom observability                     │
│  - Performance-critical paths               │
└─────────────────────────────────────────────┘
```

**Hybrid pattern (large orgs):**
```
┌─────────────────────────────────────────────┐
│  All Services: Proxy (default)              │
│    ↓                                        │
│  High-value services: SDK overlay          │
│    - Trading (stateful assertions)          │
│    - Real-time (performance)                │
│    - Custom metrics (telemetry hooks)       │
└─────────────────────────────────────────────┘
```

**Why this works:**
- ✅ Proxy handles 90% of cases
- ✅ SDK only where justified
- ✅ No "big migration" needed
- ✅ Incremental value

---

## Design Partner Recommendations

Based on stakeholder type (aligned with ecosystem strategy):

### Regulated Enterprises (Financial Services, Healthcare)

**Start with:** Sidecar Proxy (STAY with proxy)
**Reason:** Fastest path to compliance approval, zero code changes

**Rollout:**
```
Week 1: Deploy proxy for 1-2 critical agents
        Get Compliance/Risk approval
        Demonstrate audit value

Month 2-3: Expand proxy to all agents
           Work with vendors to add native MPL
           Build shared SType library

Month 4-12: As vendors add native MPL:
            - Deprecate proxy gradually
            - No client changes needed
            - Compliance/Risk already approved

SDK only if: Need stateful assertions (e.g., position limits)
```

**Key insight:** Proxy is NOT a temporary measure—it's the production solution until vendors go native.

**Key stakeholders:** Compliance, Risk, AI Safety (all get value from proxy)

---

### Startups / Greenfield Projects

**Start with:** Sidecar Proxy (NOT SDK)
**Reason:** Zero lock-in, faster iteration, benefit from ecosystem maturity

**Rollout:**
```
Day 1: Deploy proxy, define STypes
Day 2-3: Add assertions, enable QoM
Week 2+: Build features (not MPL integration)
```

**Why not SDK:**
- ❌ SDK = code coupling, harder to change
- ❌ When vendors go native, you wasted SDK effort
- ✅ Proxy = configuration, easy to evolve
- ✅ Spend time on product, not protocol integration

**SDK only if:** Performance-critical path or stateful assertions

**Key stakeholders:** Engineering (velocity), Product (focus on features)

---

### MCP/A2A Server Vendors (CRITICAL for ecosystem)

**Start with:** Native Integration
**Reason:** Differentiation, drive ecosystem adoption, capture market

**Rollout:**
```
Quarter 1: Spec review, architecture design
           Implement handshake, validation (SF)
Quarter 2: Add QoM evaluation (IC)
           Provenance generation
           Beta with design partners
Quarter 3: GA release with "MPL-native" badge
           Marketing: clients get compliance for free
Quarter 4: Cross-vendor SType standardization
           Network effects kick in
```

**Market positioning:**
- "Zero-config compliance for your agents"
- "Built-in audit trails and schema validation"
- "MPL-native—no proxy needed"

**Key stakeholders:** Product (differentiation), Engineering (platform investment), BD (ecosystem play)

---

### Large Enterprises (Mixed Estate)

**Start with:** Proxy for EVERYTHING
**Reason:** Uniform adoption, no refactoring needed

**Rollout:**
```
Month 1: Deploy proxy for all agents (100% coverage)
         Centralized SType registry
         Uniform QoM policies

Month 2-6: Build assertion library
           Work with vendors on native MPL
           Monitor for specialized SDK needs

Month 7-12: SDK only for ~5-10% of services:
            - Trading (stateful assertions)
            - Real-time (performance)
            - Custom metrics

            Proxy stays for 90%+ of services
```

**Pattern:**
```
Proxy (default) → 90% of services
   ↓
SDK (specialized) → 10% of services (trading, real-time)
   ↓
Native (future) → As vendors adopt, deprecate proxy
```

**Key stakeholders:** Enterprise Architecture, Platform Engineering, Compliance

---

## Performance Comparison

| Mode | Latency Overhead | Throughput Impact | Resource Cost |
|------|------------------|-------------------|---------------|
| **Proxy** | +5-15ms | -5-10% | +1 container per service |
| **SDK** | +2-5ms | -2-5% | +50-100MB memory |
| **Native** | +1-3ms | <1% | Negligible (integrated) |

**Notes:**
- Overhead dominated by schema validation (2-5ms) + assertion checks (variable)
- Proxy adds network hop (TCP handshake, serialization)
- SDK overhead is in-process function calls
- Caching reduces overhead significantly (schemas, compiled assertions)

---

## Integration Checklist

### Proxy Integration
- [ ] Deploy proxy (Docker/K8s)
- [ ] Configure upstream MCP/A2A endpoint
- [ ] Point client to proxy instead of server
- [ ] Define 10+ STypes in registry
- [ ] Create QoM profile
- [ ] Enable Prometheus metrics
- [ ] Test schema validation
- [ ] Test assertion enforcement
- [ ] Verify audit logs
- [ ] Present to stakeholders

**Time estimate:** 1-2 days

---

### SDK Integration (Client)
- [ ] Install SDK (`pip install mpl-sdk` or `npm install @mpl/sdk`)
- [ ] Refactor connection to use `Session.connect()`
- [ ] Replace raw tool calls with `session.call()`
- [ ] Add error handling (MPLSchemaError, MPLQoMBreach)
- [ ] Implement telemetry hooks
- [ ] Write unit tests
- [ ] Validate QoM metrics

**Time estimate:** 1 week per service

---

### SDK Integration (Server)
- [ ] Install SDK
- [ ] Define STypes for tools
- [ ] Decorate tool handlers with `@defineTool`
- [ ] Create QoM profile with assertions
- [ ] Implement QoM evaluation hooks
- [ ] Add provenance logging
- [ ] Write integration tests
- [ ] Deploy to staging
- [ ] Validate with design partners

**Time estimate:** 2-3 weeks per server

---

## FAQ

**Q: Can I use proxy AND SDK together?**
A: Yes! Hybrid mode is common. SDK on client for typed errors; proxy on server for legacy services.

**Q: What if my language doesn't have an SDK?**
A: Use proxy mode. Or contribute an SDK (we'll provide spec + reference implementation).

**Q: How do I migrate from proxy to SDK without downtime?**
A: Blue/green deployment. Run both, A/B test, gradually shift traffic, decommission proxy.

**Q: Does proxy work with all transports?**
A: MVP supports WebSocket and HTTP. gRPC coming in Phase 2.

**Q: Can I run proxy in "transparent mode" (no enforcement)?**
A: Yes, set `mode: transparent` to log only, no blocking. Good for evaluation.

**Q: What's the minimum QoM profile for MVP?**
A: `qom-basic` with Schema Fidelity only. Add Instruction Compliance when ready.

**Q: How do I handle legacy clients that don't support MPL?**
A: Proxy can strip MPL headers for downstream. Or use "transparent mode" to observe without breaking.

**Q: What if QoM check fails? Does request fail?**
A: Depends on profile. `fail_fast` = yes. `best_effort` = warn only. Configurable per metric.

**Q: How do I test without deploying proxy/SDK?**
A: Use CLI tool: `mpl validate --stype org.calendar.Event.v1 --payload event.json`

---

## Next Steps

1. **Choose your integration mode** based on decision matrix above
2. **Review transport-specific guides:**
   - `docs/mpl-with-mcp.md` for MCP integration
   - `docs/mpl-with-a2a.md` for A2A integration
3. **Follow implementation guide:**
   - `docs/implementation-guide.md` for technical details
4. **Check MVP scope:**
   - `docs/mvp-scope.md` for phased rollout

**For hands-on help:** Join MPL working group or reach out to design partner program.

---

**Summary:**

| Priority | Mode | Best For | Time to Value | Ecosystem Role |
|----------|------|----------|---------------|----------------|
| **#1** | **Proxy** | Everyone (90% of adopters) | <30 min | Universal adoption |
| **#2** | **Native** | MCP/A2A vendors | 2-4 weeks | Ecosystem play |
| **#3** | **SDK** | Power users (10% of cases) | 1 week | Specialized needs |

**MPL's Ecosystem Strategy:**

1. **Proxy** gets everyone started immediately (zero friction)
2. **Native** integration by vendors makes proxy optional (ecosystem play)
3. **SDK** only for specialized use cases (stateful assertions, performance)

**Recommendation for most teams:** Start with **Proxy**, stay with **Proxy** unless you need specialized features. When vendors add native MPL support, you benefit automatically.
