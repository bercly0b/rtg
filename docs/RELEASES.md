# Releases

How RTG releases are cut, what the pipeline produces, and the known caveats.

## Overview

Releases are built and published by [cargo-dist](https://axodotdev.github.io/cargo-dist/)
([ADR 0002](adr/0002-release-through-cargo-dist.md)). Its config lives in
[`dist-workspace.toml`](../dist-workspace.toml) and in the `[profile.dist]`
section of `Cargo.toml`. The [`release.yml`](../.github/workflows/release.yml)
workflow is generated from that config and is not edited by hand.

A release is triggered by pushing a `v*` git tag (e.g. `v0.2.0`). The workflow
then:

1. **Plans** the release. `dist` fails the run if the tag does not match the
   `Cargo.toml` version or if `release.yml` is out of date with the config.
2. **Builds** `rtg` with the `dist` profile (`release` plus thin LTO) for four
   targets:
   - `aarch64-apple-darwin` (macOS Apple Silicon)
   - `x86_64-apple-darwin` (macOS Intel, cross-compiled on the ARM runner)
   - `x86_64-unknown-linux-gnu` (Linux x86_64)
   - `aarch64-unknown-linux-gnu` (Linux ARM64 — Raspberry Pi, AWS Graviton, Apple Silicon Linux VMs)
   TDLib, OpenSSL and zlib are linked into the binary statically (the
   `static` feature of `tdlib-rs`); only libc++ comes from the system. The
   build log prints the dynamic libraries each binary links against.
3. **Packages** each target into `rtg-tui-<target>.tar.xz` with a `.sha256`
   next to it, and builds the release installer `rtg-tui-installer.sh`, a
   combined `sha256.sum` and a source tarball.
4. **Publishes** a GitHub Release with all of the above. dist writes the
   release body: the version's section of [`CHANGELOG.md`](../CHANGELOG.md),
   the install command and a table of downloads.

Every pull request runs steps 1–3 (`pr-run-mode = "upload"`), so a broken
release build shows up before the tag; the artifacts are attached to the run
and nothing is published.

## Release installer

`rtg-tui-installer.sh` downloads the archive for the user's platform, installs
`rtg` into `~/.cargo/bin` (`RTG_TUI_INSTALL_DIR=<dir>` installs into
`<dir>/bin`) and adds that directory to `PATH` through the shell profiles. It
also writes an install receipt, `~/.config/rtg-tui/rtg-tui-receipt.json`
(under `$XDG_CONFIG_HOME` if set): the receipt tells this install channel apart
from `cargo install`, which uses the same directory, and is what self-update
reads.

The installer in `releases/latest/download/` installs the latest release; the
installer of any other release (`releases/download/vX.Y.Z/rtg-tui-installer.sh`)
installs that release.

## Changing the release build

Edit `dist-workspace.toml`, then regenerate the workflow and commit both:

```bash
dist generate
```

`dist` is pinned in [`mise.toml`](../mise.toml) to the same version as
`cargo-dist-version` in `dist-workspace.toml`. To upgrade it, bump the pin and
run `dist init`, which updates the config and regenerates the workflow.

## Changelog

[`CHANGELOG.md`](../CHANGELOG.md) lists the user-visible changes of each
release. Between releases the entries wait in `changelog.d/` as fragments, one
file per pull request (`changelog.d/roz-197.md`), so parallel branches never
edit the same lines:

```markdown
- Archive chats from the chat list.
- Show the archive in the folder picker.
```

Write the items in English, for users. Fragments are joined into one flat list
in file-name order.

The `changelog` check ([`changelog.yml`](../.github/workflows/changelog.yml)) is
required for merging into `main`: it fails unless the pull request adds or edits
a fragment. A pull request without user-visible changes (docs, refactoring, CI,
tests) carries the `no-changelog` label instead; pull requests from Dependabot
pass without one.

At release, `mise run release` moves the fragments into a new
`## X.Y.Z - YYYY-MM-DD` section at the top of `CHANGELOG.md`
([`scripts/release-changelog.sh`](../scripts/release-changelog.sh)). dist puts
that section into the release body under *Release Notes* and uses its heading
as the release title. dist silently skips a version that has no section, which
is why the release task refuses to run without fragments.

## Cutting a release

Use the `mise` task from a clean `main`:

```bash
mise run release 0.2.0
```

The task:
1. Checks the working tree is clean and HEAD is on `main`.
2. Moves the fragments from `changelog.d/` into a `## 0.2.0 - <today>`
   section of `CHANGELOG.md`; fails if there are none.
3. Bumps `version = "0.2.0"` in `Cargo.toml` and refreshes `Cargo.lock`
   via `cargo check`.
4. Commits as `release: v0.2.0`.
5. Tags `v0.2.0`.
6. Pushes the commit and the tag to `origin`.

Then watch the run in [Actions](https://github.com/bercly0b/rtg/actions). A
full release takes ~10–15 minutes.

A version with a pre-release suffix (`mise run release 0.2.0-rc.1`) is
published as a GitHub pre-release. `releases/latest` keeps pointing at the last
stable release, so try it with its own installer:
`releases/download/v0.2.0-rc.1/rtg-tui-installer.sh`.

## Known caveats

- **The release binaries are not run in CI.** dist offers no hook after its
  build step, so `tests/static_tdlib.rs` (no `libtdjson` at runtime) runs in
  the quality gate on Linux instead of against each release binary.

## Who can release

Tag creation is restricted via a **tag ruleset** (`Settings → Rules → Rulesets`)
matching pattern `v*` with `Restrict creations`, `Restrict updates`, and
`Restrict deletions` enabled. Only `Repository admin` is on the bypass list,
so only admins can create, move, or delete release tags.
