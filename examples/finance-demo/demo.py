#!/usr/bin/env python3
# /// script
# requires-python = ">=3.10"
# dependencies = [
#     "openai>=1.0",
#     "requests>=2.28",
# ]
# ///
"""
MPL Finance Advisory Demo
==========================
A single-script demo that showcases a tool-using financial advisory agent
communicating with a finance MCP server through the MPL proxy.

The proxy validates investment recommendations against the
org.finance.InvestmentRecommendation.v1 schema and its fiduciary compliance assertions.

Architecture:
  User (terminal)
      -> Agent on Ollama Cloud (gpt-oss:20b, function calling via OpenAI-compatible API)
          -> MPL Proxy (localhost:9443)
              -> Finance MCP Server (localhost:8080)

Usage:
  export OLLAMA_API_KEY=...           # from https://ollama.com/settings/keys
  uv run demo.py
"""

import json
import os
import signal
import subprocess
import sys
import threading
import time
import uuid
from datetime import datetime, timezone
from http.server import HTTPServer, BaseHTTPRequestHandler

import requests
from openai import OpenAI

# ─── Configuration ────────────────────────────────────────────────────────────

PROXY_PORT = 9443
SERVER_PORT = 8080
PROXY_URL = f"http://localhost:{PROXY_PORT}"
# Ollama Cloud exposes an OpenAI-compatible API at https://ollama.com/v1.
# gpt-oss is OpenAI's open-weight agentic model; the `:cloud` suffix routes
# the request through Ollama's hosted GPUs rather than a local Ollama daemon.
OLLAMA_BASE_URL = "https://ollama.com/v1"
MODEL = "gpt-oss:20b-cloud"

# Path resolution (relative to this script)
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.abspath(os.path.join(SCRIPT_DIR, "..", ".."))
PROXY_BINARY = os.path.join(PROJECT_ROOT, "target", "release", "mpl-proxy")
CONFIG_PATH = os.path.join(SCRIPT_DIR, "mpl-config.yaml")
REGISTRY_PATH = os.path.join(PROJECT_ROOT, "registry")

# ─── Simulated Market Data ────────────────────────────────────────────────────

MARKET_DATA = {
    "AAPL": {"price": 178.50, "change": 2.3, "volume": 52_000_000, "pe_ratio": 28.5,
             "week52_high": 199.62, "week52_low": 143.90, "asset_class": "equity"},
    "MSFT": {"price": 378.90, "change": -1.1, "volume": 25_000_000, "pe_ratio": 35.2,
             "week52_high": 384.30, "week52_low": 309.45, "asset_class": "equity"},
    "VOO": {"price": 452.30, "change": 0.8, "volume": 4_500_000, "pe_ratio": 22.1,
            "week52_high": 460.00, "week52_low": 380.50, "asset_class": "etf"},
    "BTC": {"price": 43_250.00, "change": -3.5, "volume": 28_000_000_000, "pe_ratio": None,
            "week52_high": 73_800.00, "week52_low": 25_100.00, "asset_class": "crypto"},
    "AGG": {"price": 98.50, "change": 0.1, "volume": 8_200_000, "pe_ratio": None,
            "week52_high": 101.20, "week52_low": 95.80, "asset_class": "bond"},
    "GLD": {"price": 192.80, "change": 1.2, "volume": 9_100_000, "pe_ratio": None,
            "week52_high": 198.50, "week52_low": 168.30, "asset_class": "commodity"},
}

PORTFOLIO = {
    "clientId": "client-001",
    "name": "Default Portfolio",
    "totalValue": 250_000.00,
    "holdings": [
        {"symbol": "VOO", "shares": 200, "value": 90_460.00, "allocation": 36.2},
        {"symbol": "AAPL", "shares": 150, "value": 26_775.00, "allocation": 10.7},
        {"symbol": "MSFT", "shares": 80, "value": 30_312.00, "allocation": 12.1},
        {"symbol": "AGG", "shares": 500, "value": 49_250.00, "allocation": 19.7},
        {"symbol": "GLD", "shares": 100, "value": 19_280.00, "allocation": 7.7},
        {"symbol": "BTC", "shares": 0.5, "value": 21_625.00, "allocation": 8.7},
    ],
    "cash": 12_298.00,
    "cashAllocation": 4.9,
}

