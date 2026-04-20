#!/bin/bash
#
# MPL Live Demo Launcher
#
# This script sets up and runs the MPL live demonstration using local binaries.
#

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color
BOLD='\033[1m'

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SERVER_PORT=8080
PROXY_PORT=9443

# Print banner
print_banner() {
    echo -e "${CYAN}${BOLD}"
    cat << 'EOF'

    __  _______  __       __    _                ____
   /  |/  / __ \/ /      / /   (_)   _____      / __ \___  ____ ___  ____
  / /|_/ / /_/ / /      / /   / / | / / _ \    / / / / _ \/ __ `__ \/ __ \
 / /  / / ____/ /___   / /___/ /| |/ /  __/   / /_/ /  __/ / / / / / /_/ /
/_/  /_/_/   /_____/  /_____/_/ |___/\___/   /_____/\___/_/ /_/ /_/\____/

         Meaning Protocol Layer - Live Demo
EOF
    echo -e "${NC}"
}

# Print status message
info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

# Check prerequisites
check_prerequisites() {
    info "Checking prerequisites..."

    # Check uv
    if ! command -v uv &> /dev/null; then
        error "uv is required but not installed"
        echo "  Install with: curl -LsSf https://astral.sh/uv/install.sh | sh"
        exit 1
    fi
    success "uv found: $(uv --version)"

    # Check cargo
    if ! command -v cargo &> /dev/null; then
        error "cargo is required but not installed"
        echo "  Install with: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        exit 1
    fi
    success "cargo found: $(cargo --version)"
}

# Build the MPL proxy
build_proxy() {
    info "Building MPL proxy..."
    cd "$PROJECT_ROOT"
    cargo build -p mpl-proxy --release 2>&1 | tail -5
    success "MPL proxy built"
}

# Start demo server
start_server() {
    info "Starting MCP demo server on port $SERVER_PORT..."

    # Check if port is already in use
    if lsof -Pi :$SERVER_PORT -sTCP:LISTEN -t >/dev/null 2>&1; then
        warn "Port $SERVER_PORT already in use"
        return 0
    fi

    cd "$SCRIPT_DIR"
    uv run mcp_server.py > /tmp/mpl-demo-server.log 2>&1 &
    echo $! > /tmp/mpl-demo-server.pid

    sleep 1
    if curl -sf "http://localhost:$SERVER_PORT/health" >/dev/null 2>&1; then
        success "MCP server started (PID: $(cat /tmp/mpl-demo-server.pid))"
    else
        error "Failed to start MCP server"
        cat /tmp/mpl-demo-server.log
        exit 1
    fi
}

# Start MPL proxy
start_proxy() {
    info "Starting MPL proxy on port $PROXY_PORT..."

    # Check if port is already in use
    if lsof -Pi :$PROXY_PORT -sTCP:LISTEN -t >/dev/null 2>&1; then
        warn "Port $PROXY_PORT already in use"
        return 0
    fi

    cd "$PROJECT_ROOT"

    # Run proxy with config
    RUST_LOG=info cargo run -p mpl-proxy --release -- \
        --listen "0.0.0.0:$PROXY_PORT" \
        --upstream "http://localhost:$SERVER_PORT" \
        --registry "$PROJECT_ROOT/registry" \
        > /tmp/mpl-proxy.log 2>&1 &
    echo $! > /tmp/mpl-proxy.pid

    # Wait for proxy to be ready
    for i in {1..10}; do
        if curl -sf "http://localhost:$PROXY_PORT/health" >/dev/null 2>&1; then
            success "MPL proxy started (PID: $(cat /tmp/mpl-proxy.pid))"
            return 0
        fi
        sleep 1
    done

    error "Failed to start MPL proxy"
    cat /tmp/mpl-proxy.log
    exit 1
}

# Stop services
stop_services() {
    info "Stopping services..."

    # Stop proxy
    if [ -f /tmp/mpl-proxy.pid ]; then
        PID=$(cat /tmp/mpl-proxy.pid)
        if kill -0 $PID 2>/dev/null; then
            kill $PID 2>/dev/null || true
            success "Stopped MPL proxy (PID: $PID)"
        fi
        rm -f /tmp/mpl-proxy.pid
    fi

    # Stop server
    if [ -f /tmp/mpl-demo-server.pid ]; then
        PID=$(cat /tmp/mpl-demo-server.pid)
        if kill -0 $PID 2>/dev/null; then
            kill $PID 2>/dev/null || true
            success "Stopped MCP server (PID: $PID)"
        fi
        rm -f /tmp/mpl-demo-server.pid
    fi
}

# Run the demo
run_demo() {
    local scenario=${1:-all}

    info "Running demo scenario: $scenario"

    cd "$SCRIPT_DIR"
    uv run demo.py --scenario "$scenario" --proxy-url "http://localhost:$PROXY_PORT"
}

# Run interactive mode
run_interactive() {
    info "Starting interactive mode..."

    cd "$SCRIPT_DIR"
    uv run demo.py --interactive --proxy-url "http://localhost:$PROXY_PORT"
}

# Quick test
quick_test() {
    info "Running connectivity test..."

    echo -n "  MCP Server ($SERVER_PORT): "
    if curl -sf "http://localhost:$SERVER_PORT/health" >/dev/null 2>&1; then
        echo -e "${GREEN}OK${NC}"
    else
        echo -e "${RED}FAIL${NC}"
    fi

    echo -n "  MPL Proxy ($PROXY_PORT): "
    if curl -sf "http://localhost:$PROXY_PORT/health" >/dev/null 2>&1; then
        echo -e "${GREEN}OK${NC}"
    else
        echo -e "${RED}FAIL${NC}"
    fi
}

# Print usage
usage() {
    echo "Usage: $0 [command] [options]"
    echo ""
    echo "Commands:"
    echo "  start             Start MCP server and MPL proxy"
    echo "  stop              Stop all services"
    echo "  demo [scenario]   Run the demo (scenarios: all, mcp, a2a, validation, qom)"
    echo "  interactive       Run in interactive mode"
    echo "  test              Quick connectivity test"
    echo "  logs              Show service logs"
    echo "  help              Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 start          # Start all services"
    echo "  $0 demo           # Run all scenarios"
    echo "  $0 demo mcp       # Run MCP scenario only"
    echo "  $0 demo a2a       # Run A2A scenario only"
    echo "  $0 interactive    # Interactive exploration"
    echo ""
}

# Show logs
show_logs() {
    echo -e "${CYAN}=== MCP Server Log ===${NC}"
    tail -20 /tmp/mpl-demo-server.log 2>/dev/null || echo "(no log)"
    echo ""
    echo -e "${CYAN}=== MPL Proxy Log ===${NC}"
    tail -20 /tmp/mpl-proxy.log 2>/dev/null || echo "(no log)"
}

# Main
main() {
    print_banner

    case "${1:-help}" in
        start)
            check_prerequisites
            build_proxy
            start_server
            start_proxy
            echo ""
            success "All services running!"
            echo ""
            echo "  MCP Server: http://localhost:$SERVER_PORT"
            echo "  MPL Proxy:  http://localhost:$PROXY_PORT"
            echo ""
            echo "Run the demo with: $0 demo"
            ;;
        stop)
            stop_services
            ;;
        demo)
            run_demo "${2:-all}"
            ;;
        interactive)
            run_interactive
            ;;
        test)
            quick_test
            ;;
        logs)
            show_logs
            ;;
        help|--help|-h)
            usage
            ;;
        *)
            error "Unknown command: $1"
            usage
            exit 1
            ;;
    esac
}

# Trap to clean up on exit (only for start command)
if [[ "${1:-}" == "start" ]]; then
    trap 'echo ""; info "Use ./run-demo.sh stop to stop services"' EXIT
fi

main "$@"
