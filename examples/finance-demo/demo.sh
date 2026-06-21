#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# ─── Check prerequisites ─────────────────────────────────────────────────────

if ! command -v uv &>/dev/null; then
    echo "Error: uv is not installed. Install it from https://docs.astral.sh/uv/"
    exit 1
fi

if ! command -v cargo &>/dev/null; then
    echo "Error: cargo is not installed. Install Rust from https://rustup.rs/"
    exit 1
fi

if [ -z "${OLLAMA_API_KEY:-}" ]; then
    echo "Error: OLLAMA_API_KEY is not set."
    echo "  Get one from https://ollama.com/settings/keys, then:"
    echo "    export OLLAMA_API_KEY=..."
    exit 1
fi

# ─── Build proxy ─────────────────────────────────────────────────────────────

if [ ! -f "$PROJECT_ROOT/target/release/mpl-proxy" ]; then
    echo "Building mpl-proxy (first run only)..."
    cargo build --release -p mpl-proxy --manifest-path "$PROJECT_ROOT/Cargo.toml"
fi

# ─── Run demo ─────────────────────────────────────────────────────────────────

cd "$SCRIPT_DIR"
uv run demo.py
