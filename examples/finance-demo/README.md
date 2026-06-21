# MPL Finance Advisory Demo

An interactive financial advisory agent that demonstrates MPL's semantic validation and fiduciary compliance enforcement.

## Architecture

```
User (terminal)
    -> Agent on Ollama Cloud (gpt-oss:20b, function calling via OpenAI-compatible API)
        -> MPL Proxy (localhost:9443, strict mode)
            -> Finance MCP Server (localhost:8080)
```

The agent talks to Ollama Cloud (https://ollama.com/v1) using the OpenAI Python
SDK with a custom `base_url`. The MPL proxy validates investment recommendations
against `org.finance.InvestmentRecommendation.v1` schema and its 9 CEL-based
fiduciary assertions.

## Setup

```bash
# Get a key from https://ollama.com/settings/keys, then:
export OLLAMA_API_KEY=...

# Build the proxy (done automatically on first run)
cargo build --release -p mpl-proxy

# Run the demo (uv handles dependencies automatically)
uv run demo.py
```

## What It Demonstrates

1. **Schema Enforcement** - Recommendations must match the JSON Schema (required fields, correct types, valid enums)
2. **Assertion Checks** - CEL expressions enforce fiduciary rules:
   - Investment rationale must be >= 50 characters
   - Risk disclosure required (>= 20 characters)
   - Crypto assets must be marked aggressive/speculative
   - Speculative investments need explicit risk warnings
   - Sell recommendations must explain exit reasoning
3. **QoM Measurement** - Quality of Meaning scores reported on every request

## Example Queries

- "What's in my portfolio?"
- "Should I invest in AAPL?"
- "Should I buy some Bitcoin?" (triggers crypto risk assertions)
- "Should I sell MSFT?" (triggers sell rationale assertion)
- "Rebalance my portfolio to be more conservative"

## Available Symbols

AAPL, MSFT, VOO, BTC, AGG, GLD
