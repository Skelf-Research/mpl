# MPL Registry Architecture

The MPL registry is the authoritative source for Semantic Types (STypes), tool descriptors, QoM profiles, policy manifests, and agent metadata. It provides discovery, versioning, governance, and caching infrastructure to ensure semantic interoperability across distributed MPL deployments.

## 1. Design Goals

- **Global namespace:** unique, collision-free identifiers for STypes and tools across organizations.
- **Immutable versioning:** published artifacts cannot be edited; changes require new versions.
- **Fast reads:** aggressive caching and CDN distribution for schema lookups.
- **Governed writes:** namespace ownership, approval workflows, and automated validation.
- **Auditability:** full Git history and tamper-evident logs for compliance.
- **Decentralization-ready:** architecture supports federation and mirrors for regional deployments.

## 2. Registry Components

```
┌─────────────────────────────────────────────────────────────┐
│                     MPL Registry                             │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────┐      │
│  │   Git Repo  │  │  API Gateway │  │   CDN/Cache   │      │
│  │  (Source)   │→│   (REST)     │→│   (Fastly)    │      │
│  └─────────────┘  └──────────────┘  └───────────────┘      │
│         │                  │                                 │
│         ▼                  ▼                                 │
│  ┌─────────────┐  ┌──────────────┐                          │
│  │ CI/CD       │  │  Search Index│                          │
│  │ (Validate)  │  │  (Elastic)   │                          │
│  └─────────────┘  └──────────────┘                          │
│                                                               │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────┐      │
│  │ Governance  │  │  Telemetry   │  │ Webhook Sink  │      │
│  │ (CODEOWNERS)│  │  (Metrics)   │  │ (Notifications)│      │
│  └─────────────┘  └──────────────┘  └───────────────┘      │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

### 2.1 Git Repository (Source of Truth)
- **Platform:** GitHub/GitLab (public or self-hosted).
- **Structure:** namespaced directory tree (see §3).
- **Immutability:** published versions are tagged; editing requires new commits (auditable).
- **Governance:** CODEOWNERS enforce approval workflows per namespace.

### 2.2 API Gateway
- **Protocol:** REST over HTTPS (HTTP/2 for multiplexing).
- **Endpoints:** schema fetch, search, namespace listing, deprecation queries (see §5).
- **Auth:** public read, OAuth/token for write (namespace-scoped).
- **Rate limiting:** per-client quotas to prevent abuse.

### 2.3 CDN & Caching
- **Global edge:** CDN (Cloudflare, Fastly, AWS CloudFront) for <100ms latency worldwide.
- **Cache TTL:** schemas cached for 1 hour; cache-busting on publish.
- **Integrity:** ETag headers with content hashes; clients validate before use.

### 2.4 CI/CD Pipeline
- **Triggers:** on PR open, commit push, tag creation.
- **Validations:** JSON Schema lint, Protobuf compile, example tests, security scans, uniqueness checks.
- **Artifacts:** on merge to main, publish to API and invalidate CDN cache.

### 2.5 Search Index
- **Technology:** Elasticsearch or Typesense for full-text search.
- **Indexed fields:** SType ID, namespace, description, tags, tool names.
- **Use case:** developers search "calendar" → find `org.calendar.Event.v1` and related tools.

### 2.6 Governance Layer
- **CODEOWNERS:** maps namespaces to teams (e.g., `/stypes/org/calendar/ @acme-calendar-team`).
- **Approval rules:** 1+ reviewers for minor changes, 2+ for major versions or new namespaces.
- **Deprecation workflow:** automated issues/PRs when sunset dates approach.

### 2.7 Telemetry
- **Metrics:** schema fetch rate, unknown SType rate, search queries, API latency.
- **Dashboards:** Grafana/Datadog showing registry health and adoption.
- **Alerts:** spike in unknown STypes, CDN cache misses, governance violations.

### 2.8 Webhook Sink
- **Events:** new SType published, tool updated, deprecation notice.
- **Subscribers:** CI/CD pipelines, MPL clients (invalidate local caches), monitoring systems.
- **Protocol:** webhook POST with JSON payload.

## 3. Repository Structure

```
registry/
├── stypes/
│   ├── org/
│   │   ├── calendar/
│   │   │   ├── Event/
│   │   │   │   ├── v1/
│   │   │   │   │   ├── schema.json          # JSON Schema definition
│   │   │   │   │   ├── examples/
│   │   │   │   │   │   ├── basic.json       # Positive test case
│   │   │   │   │   │   └── recurrence.json
│   │   │   │   │   ├── negative/
│   │   │   │   │   │   └── missing-end.json # Negative test case
│   │   │   │   │   ├── README.md            # Semantic notes
│   │   │   │   │   └── CHANGELOG.md         # Version history
│   │   │   │   └── v2/                      # Future major version
│   │   │   ├── Query/
│   │   │   │   └── v1/
│   │   │   │       └── schema.json
│   │   └── CODEOWNERS                       # Namespace maintainers
│   ├── agent/
│   │   └── TaskPlan/
│   │       └── v1/
│   │           └── schema.json
│   └── eval/
│       └── RAGQuery/
│           └── v1/
│               └── schema.json
├── tools/
│   ├── calendar.create.v1.json              # Tool descriptor
│   ├── calendar.read.v1.json
│   └── kb.search.v1.json
├── profiles/
│   ├── qom-strict-argcheck.json             # QoM profile definition
│   ├── qom-basic.json
│   └── qom-lite.json
├── policies/
│   ├── consent-basic/
│   │   └── v1/
│   │       ├── policy.rego                  # OPA Rego rules
│   │       └── README.md
│   └── gdpr-eu/
│       └── v1/
│           ├── policy.rego
│           └── CHANGELOG.md
├── adapters/
│   ├── org.calendar.Event.v1->v2/
│   │   └── map.jsonnet                      # Version adapter
│   └── org.calendar.Event.v1->com.acme.CalEvent.v1/
│       └── map.jsonnet                      # Cross-namespace adapter
├── agents/
│   └── planner/
│       ├── manifest.json                    # Agent metadata
│       └── pubkey.pem                       # Public key for signatures
├── CODEOWNERS                               # Root-level governance
├── LICENSE
└── README.md
```

### 3.1 Namespacing Rules
- **Reverse-domain notation:** `org`, `com.acme`, `io.k8s`.
- **Collision prevention:** namespaces are allocated via PR approval; first-come, first-served with verification.
- **Reserved namespaces:** `core.*`, `mpl.*` reserved for protocol maintainers.

### 3.2 Versioning Policy
- **Major versions:** breaking changes (field removal, type changes). Appears in wire identifier (`v1`, `v2`).
- **Minor versions:** backward-compatible additions (new optional fields). Tracked in `schema.json` metadata.
- **Patch versions:** documentation, example updates. Tracked in `CHANGELOG.md`.

## 4. API Specification

### 4.1 Schema Fetch

```
GET /stypes/{namespace}/{domain}/{Name}/v{major}/schema.json
```

**Example:**
```
GET https://registry.mpl.dev/stypes/org/calendar/Event/v1/schema.json
```

**Response:**
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "urn:stype:org.calendar.Event.v1",
  "version": "1.2.0",
  "type": "object",
  "properties": {
    "eventId": {"type": "string"},
    "title": {"type": "string"},
    "start": {"type": "string", "format": "date-time"},
    "end": {"type": "string", "format": "date-time"}
  },
  "required": ["eventId", "title", "start", "end"],
  "additionalProperties": false
}
```

