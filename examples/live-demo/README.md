# MPL Live Demo

A comprehensive live demonstration of MPL (Meaning Protocol Layer) working with real MCP servers and A2A agent-to-agent communication.

## Prerequisites

- [uv](https://docs.astral.sh/uv/) - Fast Python package manager
- [Rust/Cargo](https://rustup.rs/) - For building the MPL proxy

Install prerequisites:
```bash
# Install uv
curl -LsSf https://astral.sh/uv/install.sh | sh

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Quick Start

### Option 1: Using the Launcher Script (Recommended)

```bash
cd examples/live-demo

# Start the MCP server and MPL proxy
./run-demo.sh start

# Run the full demo
./run-demo.sh demo

# Run specific scenarios
./run-demo.sh demo mcp    # MCP tool invocation demo
./run-demo.sh demo a2a    # Agent-to-agent workflow
./run-demo.sh demo qom    # QoM profile demonstration

# Interactive mode
./run-demo.sh interactive

# Check service status
./run-demo.sh test

# View logs
./run-demo.sh logs

# Stop services
./run-demo.sh stop
```

### Option 2: Manual Setup

```bash
# Terminal 1: Start the MCP server
cd examples/live-demo
uv run mcp_server.py

# Terminal 2: Start the MPL proxy (from project root)
cargo run -p mpl-proxy --release -- \
    --listen 0.0.0.0:9443 \
    --upstream http://localhost:8080 \
    --registry ./registry

# Terminal 3: Run the demo
cd examples/live-demo
uv run demo.py
```

## Demo Scenarios

### 1. MCP Tool Invocation (`--scenario mcp`)

Demonstrates:
- Creating typed calendar events through the MPL proxy
- Schema validation success and failure cases
- QoM metric reporting

### 2. Multi-SType Validation (`--scenario validation`)

Tests multiple semantic types:
- `org.calendar.Event.v1` - Calendar events
- `org.communication.Message.v1` - Messages
- `org.agent.TaskPlan.v1` - Task plans
- `data.query.Query.v1` - Data queries

### 3. A2A Multi-Agent Workflow (`--scenario a2a`)

Shows agent-to-agent communication:
- Planner agent creates task plans
- Executor agent invokes tools
- QoM validation at each step
- Full audit trail

### 4. QoM Profile Comparison (`--scenario qom`)

Compares different QoM profiles:
- `qom-basic` - Development mode (SF = 1.0)
- `qom-strict-argcheck` - Production mode (SF = 1.0, IC >= 0.97)

## Features

- **Colorful Terminal Output**: Easy-to-read visualization
- **Flow Diagrams**: Shows message flow between components
- **Progress Bars**: Visual QoM metric display
- **Real-time Metrics**: Latency and validation results
- **Interactive Mode**: Explore MPL capabilities hands-on

## Architecture

```
+------------------+     +------------------+     +------------------+
|                  |     |                  |     |                  |
|   Demo Client    | --> |    MPL Proxy     | --> |   MCP Server     |
|   (demo.py)      |     | (cargo binary)   |     | (mcp_server.py)  |
|                  |     |   port 9443      |     |   port 8080      |
+------------------+     +------------------+     +------------------+
        |                        |
        |                        v
        |               +------------------+
        |               |    Registry      |
        |               | ../../registry/  |
        |               |  (SType schemas) |
        |               +------------------+
        v
  Terminal Output
  (visualization)
```

The MPL proxy is built from Rust source using `cargo build -p mpl-proxy --release`
and validates all requests against the SType schemas in the registry.

## Command Reference

```
uv run demo.py [OPTIONS]

Options:
  --scenario {all,mcp,a2a,validation,qom}
                        Which scenario to run (default: all)
  --proxy-url URL       MPL proxy URL (default: http://localhost:9443)
  --interactive         Run in interactive mode
  --no-color            Disable colored output
```

## Troubleshooting

### Services not responding

```bash
# Check if services are running
./run-demo.sh test

# View service logs
./run-demo.sh logs

# Restart services
./run-demo.sh stop
./run-demo.sh start
```

### Schema validation failing

Ensure the registry path is correct:
```bash
# The proxy should be started with --registry pointing to the registry folder
ls ../../registry/stypes/
```

### Port conflicts

```bash
# Check what's using the ports
lsof -i :8080
lsof -i :9443

# Kill processes if needed
kill -9 <PID>

# Or use the stop command
./run-demo.sh stop
```

### Build failures

```bash
# Make sure Rust is up to date
rustup update

# Clean and rebuild
cd ../..
cargo clean
cargo build -p mpl-proxy --release
```

## Next Steps

After running the demo:

1. **Explore the Registry**: Check `../../registry/stypes/` for available semantic types
2. **Try Custom STypes**: Create your own schemas in the registry
3. **Read the Docs**: See `../../docs/` for detailed documentation
