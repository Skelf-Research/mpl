# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `mpl_core::util::expand_tilde(&str) -> PathBuf` shared utility with unit tests.
- 16 integration tests for `mpl-registry-api` covering every handler, the
  cache, the state path helpers, and the error → HTTP status mapping.
- `cargo-audit` step in CI plus `.cargo/audit.toml` with per-advisory rationale
  for transitive vulns we cannot patch from this repo.
- READMEs for `examples/demo-server/` and `examples/tutorials/`.
- New `mpl-bench` workspace crate as a research/eval harness (`publish = false`).
- `.github/workflows/release.yaml`: tag-triggered publish to crates.io after
  re-running every CI gate.
- `RELEASE.md`: documented release process.
- Criterion benchmarks for protocol hot paths (`semantic_hash`, `qom_compute`).
  Reference numbers on commodity hardware: small payload hash ~1.8 µs,
  medium payload hash ~39 µs, QoM compute over finance Transfer ~134 µs.

### Changed
- Bumped `pyo3` `0.22` → `0.29`. The full migration is in: `IntoPy`/`PyObject`/
  `Python::with_gil`/`PyDict::new_bound` are replaced with
  `IntoPyObjectExt::into_py_any` / `Py<PyAny>` / `Python::attach` / `PyDict::new`;
  `Clone`-deriving `#[pyclass]` types opt in to the future `FromPyObject`
  derive via the `from_py_object` attribute. This permanently closes
  `RUSTSEC-2026-0176` (PyList/PyTuple nth out-of-bounds) and
  `RUSTSEC-2026-0177` (PyCFunction Sync bound).
- Bumped `prometheus` `0.13` → `0.14` so we drop the vulnerable `protobuf 2.x`.
- `mpl-registry-api` dev-dep `reqwest` `0.11` → workspace `0.12`.
- `mpl-proxy` `data_dir` fallback now expands `~/` via
  `mpl_core::util::expand_tilde`. Previously the literal `Path::new("~/.mpl")`
  created an actual `~/.mpl/` subdirectory wherever the binary was invoked.
- Every non-test `.unwrap()` in the workspace replaced with
  `.expect("rationale")` so the panic-impossibility invariant is documented in
  code.
- README badges: stale hard-coded test count replaced with a live CI badge,
  added a `cargo-audit` badge.

### Fixed
- CI is green on every gate: `cargo fmt --check`,
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  `cargo test --workspace` (**215 passed, 0 failed**), `cargo audit`.

## [0.1.2] - prior

Versioned releases prior to this changelog were not annotated; see
`git log v0.1.2..HEAD` for the cumulative diff.
