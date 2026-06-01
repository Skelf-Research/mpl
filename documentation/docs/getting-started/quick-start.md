# Quick Start

Get value from MPL in under 5 minutes. No configuration required.

---

## Level 0: Zero-Config Proxy

Start the proxy with a single command:

```bash
# Install
cargo install mplx

# Start proxy pointing to your MCP server
mpl proxy http://your-mcp-server:8080

# That's it! The proxy is now running on :9443
# - Dashboard: http://localhost:9080
# - Metrics:   http://localhost:9100/metrics
```

Your MCP clients can now connect through the proxy at `http://localhost:9443`. No code changes needed.

**What you get immediately:**

- Traffic visibility (all requests logged with SType detection)
- Request/response metrics (counts, latencies, error rates)
- Web dashboard for monitoring

---

## Level 1: Schema Learning

Let MPL learn your schemas from real traffic:

```bash
# Start proxy with learning enabled (default behavior)
mpl proxy http://your-mcp-server:8080 --learn

# Let it observe traffic for a while, then generate schemas
mpl schemas generate

# Review what it found
mpl schemas list

# Approve schemas (individually or all at once)
mpl schemas approve --all

# Restart with schema validation enabled
mpl proxy http://your-mcp-server:8080 --schemas ./schemas
```

**What you get:**

- Auto-generated JSON Schemas from real traffic patterns
- Schema validation for all requests passing through
- Confidence that payloads match expected structure

---

## Level 2: Production Mode

Switch to production mode to enforce validation:

```bash
mpl proxy http://your-mcp-server:8080 \
  --mode production \
  --schemas ./schemas
```

**What you get:**

- Invalid requests are **blocked** before reaching the server
- QoM (Quality of Meaning) metrics computed per request
- Typed error responses for validation failures

---

## Level 3: SDK Integration

For programmatic access, use the SDK:

=== "Python"

    ```python
    from mpl_sdk import Client, Mode

    # Simple usage - connect through the proxy
    client = Client("http://localhost:9443")
    result = await client.call("calendar.create", {
        "title": "Team Meeting",
        "start": "2024-01-15T10:00:00Z"
    })
    print(result.data)       # Response payload
    print(result.valid)      # Schema validation passed?
    print(result.qom_passed) # QoM profile met?

    # Production mode - raises on validation failure
    client = Client("http://localhost:9443", mode=Mode.PRODUCTION)
    ```

=== "TypeScript"

    ```typescript
    import { MplClient, Mode } from '@mpl/sdk';

    // Simple usage - connect through the proxy
    const client = new MplClient('http://localhost:9443');
    const result = await client.call('calendar.create', {
      title: 'Team Meeting',
      start: '2024-01-15T10:00:00Z',
    });
    console.log(result.data);      // Response payload
    console.log(result.valid);     // Schema validation passed?
    console.log(result.qomPassed); // QoM profile met?

    // Production mode - throws on validation failure
    const client = new MplClient('http://localhost:9443', { mode: Mode.Production });
    ```

---

## Development Workflow

The recommended development cycle:

```bash
# 1. Start in development mode (observe, don't enforce)
mpl proxy http://localhost:8080

# 2. Run your tests / use your application normally
npm test  # or pytest, etc.

# 3. Generate and review schemas from observed traffic
mpl schemas generate
mpl schemas list

# 4. Approve good schemas
mpl schemas approve --all

# 5. Switch to production mode for CI/CD
mpl proxy http://localhost:8080 --mode production --schemas ./schemas
```

---

## CLI Quick Reference

```bash
# Proxy
mpl proxy <upstream>                    # Zero-config start
mpl proxy <upstream> --mode production  # Enforce validation
mpl proxy <upstream> --learn            # Record traffic for learning

# Schemas
mpl schemas generate                    # Generate from recorded traffic
mpl schemas list                        # List all schemas
mpl schemas approve --all               # Approve all pending
mpl schemas show <stype>                # Show schema details

# Validation
mpl validate --stype org.calendar.Event.v1 payload.json
```

---

## Troubleshooting

### Proxy won't start

```bash
# Check if port 9443 is in use
lsof -i :9443

# Use a different port
mpl proxy http://localhost:8080 --listen 0.0.0.0:9444
```

### Schemas not generating

```bash
# Check that traffic was recorded
ls ~/.mpl/traffic/

# Lower the minimum sample threshold
mpl schemas generate --min-samples 5
```

### Validation errors in development

```bash
# Development mode logs but doesn't block
mpl proxy http://localhost:8080 --mode development

# Check the dashboard for error details
open http://localhost:9080
```

---

## Next Steps

- [First Validation](first-validation.md) — Understand validation in detail
- [Docker Compose](docker-compose.md) — Full stack deployment
- [Concepts: STypes](../concepts/stypes.md) — Deep dive into Semantic Types
- [SDK Reference](../reference/python/index.md) — Full Python SDK docs
