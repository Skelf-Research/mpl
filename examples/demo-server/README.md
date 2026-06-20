# Demo MCP Server

A minimal Python HTTP server used as a counterparty for testing the **mpl-proxy** in front of an MCP-style backend. It exposes a few simple endpoints, accepts arbitrary JSON, and stores requests in-memory so you can verify the proxy's typed-envelope, schema-fidelity, and policy behavior end-to-end without standing up a real tool stack.

## Endpoints

```
GET  /health        -> {"status":"healthy","service":"demo-mcp-server"}
GET  /capabilities  -> server name, version, supported methods
POST /events        -> create an event (stored in-memory)
POST /messages      -> append a message (stored in-memory)
```

The server is **not** a production MCP implementation — it is the smallest thing the proxy can talk to.

## Run it

```bash
python3 server.py            # listens on http://localhost:8080
```

Then put `mpl-proxy` in front of it:

```bash
cargo run -p mplx -- proxy --upstream http://localhost:8080 --listen 0.0.0.0:9443
```

Now any client targeting `http://localhost:9443` will be routed through the proxy: the proxy validates the envelope against the SType schemas in `../../registry/`, computes QoM, applies policies, and forwards to the demo server.

## Useful for

- Verifying that schema rejection short-circuits before the upstream is hit.
- Watching the QoM report attach to round-trips with no real backend dependencies.
- Reproducing the conformance tests in `crates/mpl-proxy/tests/proxy_e2e_test.rs` locally.