**Headers:**
- `ETag`: content hash for cache validation.
- `Cache-Control`: `public, max-age=3600`.

### 4.2 Tool Descriptor Fetch

```
GET /tools/{toolId}.json
```

**Example:**
```
GET https://registry.mpl.dev/tools/calendar.create.v1.json
```

**Response:**
```json
{
  "id": "calendar.create.v1",
  "name": "calendar.create",
  "description": "Create a calendar event.",
  "args_stype": "org.calendar.Event.v1",
  "returns_stype": "org.calendar.Event.v1",
  "policies": ["policy.ref#consent-basic-v1"],
  "profiles": ["qom-strict-argcheck"],
  "features": ["recurrence", "attendee_roles"],
  "impl": {
    "url": "https://api.example.com/v1/calendar/event",
    "type": "http"
  }
}
```

### 4.3 QoM Profile Fetch

```
GET /profiles/{profileName}.json
```

**Example:**
```
GET https://registry.mpl.dev/profiles/qom-strict-argcheck.json
```

**Response:**
```json
{
  "name": "qom-strict-argcheck",
  "metrics": {
    "schema_fidelity": {"min": 1.0},
    "instruction_compliance": {"min": 0.97},
    "groundedness": {"min": 0.95, "sample_rate": 0.5},
    "determinism_jitter": {"min": 0.95, "sample_rate": 0.2},
    "ontology_adherence": {"min": 0.98}
  },
  "retry_policy": {
    "max_retries": 1,
    "degrade_to": "qom-basic",
    "on_failure": "escalate"
  }
}
```