# ─── Finance MCP Server ──────────────────────────────────────────────────────

class FinanceMCPHandler(BaseHTTPRequestHandler):
    """Simple HTTP handler simulating a finance MCP server."""

    def log_message(self, format, *args):
        """Suppress default request logging."""
        pass

    def do_GET(self):
        if self.path == "/health":
            self._respond(200, {"status": "healthy", "service": "finance-mcp"})
        else:
            self._respond(404, {"error": "not found"})

    def do_POST(self):
        content_length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(content_length) if content_length > 0 else b"{}"
        try:
            payload = json.loads(body) if body else {}
        except json.JSONDecodeError:
            payload = {}

        if self.path == "/api/market-data":
            self._handle_market_data(payload)
        elif self.path == "/api/portfolio":
            self._handle_portfolio(payload)
        elif self.path == "/api/recommendations":
            self._handle_recommendation(payload)
        else:
            self._respond(404, {"error": "not found"})

    def _handle_market_data(self, payload):
        # Extract args from Record wrapper
        args = payload.get("data", payload)
        symbol = args.get("symbol", "").upper()
        if symbol in MARKET_DATA:
            data = MARKET_DATA[symbol].copy()
            data["symbol"] = symbol
            record = {
                "recordId": f"mkt-{symbol}-{uuid.uuid4().hex[:8]}",
                "data": data,
                "timestamps": {"createdAt": datetime.now(timezone.utc).isoformat()},
            }
            self._respond(200, record)
        else:
            self._respond(404, {"error": f"Unknown symbol: {symbol}"})

    def _handle_portfolio(self, payload):
        args = payload.get("data", payload)
        client_id = args.get("clientId", "client-001")
        record = {
            "recordId": f"pf-{client_id}-{uuid.uuid4().hex[:8]}",
            "data": PORTFOLIO,
            "timestamps": {"createdAt": datetime.now(timezone.utc).isoformat()},
        }
        self._respond(200, record)

    def _handle_recommendation(self, payload):
        # The payload IS the recommendation - just add an ID and timestamp
        rec = payload.copy()
        rec["recommendationId"] = f"REC-{uuid.uuid4().hex[:8]}"
        rec["generatedAt"] = datetime.now(timezone.utc).isoformat()
        if "clientId" not in rec:
            rec["clientId"] = "client-001"
        self._respond(201, rec)

    def _respond(self, status, body):
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps(body).encode())


