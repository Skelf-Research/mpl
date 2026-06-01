---
title: Contributing
---

# Contributing

Thank you for your interest in contributing to MPL! This guide covers everything you need to get started: setting up your development environment, building the project, running tests, and submitting pull requests.

## Development Setup

MPL is a Rust workspace with Python and TypeScript SDKs. Depending on which component you want to work on, you will need different tools installed.

### Prerequisites

| Component | Requirements |
|-----------|-------------|
| **Core + Proxy** (Rust) | Rust 1.75+ (`rustup` recommended) |
| **Python SDK** | Python 3.10+, `maturin` (`pip install maturin`) |
| **TypeScript SDK** | Node.js 18+, npm |
| **Integration Tests** | Docker |

### Installing Prerequisites

=== "Rust"

    ```bash
    # Install Rust via rustup
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

    # Verify installation (requires 1.75+)
    rustc --version
    cargo --version
    ```

=== "Python"

    ```bash
    # Ensure Python 3.10+
    python3 --version

    # Install maturin for building PyO3 bindings
    pip install maturin
    ```

=== "TypeScript"

    ```bash
    # Ensure Node.js 18+
    node --version

    # npm comes with Node.js
    npm --version
    ```

=== "Docker"

    ```bash
    # Install Docker for integration tests
    # See https://docs.docker.com/get-docker/
    docker --version
    ```

## Building from Source

### Clone the Repository

```bash
git clone https://github.com/Skelf-Research/mpl.git
cd mpl
```

### Build the Rust Workspace

The Rust workspace includes the core protocol implementation, sidecar proxy, and registry.

```bash
# Build all crates in the workspace
cargo build

# Build in release mode
cargo build --release
```

### Build the Python SDK

The Python SDK uses PyO3 bindings built with `maturin`.

```bash
cd python

# Create a virtual environment (recommended)
python3 -m venv .venv
source .venv/bin/activate

# Build and install in development mode
maturin develop

# Verify the installation
python -c "import mpl; print(mpl.__version__)"
```

### Build the TypeScript SDK

```bash
cd typescript

# Install dependencies
npm install

# Build the SDK
npm run build
```

## Running Tests

### Rust Tests

The Rust workspace contains 144 tests across all crates.

```bash
# Run all workspace tests
cargo test

# Run tests for a specific crate
cargo test -p mpl-core
cargo test -p mpl-proxy
cargo test -p mpl-registry

# Run tests with output
cargo test -- --nocapture

# Run a specific test
cargo test test_name
```

### Python Tests

```bash
cd python

# Activate virtual environment
source .venv/bin/activate

# Run the test suite
pytest

# Run with verbose output
pytest -v

# Run a specific test file
pytest tests/test_envelope.py
```

### TypeScript Tests

```bash
cd typescript

# Run the test suite
npm test

# Run tests in watch mode (during development)
npm run test:watch
```

### Integration Tests

Integration tests require Docker to be running.

```bash
# Run integration tests
cargo test --features integration

# Or use Docker Compose for the full test environment
docker compose -f docker-compose.test.yml up --abort-on-container-exit
```

## Code Structure

Understanding the project layout helps you find where to make changes.

```
mpl/
├── crates/                  # Rust workspace crates
│   ├── mpl-core/            # Core protocol implementation
│   ├── mpl-proxy/           # Sidecar proxy
│   ├── mpl-registry/        # Registry service
│   ├── mpl-policy/          # Policy engine
│   └── mpl-metrics/         # QoM metrics implementation
├── python/                  # Python SDK (PyO3 bindings)
│   ├── src/                 # Rust source for bindings
│   ├── mpl/                 # Python package
│   └── tests/               # Python tests
├── typescript/              # TypeScript SDK
│   ├── src/                 # TypeScript source
│   └── tests/               # TypeScript tests
├── registry/                # Registry data and SType definitions
│   ├── stypes/              # Pre-seeded SType schemas
│   └── schemas/             # JSON Schema definitions
└── documentation/           # MkDocs documentation site
    ├── docs/                # Markdown source files
    └── mkdocs.yml           # MkDocs configuration
```

## Pull Request Workflow

### 1. Fork and Clone

```bash
# Fork on GitHub, then clone your fork
git clone https://github.com/YOUR_USERNAME/mpl.git
cd mpl
git remote add upstream https://github.com/Skelf-Research/mpl.git
```

### 2. Create a Branch

```bash
# Sync with upstream
git fetch upstream
git checkout main
git merge upstream/main

# Create a feature branch
git checkout -b feature/your-feature-name
```

### 3. Make Your Changes

- Write code following the style guidelines below
- Add tests for new functionality
- Update documentation if needed

### 4. Commit Your Changes

Follow the commit message conventions described below.

```bash
git add .
git commit -m "feat(core): add envelope validation for nested payloads"
```

### 5. Push and Create PR

```bash
git push origin feature/your-feature-name
```

Then open a pull request on GitHub against the `main` branch.

### PR Checklist

Before submitting, ensure:

