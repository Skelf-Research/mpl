# MPL Quick Start

Get value from MPL in under 5 minutes. No configuration required.

## Level 0: Zero-Config Proxy (5 minutes)

Start the proxy with a single command:

```bash
# Install
cargo install mpl-cli

# Start proxy pointing to your MCP server
mpl proxy http://your-mcp-server:8080

# That's it! The proxy is now running on :9443
# - Metrics: http://localhost:9100/metrics
# - Dashboard: http://localhost:9080
```

Your MCP clients can now connect through the proxy. No changes needed.

**What you get:**
- Traffic visibility (all requests logged)
- Metrics (request counts, latencies)
- Dashboard for monitoring

## Level 1: Schema Learning (30 minutes)

Let MPL learn your schemas from real traffic:

```bash
# Start proxy with learning enabled (default)
mpl proxy http://your-mcp-server:8080 --learn

# Let it observe traffic for a while...
# Then generate schemas from what it learned
mpl schemas generate

# Review what it found
mpl schemas list

# Approve schemas (or approve all)
mpl schemas approve --all

# Now restart with schema validation
mpl proxy http://your-mcp-server:8080 --schemas ./schemas
```

**What you get:**
- Auto-generated JSON schemas from real traffic
- Schema validation for requests
- Confidence that payloads match expected structure

## Level 2: Production Mode (1 hour)

Switch to production mode to enforce validation:

```bash
# Production mode blocks invalid requests
mpl proxy http://your-mcp-server:8080 \
  --mode production \
  --schemas ./schemas
```

**What you get:**
- Invalid requests are blocked
- QoM (Quality of Meaning) metrics
- Higher confidence in data quality

## Level 3: SDK Integration (Optional)

For programmatic access, use the simplified SDK:

### Python

```python
from mpl_sdk import Client, Mode

# Simple usage
client = Client("http://localhost:9443")
result = await client.call("calendar.create", {
    "title": "Team Meeting",
    "start": "2024-01-15T10:00:00Z"
})
print(result.data)

# With production mode
client = Client("http://localhost:9443", mode=Mode.PRODUCTION)
```

### TypeScript

```typescript
import { Client, Mode } from 'mpl-sdk';

// Simple usage
const client = new Client('http://localhost:9443');
const result = await client.call('calendar.create', {
  title: 'Team Meeting',
  start: '2024-01-15T10:00:00Z',
});
console.log(result.data);

// With production mode
const client = new Client('http://localhost:9443', { mode: Mode.Production });
```

## CLI Reference

```bash
# Start proxy
mpl proxy <upstream>                    # Zero-config start
mpl proxy <upstream> --mode production  # Enforce validation
mpl proxy <upstream> --learn            # Record traffic for schema learning

# Schema management
mpl schemas generate                    # Generate from recorded traffic
mpl schemas list                        # List all schemas
mpl schemas approve <stype>             # Approve a specific schema
mpl schemas approve --all               # Approve all pending schemas
mpl schemas show <stype>                # Show schema details
mpl schemas export                      # Export to registry format

# Dashboard
mpl ui                                  # Launch web dashboard
mpl ui --port 8080                      # Custom port
```

## Common Patterns

### Development Workflow

```bash
# 1. Start in development mode with learning
mpl proxy http://localhost:8080

# 2. Run your tests / use your application normally

# 3. Generate and review schemas
mpl schemas generate
mpl schemas list

# 4. Approve good schemas
mpl schemas approve --all

# 5. Switch to production mode for CI/CD
mpl proxy http://localhost:8080 --mode production --schemas ./schemas
```

### CI/CD Pipeline

```yaml
# In your CI config
test:
  script:
    - mpl proxy http://mcp-server:8080 --mode production --schemas ./schemas &
    - sleep 2
    - npm test  # Tests run through the validating proxy
```

### Monitoring

Open http://localhost:9080 for the dashboard, or query metrics:

```bash
curl http://localhost:9100/metrics
```

## Next Steps

- [Schema Reference](./schemas.md) - Understanding STypes and schemas
- [QoM Profiles](./qom.md) - Quality of Meaning metrics
- [Advanced Configuration](./advanced.md) - Full configuration options
- [SDK Reference](./sdk.md) - Complete SDK documentation

## Troubleshooting

### Proxy won't start

```bash
# Check if port is in use
lsof -i :9443

# Try a different port
mpl proxy http://localhost:8080 --listen 0.0.0.0:9444
```

### Schemas not generating

```bash
# Check traffic was recorded
ls ~/.mpl/traffic/

# Need more samples? Default is 10 minimum
mpl schemas generate --min-samples 5
```

### Validation errors

```bash
# Development mode logs but doesn't block
mpl proxy http://localhost:8080 --mode development

# Check the dashboard for details
open http://localhost:9080
```
