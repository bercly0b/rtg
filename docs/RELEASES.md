# Releases

How RTG releases are cut, what the pipeline produces, and the known caveats.

## Overview

A release is triggered by pushing a `v*` git tag (e.g. `v0.2.0`). The
[`release.yml`](../.github/workflows/release.yml) workflow then:

1. **Verifies** that `Cargo.toml` version matches the tag (fails the run otherwise).
2. **Builds** `rtg` in `--release` mode for four targets:
   - `aarch64-apple-darwin` (macOS Apple Silicon)
   - `x86_64-apple-darwin` (macOS Intel, cross-compiled from the ARM runner)
   - `x86_64-unknown-linux-gnu` (Linux x86_64)
   - `aarch64-unknown-linux-gnu` (Linux ARM64 — Raspberry Pi, AWS Graviton, Apple Silicon Linux VMs)
3. **Fixes the rpath** so the binary finds `libtdjson` next to itself
   (`@executable_path/../lib` on macOS, `$ORIGIN/../lib` on Linux).
4. **Packages** each target into `rtg-<version>-<target>.tar.gz` containing:
   ```
   rtg-<version>-<target>/
   ├── bin/rtg
   ├── lib/libtdjson.<version>.{dylib,so}
   └── install.sh
   ```
5. **Publishes** a GitHub Release with all archives plus a `SHA256SUMS` file
   and auto-generated release notes from the commit log.

## Cutting a release

Use the `just` recipe from a clean `main`:

```bash
just release 0.2.0
```

The recipe:
1. Checks the working tree is clean and HEAD is on `main`.
2. Bumps `version = "0.2.0"` in `Cargo.toml` and refreshes `Cargo.lock`
   via `cargo check`.
3. Commits as `release: v0.2.0`.
4. Tags `v0.2.0`.
5. Pushes the commit and the tag to `origin`.

Then watch the run in [Actions](https://github.com/bercly0b/rtg/actions). A
full release takes ~10–15 minutes.

## Who can release

Tag creation is restricted via a **tag ruleset** (`Settings → Rules → Rulesets`)
matching pattern `v*` with `Restrict creations`, `Restrict updates`, and
`Restrict deletions` enabled. Only `Repository admin` is on the bypass list,
so only admins can create, move, or delete release tags.
