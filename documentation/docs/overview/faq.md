# FAQ

## General

### Does MPL replace MCP or A2A?

No. MPL is an **overlay** that augments existing protocols with semantic contracts. Your MCP and A2A infrastructure remains unchanged. MPL adds typed envelopes, quality metrics, and policy enforcement on top of whatever transport you use.

### What's the runtime overhead?

The sidecar proxy adds **<1ms p99 latency** for schema validation and envelope processing. QoM metrics that require re-execution (Determinism, Groundedness) are sampled, not applied to every request.

### Can I use MPL without the proxy?

Yes. The Python and TypeScript SDKs can be used for direct integration if you prefer to embed MPL logic in your application code. The proxy is recommended for zero-code adoption.

### What happens if MPL is down?

In sidecar mode, if the proxy is unavailable, traffic fails closed by default (safe behavior for regulated environments). You can configure failover to pass-through mode if availability is prioritized over enforcement.

---

## Technical

### How does MPL compare to OpenAPI/JSON Schema?

OpenAPI defines API contracts for HTTP endpoints. MPL defines **semantic contracts for agent communications**—adding quality metrics, capability negotiation, provenance, and policy enforcement that OpenAPI doesn't address. MPL uses JSON Schema internally for payload validation.

### What about non-JSON payloads?

MPL currently supports JSON payloads with JSON Schema validation. Protobuf support is planned for binary-heavy workloads. The envelope structure is transport-agnostic.

### How does versioning work?

STypes use semantic versioning. Only the **major version** appears in the wire identifier (e.g., `org.calendar.Event.v1`). Minor and patch versions are tracked in schema metadata and are backward-compatible. Major version bumps create new SType identifiers.

### Can I create custom STypes?

Yes. Define a JSON Schema, place it in the registry directory structure, and register it. See the [Custom SType Tutorial](../guides/tutorials/custom-stype.md) for a step-by-step guide.

### What schemas are pre-included?

MPL ships with 25+ pre-seeded STypes across four namespaces:

- `org.*` — Calendar events, task plans, tool invocations, workflows
- `data.*` — Tables, records, queries, file metadata
- `eval.*` — RAG queries, search results, feedback
- `ai.*` — Prompt templates, completion responses, reasoning traces

See the [Pre-seeded Types](../concepts/registry.md) section for the full list.

---

## Deployment

### What are the system requirements?

- **Proxy:** Linux/macOS, Rust 1.75+ (or Docker)
- **Python SDK:** Python 3.10+
- **TypeScript SDK:** Node.js 18+
- **Resources:** 500m CPU, 256Mi memory (recommended for proxy)

### Can I deploy to Kubernetes?

Yes. MPL provides a Helm chart for Kubernetes deployment with support for HPA, ServiceMonitor, network policies, and pod disruption budgets. See [Kubernetes Deployment](../deployment/kubernetes.md).

### How do I monitor MPL?

The proxy exposes Prometheus metrics on port 9100 including validation rates, QoM scores, latency histograms, and error counts. A built-in dashboard is available on port 9080. See [Monitoring](../guides/operations/monitoring.md).

---

## Adoption

### Where should I start?

1. Install the CLI: `cargo install mpl-cli`
2. Start the proxy in transparent mode pointing at your MCP server
3. Observe traffic and let MPL learn schemas
4. Review and approve generated schemas
5. Switch to strict mode when ready

See the [Quick Start](../getting-started/quick-start.md) for the full walkthrough.

### How long does integration take?

- **Sidecar proxy (zero-code):** Minutes to deploy, observe immediately
- **SDK integration:** A few hours for basic typed calls
- **Full QoM enforcement:** Incremental; start with Schema Fidelity, add metrics as needed

### Can I adopt MPL incrementally?

Yes. MPL is designed for progressive adoption:

1. **Transparent mode** — Observe traffic, no enforcement
2. **Learning mode** — Auto-generate schemas from traffic
3. **Selective enforcement** — Enforce on specific STypes
4. **Full enforcement** — All traffic validated and quality-checked

---

## Security & Compliance

### Which regulations does MPL help with?

MPL provides controls mapping to SOX, GDPR, HIPAA, EU AI Act, and UK FCA/PRA requirements. See the [Compliance Mapping](../security/compliance.md) for detailed control-to-regulation mappings.

### How does tamper detection work?

Every MPL envelope includes a **semantic hash** (BLAKE3) computed over the canonicalized payload. Recipients can verify the hash to detect any modification. Multi-hop integrity is maintained through hash chains in provenance metadata.

### Is the hash cryptographically secure?

Yes. MPL uses BLAKE3, a cryptographic hash function that is both secure and fast. Semantic hashing adds canonicalization (sorted keys, normalized encoding) to ensure semantically equivalent payloads produce identical hashes.