def start_mcp_server():
    """Start the finance MCP server in a background thread."""
    server = HTTPServer(("0.0.0.0", SERVER_PORT), FinanceMCPHandler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    return server


# ─── MPL Proxy Management ────────────────────────────────────────────────────

def build_proxy():
    """Build the MPL proxy binary if not present."""
    if os.path.isfile(PROXY_BINARY):
        return True

    print("  Building mpl-proxy (first run only)...")
    result = subprocess.run(
        ["cargo", "build", "--release", "-p", "mpl-proxy"],
        cwd=PROJECT_ROOT,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print(f"  Build failed: {result.stderr[:500]}")
        return False
    return True


def start_proxy():
    """Start the MPL proxy as a subprocess."""
    proc = subprocess.Popen(
        [
            PROXY_BINARY,
            "--config", CONFIG_PATH,
            "--registry", REGISTRY_PATH,
        ],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        cwd=PROJECT_ROOT,
    )
    # Wait for proxy to be ready
    for _ in range(30):
        time.sleep(0.2)
        try:
            r = requests.get(f"{PROXY_URL}/health", timeout=1)
            if r.status_code == 200:
                return proc
        except requests.ConnectionError:
            pass
    # Check if process died
    if proc.poll() is not None:
        stderr = proc.stderr.read().decode()
        print(f"  Proxy failed to start: {stderr[:500]}")
    return None


# ─── Agent (Ollama Cloud via OpenAI-compatible API) ───────────────────────────

SYSTEM_PROMPT = """You are a financial advisor assistant. You help clients understand their \
portfolio and make investment decisions. Be concise and direct.

Available tools:
- get_market_data: Look up current market data for a security symbol
- get_portfolio: Retrieve the client's current portfolio holdings
- create_recommendation: Create a formal investment recommendation"""

TOOLS = [
    {
        "type": "function",
        "function": {
            "name": "get_market_data",
            "description": "Get current market data for a security symbol (price, volume, P/E, 52-week range)",
            "parameters": {
                "type": "object",
                "properties": {
                    "symbol": {
                        "type": "string",
                        "description": "Security symbol (e.g., AAPL, MSFT, VOO, BTC, AGG, GLD)",
                    }
                },
                "required": ["symbol"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "get_portfolio",
            "description": "Retrieve the client's current portfolio holdings, allocations, and total value",
            "parameters": {
                "type": "object",
                "properties": {
                    "client_id": {
                        "type": "string",
                        "description": "Client identifier (default: client-001)",
                    }
                },
                "required": [],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "create_recommendation",
            "description": "Create a formal investment recommendation. Requires substantive rationale and risk disclosure.",
            "parameters": {
                "type": "object",
                "properties": {
                    "symbol": {"type": "string", "description": "Security symbol"},
                    "action": {
                        "type": "string",
                        "enum": ["buy", "sell", "hold", "rebalance"],
                        "description": "Recommended action",
                    },
                    "rationale": {
                        "type": "string",
                        "description": "Detailed explanation (must be at least 50 characters)",
                    },
                    "riskLevel": {
                        "type": "string",
                        "enum": ["conservative", "moderate", "aggressive", "speculative"],
                        "description": "Risk classification",
                    },
                    "riskDisclosure": {
                        "type": "string",
                        "description": "Risk disclosure statement (required, at least 20 characters)",
                    },
                    "assetClass": {
                        "type": "string",
                        "enum": ["equity", "bond", "etf", "mutual_fund", "crypto", "commodity", "real_estate", "cash"],
                        "description": "Asset class classification",
                    },
                    "amount": {"type": "number", "description": "Recommended investment amount"},
                    "allocationPercentage": {
                        "type": "number",
                        "description": "Target allocation percentage (0-100)",
                    },
                    "timeHorizon": {
                        "type": "string",
                        "enum": ["short_term", "medium_term", "long_term"],
                        "description": "Investment time horizon",
                    },
                    "confidenceScore": {
                        "type": "number",
                        "description": "Model confidence (0.0-1.0)",
                    },
                },
                "required": ["symbol", "action", "rationale", "riskLevel"],
            },
        },
    },
]


def execute_tool_call(name, arguments):
    """Execute a tool call through the MPL proxy and return (result, mpl_info)."""
    mpl_info = {}

    if name == "get_market_data":
        stype = "data.record.Record.v1"
        endpoint = "/api/market-data"
        payload = {
            "recordId": f"query-{uuid.uuid4().hex[:8]}",
            "data": {"symbol": arguments.get("symbol", "")},
        }
    elif name == "get_portfolio":
        stype = "data.record.Record.v1"
        endpoint = "/api/portfolio"
        payload = {
            "recordId": f"query-{uuid.uuid4().hex[:8]}",
            "data": {"clientId": arguments.get("client_id", "client-001")},
        }
    elif name == "create_recommendation":
        stype = "org.finance.InvestmentRecommendation.v1"
        endpoint = "/api/recommendations"
        payload = arguments
    else:
        return {"error": f"Unknown tool: {name}"}, mpl_info

    headers = {"Content-Type": "application/json"}
    if stype:
        headers["X-MPL-SType"] = stype
        headers["X-MPL-Profile"] = "qom-basic"

    try:
        resp = requests.post(
            f"{PROXY_URL}{endpoint}",
            json=payload,
            headers=headers,
            timeout=10,
        )

        # Collect MPL response headers
        mpl_info["stype"] = stype
        mpl_info["status"] = resp.status_code
        mpl_info["qom_result"] = resp.headers.get("X-MPL-QoM-Result", "n/a")
        mpl_info["sem_hash"] = resp.headers.get("X-MPL-Sem-Hash", "")

        result = resp.json()

        # If validation failed (400), extract details
        if resp.status_code == 400:
            mpl_info["validation_failed"] = True
            mpl_info["errors"] = result.get("details", [])
            return result, mpl_info

        mpl_info["validation_failed"] = False
        return result, mpl_info

    except requests.ConnectionError:
        return {"error": "Could not connect to MPL proxy"}, mpl_info
    except Exception as e:
        return {"error": str(e)}, mpl_info


def display_mpl_result(tool_name, mpl_info):
    """Display MPL validation results in the terminal."""
    stype = mpl_info.get("stype", "unknown")
    status = mpl_info.get("status", "?")
    qom = mpl_info.get("qom_result", "n/a")
    failed = mpl_info.get("validation_failed", False)
    qom_icon = "\u2713" if qom == "pass" else ("\u2717" if qom == "fail" else "?")

    print(f"  \u2192 MPL: SType={stype}")
    print(f"    Schema: {'PASS' if not failed else 'FAIL'} | QoM: {qom} {qom_icon} | Status: {status}")

    if failed:
        errors = mpl_info.get("errors", [])
        if errors:
            print("    Validation errors:")
            for err in errors:
                print(f"      \u2717 {err}")
    else:
        sem_hash = mpl_info.get("sem_hash", "")
        if sem_hash:
            print(f"    Sem-Hash: {sem_hash[:16]}...")


def run_agent():
    """Run the interactive agent loop against Ollama Cloud's OpenAI-compatible API."""
    api_key = os.environ.get("OLLAMA_API_KEY")
    if not api_key:
        print("\nError: OLLAMA_API_KEY environment variable not set.")
        print("  Get one from https://ollama.com/settings/keys, then:")
        print("  export OLLAMA_API_KEY=...")
        sys.exit(1)

    client = OpenAI(api_key=api_key, base_url=OLLAMA_BASE_URL)
    messages = [{"role": "system", "content": SYSTEM_PROMPT}]

    print("\nType your financial questions below. Type 'quit' or Ctrl+C to exit.\n")
    print("Example questions:")
    print("  - What's in my portfolio?")
    print("  - Should I invest in AAPL?")
    print("  - Should I buy some Bitcoin?")
    print("  - Should I sell MSFT?")
    print()

    while True:
        try:
            user_input = input("You: ").strip()
        except (EOFError, KeyboardInterrupt):
            print("\nGoodbye!")
            break

        if not user_input:
            continue
        if user_input.lower() in ("quit", "exit", "q"):
            print("Goodbye!")
            break

        messages.append({"role": "user", "content": user_input})

        # Agent loop: may require multiple iterations for tool calls
        while True:
            response = client.chat.completions.create(
                model=MODEL,
                messages=messages,
                tools=TOOLS,
                tool_choice="auto",
            )

            choice = response.choices[0]
            message = choice.message

            # If the model wants to call tools
            if message.tool_calls:
                messages.append(message)

                for tool_call in message.tool_calls:
                    fn_name = tool_call.function.name
                    fn_args = json.loads(tool_call.function.arguments)

                    print(f"\n[Agent] Calling {fn_name}({json.dumps(fn_args, separators=(',', ':'))})")

                    result, mpl_info = execute_tool_call(fn_name, fn_args)
                    display_mpl_result(fn_name, mpl_info)

                    # Feed result back to the model
                    tool_result = json.dumps(result)
                    messages.append({
                        "role": "tool",
                        "tool_call_id": tool_call.id,
                        "content": tool_result,
                    })

                # Continue the loop to get the final response or more tool calls
                continue

            # Final text response
            if message.content:
                print(f"\nAdvisor: {message.content}\n")
                messages.append({"role": "assistant", "content": message.content})

            break


# ─── Validation Showcase ──────────────────────────────────────────────────────

def run_showcase():
    """Send deliberate good and bad requests to demonstrate MPL enforcement."""

    showcase_cases = [
        {
            "label": "1. Insufficient rationale (too short)",
            "payload": {
                "symbol": "AAPL",
                "action": "buy",
                "rationale": "Looks good",
                "riskLevel": "moderate",
            },
        },
        {
            "label": "2. Speculative asset without proper risk warning",
            "payload": {
                "symbol": "DOGE",
                "action": "buy",
                "rationale": "Meme coin with viral potential and growing community support could see significant short-term gains",
                "riskLevel": "speculative",
                "riskDisclosure": "This investment may lose value.",
                "assetClass": "crypto",
                "allocationPercentage": 40,
            },
        },
        {
            "label": "3. Valid recommendation (all assertions pass)",
            "payload": {
                "symbol": "VOO",
                "action": "buy",
                "rationale": "Broad market ETF with low expense ratio provides excellent diversification and long-term growth potential",
                "riskLevel": "moderate",
                "riskDisclosure": "Past performance does not guarantee future results. All investments carry risk of loss.",
                "assetClass": "etf",
                "timeHorizon": "long_term",
                "confidenceScore": 0.9,
            },
        },
    ]

    for case in showcase_cases:
        p = case["payload"]
        print(f"\n  {case['label']}")
        print(f"    Request: symbol={p['symbol']} action={p['action']} riskLevel={p['riskLevel']}")
        print(f"    Rationale: \"{p['rationale'][:60]}{'...' if len(p['rationale']) > 60 else ''}\"")
        if "riskDisclosure" in p:
            print(f"    Disclosure: \"{p['riskDisclosure'][:60]}{'...' if len(p['riskDisclosure']) > 60 else ''}\"")
        else:
            print(f"    Disclosure: (none)")
        result, mpl_info = execute_tool_call("create_recommendation", case["payload"])
        display_mpl_result("create_recommendation", mpl_info)


# ─── Main ─────────────────────────────────────────────────────────────────────

def main():
    print("\n" + "=" * 60)
    print("  MPL Finance Advisory Demo")
    print(f"  Model: {MODEL} | Proxy: localhost:{PROXY_PORT}")
    print("=" * 60)

    # Step 1: Build proxy if needed
    print("\n[1/3] Checking MPL proxy binary...")
    if not build_proxy():
        print("  Failed to build mpl-proxy. Run: cargo build --release -p mpl-proxy")
        sys.exit(1)
    print("  OK")

    # Step 2: Start MCP server
    print(f"[2/3] Starting Finance MCP server on :{SERVER_PORT}...")
    server = start_mcp_server()
    # Verify server is up
    time.sleep(0.3)
    try:
        r = requests.get(f"http://localhost:{SERVER_PORT}/health", timeout=2)
        if r.status_code != 200:
            raise Exception("unhealthy")
    except Exception:
        print("  Failed to start MCP server")
        sys.exit(1)
    print("  OK")

    # Step 3: Start MPL proxy
    print(f"[3/3] Starting MPL proxy on :{PROXY_PORT}...")
    proxy_proc = start_proxy()
    if proxy_proc is None:
        print("  Failed to start MPL proxy")
        print(f"  Make sure port {PROXY_PORT} is available")
        sys.exit(1)
    print("  OK")

    print("\n" + "-" * 60)
    print("\n  MPL Validation Showcase")
    print("  " + "-" * 40)
    run_showcase()
    print("\n" + "-" * 60)

    # Run the agent
    try:
        run_agent()
    except KeyboardInterrupt:
        print("\nShutting down...")
    finally:
        if proxy_proc:
            proxy_proc.terminate()
            proxy_proc.wait(timeout=5)
        print("Done.")


if __name__ == "__main__":
    main()