### 4.4 Search

```
GET /search?q={query}&type={stype|tool|profile}&namespace={namespace}
```

**Example:**
```
GET https://registry.mpl.dev/search?q=calendar&type=stype
```

**Response:**
```json
{
  "results": [
    {
      "id": "org.calendar.Event.v1",
      "type": "stype",
      "description": "Calendar event with start/end times.",
      "namespace": "org.calendar",
      "uri": "https://registry.mpl.dev/stypes/org/calendar/Event/v1/schema.json"
    },
    {
      "id": "org.calendar.Query.v1",
      "type": "stype",
      "description": "Query parameters for calendar searches.",
      "namespace": "org.calendar",
      "uri": "https://registry.mpl.dev/stypes/org/calendar/Query/v1/schema.json"
    }
  ]
}
```

### 4.5 Namespace Listing

```
GET /namespaces
```

**Response:**
```json
{
  "namespaces": [
    {"name": "org.calendar", "owner": "acme-calendar-team", "stypes": 3, "tools": 2},
    {"name": "agent", "owner": "mpl-core-team", "stypes": 5, "tools": 0},
    {"name": "eval", "owner": "eval-working-group", "stypes": 2, "tools": 3}
  ]
}
```

### 4.6 Deprecation Queries

```
GET /deprecations
```

**Response:**
```json
{
  "deprecated": [
    {
      "id": "org.calendar.Event.v1",
      "sunset_date": "2026-12-31",
      "replacement": "org.calendar.Event.v2",
      "reason": "Added timezone support; UTC-only deprecated."
    }
  ]
}
```

### 4.7 Publish (Write API)

```
POST /stypes
Authorization: Bearer {token}
Content-Type: application/json

{
  "namespace": "org.calendar",
  "domain": "calendar",
  "name": "Event",
  "version": "v1",
  "schema": { ... },
  "examples": [ ... ]
}
```

**Response:**
```json
{
  "status": "published",
  "uri": "https://registry.mpl.dev/stypes/org/calendar/Event/v1/schema.json",
  "etag": "b3:c912..."
}
```

**Validation:**
- Token scoped to namespace `org.calendar`.
- Schema passes JSON Schema meta-validation.
- Examples validate against schema.
- No collision with existing `org.calendar.Event.v1`.

## 5. Caching & Performance

### 5.1 Client-Side Caching
- **Mechanism:** clients cache schemas locally (in-memory or disk).
- **Invalidation:** periodic polling (`If-None-Match` with ETag), webhook notifications, or TTL expiry.
- **Integrity:** validate ETag matches expected hash before using cached schema.

### 5.2 CDN Strategy
- **Edge locations:** 100+ POPs worldwide for <100ms latency.
- **Cache-Control:** `public, max-age=3600` for schemas; `no-cache` for search results.
- **Purging:** on publish, API triggers CDN purge via API (Fastly/Cloudflare).

### 5.3 API Rate Limits
- **Unauthenticated:** 100 requests/minute.
- **Authenticated:** 1000 requests/minute per token.
- **Search:** 10 requests/minute (expensive operation).

## 6. Governance & Contribution Workflow

### 6.1 Contribution Process

1. **Fork repository** and create feature branch.
2. **Add/update SType:**
   - Create directory structure: `/stypes/{ns}/{domain}/{Name}/v{major}/`.
   - Write `schema.json`, examples, negative test cases, README.
3. **Run validation:** `mpl-registry lint` (local CLI tool).
4. **Open PR:** CI runs automated checks (schema validation, example tests, security scans).
5. **Review:** CODEOWNERS-designated maintainers review; request changes or approve.
6. **Merge:** on approval, CI publishes to API and invalidates CDN cache.
7. **Notification:** webhooks notify subscribers of new SType.

