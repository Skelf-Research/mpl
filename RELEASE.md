# Release process

How a new MPL release is cut. Single source of truth — keep CI/`release.yaml`
and this file in sync.

## Prerequisites

- Push access to `main`.
- Write access to repo for tag creation.
- A valid `CARGO_REGISTRY_TOKEN` repository secret (crates.io API token with
  publish rights for `mpl-protocol`, `mpl-proxy`, `mplx`, `mpl-registry-api`,
  `mpl-python`).

## Steps

1. **Verify CI is green on `main`.** All four gates: fmt, clippy
   (`-D warnings`), tests, audit. Don't proceed otherwise.

2. **Pick the next version.** Follow [SemVer](https://semver.org/):
   - `0.1.X` → bugfix, no public-API change, no on-wire change.
   - `0.X.0` → backward-compatible additions, public-API extensions.
   - `1.0.0` → first stable: lock the public API and the wire format.

3. **Bump the workspace version.** Edit `[workspace.package].version` in
   `Cargo.toml`, then `cargo build` so `Cargo.lock` updates.

4. **Update `CHANGELOG.md`.** Move `[Unreleased]` to a new
   `[X.Y.Z] - YYYY-MM-DD` section. Leave `[Unreleased]` empty for the next
   cycle.

5. **Commit and tag.**
   ```bash
   git add Cargo.toml Cargo.lock CHANGELOG.md
   git commit -m "release: vX.Y.Z"
   git tag vX.Y.Z
   git push origin main vX.Y.Z
   ```

6. **Wait for `release.yaml`.** It will:
   - re-verify all four CI gates,
   - `cargo publish` each crate in dep order with index-propagation pauses,
   - open a GitHub Release with auto-generated notes.

7. **Verify on crates.io** that the five crates show the new version. If
   something failed mid-publish, finish the remaining publishes manually:
   ```bash
   cargo publish -p <stuck-crate>
   ```
   then re-trigger the release notes.

## Why not `cargo-release` or `release-plz`

These tools work; we chose explicit so the steps are obvious from the
workflow file. If contributor count grows, revisit.