- [ ] All tests pass (`cargo test`, `pytest`, `npm test`)
- [ ] Code follows the style guidelines
- [ ] New code has appropriate test coverage
- [ ] Documentation is updated if needed
- [ ] Commit messages follow conventions

## Commit Message Conventions

MPL uses [Conventional Commits](https://www.conventionalcommits.org/) format:

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

### Types

| Type | Description |
|------|-------------|
| `feat` | A new feature |
| `fix` | A bug fix |
| `docs` | Documentation only changes |
| `style` | Formatting, missing semicolons, etc. |
| `refactor` | Code change that neither fixes a bug nor adds a feature |
| `perf` | Performance improvement |
| `test` | Adding or updating tests |
| `chore` | Build process or auxiliary tool changes |

### Scopes

| Scope | Component |
|-------|-----------|
| `core` | mpl-core crate |
| `proxy` | mpl-proxy crate |
| `registry` | mpl-registry crate |
| `policy` | mpl-policy crate |
| `python` | Python SDK |
| `typescript` | TypeScript SDK |
| `docs` | Documentation |

### Examples

```
feat(core): add support for multi-hop envelope chains
fix(proxy): resolve connection timeout on high-latency networks
docs(registry): add SType authoring guide
test(python): add integration tests for envelope signing
chore(typescript): update dependencies to latest versions
```

## Adding New STypes to the Registry

STypes (Semantic Types) are the core schema definitions in MPL. When adding a new SType:

### Namespace Rules

- Use reverse-domain notation: `org.example.category.name`
- Official MPL types use the `io.mpl.*` namespace
- Community types should use your organization's namespace
- Names must be lowercase with dots as separators

### Schema Requirements

Every SType must include:

1. **Schema definition** (JSON Schema format)
2. **Metadata** (name, version, description, namespace)
3. **At least one example** payload
4. **Negative test cases** (payloads that should fail validation)

### SType File Structure

```
registry/stypes/your-namespace/
├── schema.json          # JSON Schema definition
├── metadata.json        # SType metadata
├── examples/
│   ├── valid_01.json    # Valid example payloads
│   └── valid_02.json
└── negative/
    ├── missing_field.json    # Should fail: missing required field
    └── wrong_type.json       # Should fail: incorrect field type
```

### Example SType Definition

```json title="schema.json"
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "WeatherForecast",
  "description": "Weather forecast data for a specific location",
  "type": "object",
  "required": ["location", "temperature", "unit", "timestamp"],
  "properties": {
    "location": { "type": "string" },
    "temperature": { "type": "number" },
    "unit": { "enum": ["celsius", "fahrenheit"] },
    "timestamp": { "type": "string", "format": "date-time" }
  },
  "additionalProperties": false
}
```

### Negative Tests

Always include negative test cases that validate rejection of malformed payloads:

```json title="negative/missing_field.json"
{
  "_description": "Missing required 'temperature' field",
  "location": "London",
  "unit": "celsius",
  "timestamp": "2025-01-01T00:00:00Z"
}
```

```json title="negative/wrong_type.json"
{
  "_description": "Temperature should be a number, not a string",
  "location": "London",
  "temperature": "twenty",
  "unit": "celsius",
  "timestamp": "2025-01-01T00:00:00Z"
}
```

## Documentation Contributions

The documentation site is built with [MkDocs](https://www.mkdocs.org/) using the [Material for MkDocs](https://squidfunk.github.io/mkdocs-material/) theme.

### Local Development

```bash
cd documentation

# Install dependencies
pip install mkdocs-material

# Serve locally with hot-reload
mkdocs serve

# Build the static site
mkdocs build
```

### Documentation Structure

```
documentation/docs/
├── index.md              # Home page
├── overview/             # Project overview
├── getting-started/      # Quick start guides
├── concepts/             # Core concepts
├── guides/               # Tutorials and how-tos
├── reference/            # API reference
├── security/             # Security documentation
└── community/            # Community resources
```

### Writing Guidelines

- Use clear, concise language
- Include code examples where appropriate
- Use admonitions for warnings, tips, and notes
- Add diagrams for complex concepts (Mermaid is supported)
- Test all code examples before submitting

## Code Style

### Rust

MPL uses `rustfmt` for consistent formatting.

```bash
# Format all Rust code
cargo fmt

# Check formatting without modifying files
cargo fmt --check

# Run clippy for lints
cargo clippy -- -D warnings
```

### Python

MPL uses `black` for Python formatting.

```bash
cd python

# Format all Python code
black .

# Check formatting without modifying files
black --check .

# Run type checking
mypy mpl/
```

### TypeScript

MPL uses `eslint` for TypeScript linting and formatting.

```bash
cd typescript

# Lint all TypeScript code
npx eslint src/

# Fix auto-fixable issues
npx eslint src/ --fix

# Run prettier for formatting
npx prettier --write src/
```

## Getting Help

If you have questions about contributing:

- Open a [Discussion](https://github.com/Skelf-Research/mpl/discussions) on GitHub
- Check existing [Issues](https://github.com/Skelf-Research/mpl/issues) for related topics
- Review the [documentation](../index.md) for context on the project

We appreciate all contributions, from fixing typos to implementing major features. Thank you for helping improve MPL!