### 6.2 Approval Rules
- **New namespace:** 2+ core maintainers + evidence of ownership (domain verification).
- **New SType:** 1+ namespace maintainers.
- **Major version:** 2+ namespace maintainers (breaking change).
- **Minor/patch:** 1+ namespace maintainers.

### 6.3 Automated Validation
- **Schema lint:** no unbounded `additionalProperties`, no external `$ref` URLs.
- **Example validation:** all examples must pass schema validation.
- **Negative tests:** must fail validation with expected errors.
- **Security scan:** detect suspicious patterns (regex DoS, XXE vulnerabilities).
- **Uniqueness check:** no duplicate SType IDs within namespace.

## 7. Security

See `docs/security.md` for comprehensive threat model. Registry-specific controls:
- **Signed artifacts:** schemas signed by namespace maintainers; clients verify before use.
- **Audit logs:** Git history + API access logs provide tamper-evident trail.
- **Namespace isolation:** CODEOWNERS prevent cross-namespace tampering.
- **DDoS protection:** CDN and rate limits prevent abuse.

## 8. Observability

### 8.1 Metrics
- **Request rate:** schemas fetched per second.
- **Cache hit ratio:** CDN cache effectiveness (target >95%).
- **Unknown SType rate:** clients requesting unregistered STypes (target <0.1%).
- **Search latency:** p50/p99 for search queries.
- **API errors:** 4xx/5xx rates.

### 8.2 Dashboards
- **Registry health:** uptime, latency, cache performance.
- **Adoption metrics:** SType growth, tool registrations, namespace activity.
- **Governance metrics:** PR review times, CODEOWNERS coverage.

### 8.3 Alerts
- **High unknown SType rate:** indicates schema drift or misconfiguration.
- **Cache miss spike:** CDN issues or purge failures.
- **API error spike:** backend degradation or attack.

## 9. Federation & Mirrors

### 9.1 Federated Registries
- **Use case:** organizations with air-gapped or regional requirements.
- **Architecture:** each org runs a local registry; sync via Git replication or API mirroring.
- **Namespace delegation:** root registry delegates subnamespaces (e.g., `com.acme.*` managed by Acme Corp).

### 9.2 Mirror Configuration
- **Read-only mirrors:** periodic sync from canonical registry (hourly/daily).
- **Fallback:** clients try canonical first, fallback to mirror on timeout.
- **Integrity:** mirrors serve signed artifacts; clients verify signatures.

## 10. Migration & Versioning

### 10.1 Schema Evolution
- **Backward-compatible changes:** add optional fields, increase enum values → minor version bump.
- **Breaking changes:** remove fields, change types, tighten constraints → major version bump (new `v2`).

### 10.2 Deprecation Workflow
1. **Mark deprecated:** add `deprecated: true` and `sunset_date` to schema metadata.
2. **Publish notice:** API includes deprecation info; handshake returns warning.
3. **Grace period:** 6–12 months for migration.
4. **Sunset:** after sunset date, return `E-UNKNOWN-STYPE` for deprecated version.

### 10.3 Adapters
- **Purpose:** bridge version skew (v1 → v2) or namespace differences.
- **Format:** JSONLogic, Jsonnet, or custom scripts.
- **Storage:** `/adapters/{from}->{to}/map.jsonnet`.
- **Runtime:** clients/proxies apply adapters transparently when SType mismatch detected.

## 11. Future Enhancements

- **Protobuf support:** native Protobuf schema definitions alongside JSON Schema.
- **OpenAPI integration:** auto-generate tool descriptors from OpenAPI specs.
- **Versioned policies:** track policy manifest evolution with same rigor as STypes.
- **Decentralized identity:** use DID (Decentralized Identifiers) for agent keys.
- **IPFS backend:** explore content-addressable storage for immutability guarantees.

---

For implementation guidance, see:
- `docs/implementation-guide.md#10-developer-workflow--interfaces` - CLI tooling for registry interaction
- `docs/security.md#5-secure-registry-operations` - Security controls and governance
- `GLOSSARY.md` - Registry-related term definitions (SType, namespace, deprecation, etc.)
