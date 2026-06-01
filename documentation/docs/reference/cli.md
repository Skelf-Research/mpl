---
title: CLI Reference
---

# CLI Reference

Complete command reference for the `mpl` binary. The MPL CLI provides proxy management, schema operations, payload validation, QoM evaluation, and conformance testing.

---

## Installation

=== "Homebrew"

    ```bash
    brew install anthropic/tap/mpl
    ```

=== "Go Install"

    ```bash
    go install github.com/anthropic/mpl@latest
    ```

=== "Binary Download"

    Download the latest release from the [releases page](https://github.com/anthropic/mpl/releases) and place the binary in your `$PATH`.

---

## Global Options

These options are available on all commands:

| Option | Description |
|---|---|
| `--verbose` | Enable verbose output (debug-level logging) |
| `--quiet` | Suppress all non-essential output |
| `--data-dir <path>` | Override the default data directory (default: `~/.mpl`) |

---

## Commands

### `mpl proxy`

Start the MPL proxy server. The proxy sits between your application and the upstream MCP/A2A server, providing schema validation, QoM enforcement, and observability.

```bash
mpl proxy <upstream>
```

#### Arguments

| Argument | Description |
|---|---|
| `<upstream>` | Address of the upstream MCP/A2A server (e.g., `localhost:8080`) |

#### Options

| Option | Default | Description |
|---|---|---|
| `--listen <addr>` | `0.0.0.0:9443` | Address and port for the proxy to listen on |
| `--mode <mode>` | `transparent` | Proxy enforcement mode: `transparent`, `development`, or `production` |
| `--learn` | `false` | Enable schema learning from observed traffic |
| `--schemas <path>` | `./schemas` | Path to the schema directory |
| `--metrics-port <port>` | `9100` | Port for the Prometheus metrics endpoint |
| `--config <path>` | `./mpl-config.yaml` | Path to the configuration file |

#### Modes

| Mode | Behavior |
|---|---|
| `transparent` | Pass-through with logging only; no blocking |
| `development` | Validate and warn on violations; log detailed diagnostics |
| `production` | Enforce all schemas and policies; reject invalid payloads |

#### Examples

Start the proxy in transparent mode with default settings:

```bash
mpl proxy localhost:8080
```

Start in production mode with a custom listen address:

```bash
mpl proxy localhost:8080 \
  --listen 0.0.0.0:9443 \
  --mode production \
  --schemas ./registry/schemas
```

Start with schema learning enabled:

```bash
mpl proxy localhost:8080 \
  --learn \
  --mode development
```

Start with a specific config file:

```bash
mpl proxy localhost:8080 --config /etc/mpl/mpl-config.yaml
```

---

### `mpl schemas generate`

Generate JSON schemas from observed traffic. Requires that the proxy has been running with `--learn` enabled and has collected sufficient samples.

```bash
mpl schemas generate
```

#### Options

| Option | Default | Description |
|---|---|---|
| `--min-samples <n>` | `10` | Minimum number of observed samples required to generate a schema |
| `--output <path>` | `./schemas` | Output directory for generated schema files |

#### Examples

Generate schemas with default settings:

```bash
mpl schemas generate
```

Generate schemas requiring at least 50 samples per SType:

```bash
mpl schemas generate --min-samples 50 --output ./registry/schemas
```

---

### `mpl schemas list`

List all known schemas in the registry.

```bash
mpl schemas list
```

#### Options

| Option | Default | Description |
|---|---|---|
| `--status <status>` | `all` | Filter by status: `pending`, `approved`, or `all` |

#### Examples

List all schemas:

```bash
mpl schemas list
```

```
SType                    Status      Version   Samples
─────────────────────────────────────────────────────
mcp.tool.search          approved    1.2.0     847
mcp.tool.calculate       approved    1.0.0     312
mcp.resource.file        pending     -         28
a2a.task.summarize       pending     -         15
```

List only pending schemas:

```bash
mpl schemas list --status pending
```

---

### `mpl schemas approve`

Approve a pending schema, promoting it to active enforcement.

```bash
mpl schemas approve <stype>
```

#### Arguments

| Argument | Description |
|---|---|
| `<stype>` | The SType identifier to approve (e.g., `mcp.tool.search`) |

#### Options

| Option | Description |
|---|---|
| `--all` | Approve all pending schemas at once |

#### Examples

Approve a single schema:

```bash
mpl schemas approve mcp.tool.search
```

Approve all pending schemas:

```bash
mpl schemas approve --all
```

---

### `mpl schemas show`

Display detailed information about a specific schema, including its JSON Schema definition and metadata.

```bash
mpl schemas show <stype>
```

#### Arguments

| Argument | Description |
|---|---|
| `<stype>` | The SType identifier to display |

#### Examples

```bash
mpl schemas show mcp.tool.search
```

```
SType: mcp.tool.search
Status: approved
Version: 1.2.0
Samples: 847
Hash: sha256:a1b2c3d4...

Schema:
{
  "type": "object",
  "properties": {
    "query": { "type": "string", "minLength": 1 },
    "limit": { "type": "integer", "minimum": 1, "maximum": 100 },
    "filters": {
      "type": "object",
      "properties": {
        "date_range": { "type": "string" }
      }
    }
  },
  "required": ["query"]
}
```

---

### `mpl schemas export`

Export all approved schemas to a portable registry format suitable for distribution or version control.

```bash
mpl schemas export
```

#### Examples

```bash
mpl schemas export > registry-export.json
```

---

### `mpl validate`

Validate a JSON payload against a registered SType schema.

```bash
mpl validate <payload>
```

#### Arguments

| Argument | Description |
|---|---|
| `<payload>` | JSON payload string or path to a JSON file (prefix with `@` for file paths) |

#### Options

| Option | Required | Description |
|---|---|---|
| `--stype <stype>` | Yes | The SType to validate against |
| `--registry <path>` | No | Path to the schema registry (default: `./registry`) |

#### Examples

Validate an inline JSON payload:

```bash
mpl validate '{"query": "hello", "limit": 10}' --stype mcp.tool.search
```

```
Validation: PASSED
SType: mcp.tool.search
Schema version: 1.2.0
```

Validate from a file:

```bash
mpl validate @payload.json --stype mcp.tool.search --registry ./my-registry
```

Validation failure example:

```bash
mpl validate '{"limit": -5}' --stype mcp.tool.search
```

```
Validation: FAILED
SType: mcp.tool.search
Errors:
  - $.query: required property missing
  - $.limit: value -5 is less than minimum 1
```

---

### `mpl hash`

Compute the semantic hash of a JSON payload. Semantic hashing normalizes the payload structure before hashing, ensuring equivalent payloads produce identical hashes regardless of key ordering.

```bash
mpl hash <payload>
```

#### Arguments

| Argument | Description |
|---|---|
| `<payload>` | JSON payload string or path to a JSON file (prefix with `@`) |

#### Examples

```bash
mpl hash '{"query": "hello", "limit": 10}'
```

```
sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
```

Hash from a file:

```bash
mpl hash @request.json
```

---

### `mpl qom evaluate`

Evaluate Quality of Message (QoM) metrics for a given payload against a QoM profile.

```bash
mpl qom evaluate
```

#### Options

| Option | Description |
|---|---|
| `--profile <name>` | QoM profile to evaluate against (e.g., `qom-basic`, `qom-strict`) |
| `--payload <json>` | JSON payload to evaluate |

#### Examples

Evaluate with the basic profile:

```bash
mpl qom evaluate --profile qom-basic --payload '{"query": "summarize this document", "context": "..."}'
```

```
QoM Evaluation Results
──────────────────────
Profile: qom-basic
Overall Score: 0.87

Dimensions:
  completeness:    0.92  PASS
  consistency:     0.85  PASS
  specificity:     0.84  PASS
  schema_conform:  1.00  PASS

Verdict: PASS (threshold: 0.80)
```

Evaluate with a strict profile:

```bash
mpl qom evaluate --profile qom-strict --payload @message.json
```

---

### `mpl init`

Initialize a new registry namespace with the default directory structure and configuration files.

```bash
mpl init <namespace>
```

#### Arguments

| Argument | Description |
|---|---|
| `<namespace>` | Namespace identifier (e.g., `my-org.my-project`) |

#### Examples

```bash
mpl init my-org.my-project
```

```
Initialized MPL registry namespace: my-org.my-project
Created:
  ./registry/my-org.my-project/
  ./registry/my-org.my-project/schemas/
  ./registry/my-org.my-project/profiles/
  ./registry/my-org.my-project/policies/
  ./registry/my-org.my-project/manifest.yaml
```

---

### `mpl conformance`

Run conformance tests to verify that schemas, profiles, and policies are correctly configured and that the proxy behaves as expected.

```bash
mpl conformance
```

#### Options

| Option | Description |
|---|---|
| `--registry <path>` | Path to the registry to test (default: `./registry`) |
| `--filter <pattern>` | Filter tests by glob pattern |

#### Examples

Run all conformance tests:

```bash
mpl conformance
```

```
Running conformance tests...

  ✓ Schema validation: mcp.tool.search (12 cases)
  ✓ Schema validation: mcp.tool.calculate (8 cases)
  ✓ QoM profile: qom-basic (5 cases)
  ✓ QoM profile: qom-strict (5 cases)
  ✓ Policy: rate-limit (3 cases)
  ✓ Policy: content-filter (4 cases)

Results: 37/37 passed
```

Run only schema-related tests:

```bash
mpl conformance --filter "schema.*"
```

Run tests against a specific registry:

```bash
mpl conformance --registry ./staging-registry --filter "mcp.tool.*"
```

---

## Exit Codes

| Code | Meaning |
|---|---|
| `0` | Success |
| `1` | General error |
| `2` | Validation failure |
| `3` | Configuration error |
| `4` | Connection error (upstream unreachable) |
| `5` | Schema not found |

---

## Environment Variables

The CLI respects the following environment variables:

| Variable | Description |
|---|---|
| `MPL_DATA_DIR` | Override the default data directory |
| `MPL_CONFIG` | Path to the config file |
| `MPL_UPSTREAM` | Default upstream address |
| `MPL_MODE` | Default proxy mode |
| `MPL_LOG_LEVEL` | Log verbosity (`debug`, `info`, `warn`, `error`) |
| `MPL_METRICS_PORT` | Metrics endpoint port |

---

## Shell Completions

Generate shell completion scripts:

=== "Bash"

    ```bash
    mpl completion bash > /etc/bash_completion.d/mpl
    ```

=== "Zsh"

    ```bash
    mpl completion zsh > "${fpath[1]}/_mpl"
    ```

=== "Fish"

    ```bash
    mpl completion fish > ~/.config/fish/completions/mpl.fish
    ```
