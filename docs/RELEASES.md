# Releases

How RTG releases are cut and what is not obvious from the release config.

## Overview

Releases are built and published by [cargo-dist](https://axodotdev.github.io/cargo-dist/)
([ADR 0002](adr/0002-release-through-cargo-dist.md)). The targets, installers
and publish jobs are set in [`dist-workspace.toml`](../dist-workspace.toml) and
in `[profile.dist]` of `Cargo.toml`.

Pushing a `v*` tag publishes a GitHub Release with an archive per target, the
release installer `rtg-tui-installer.sh` and the Homebrew formula `rtg.rb`,
then publishes the formula to the Homebrew tap and the package to crates.io.
Every pull request runs the same build without publishing, so a broken release
build shows up before the tag.

TDLib, OpenSSL and zlib are linked into the binary statically; only libc++
comes from the system.

## Names

The package is `rtg-tui`, because `rtg` was taken on crates.io. The binary and
the Homebrew formula are `rtg`:

- `cargo install --locked rtg-tui`
- `brew install bercly0b/rtg/rtg`, from the tap
  [`bercly0b/homebrew-rtg`](https://github.com/bercly0b/homebrew-rtg)

`exclude` in `Cargo.toml` keeps `docs/assets/` out of the package: crates.io
rejects packages over 10 MB, and the README demo GIF takes most of that.

## Release installer

`rtg-tui-installer.sh` installs `rtg` into `~/.cargo/bin`
(`RTG_TUI_INSTALL_DIR=<dir>` installs into `<dir>/bin`) and writes an install
receipt, `~/.config/rtg-tui/rtg-tui-receipt.json` (under `$XDG_CONFIG_HOME` if
set). The receipt tells this install channel apart from `cargo install`, which
uses the same directory, and is what [self-update](features/self-update.md)
reads.

An RTG installed with Homebrew does not update itself, as Homebrew requires:
`rtg update` and the status bar point at `brew upgrade rtg`
([install channel](features/update-check.md#install-channel)).

## Publish tokens

| Job | Secret of this repository | What it is |
| --- | --- | --- |
| `publish-homebrew-formula` | `HOMEBREW_TAP_TOKEN` | a token with write access to the tap's contents |
| [`publish-crates.yml`](../.github/workflows/publish-crates.yml) | `CARGO_REGISTRY_TOKEN` | a crates.io token with the `publish-update` scope for `rtg-tui` |

If a token has expired, its job fails after the GitHub Release is already out:
renew the token, update the secret and re-run the failed job.

## Changing the release build

[`release.yml`](../.github/workflows/release.yml) is generated and is not
edited by hand. Edit `dist-workspace.toml`, run `dist generate` and commit
both; `dist` fails a run whose workflow is out of date with the config.

`dist` is pinned in [`mise.toml`](../mise.toml) to the same version as
`cargo-dist-version` in `dist-workspace.toml`. To upgrade it, bump the pin and
run `dist init`.

## Changelog

[`CHANGELOG.md`](../CHANGELOG.md) lists the user-visible changes of each
release. Between releases the entries wait in `changelog.d/` as fragments, one
file per pull request (`changelog.d/rtg-118.md`), so parallel branches never
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

At release, the fragments move into a new `## X.Y.Z - YYYY-MM-DD` section at
the top of `CHANGELOG.md`
([`scripts/release-changelog.sh`](../scripts/release-changelog.sh)). dist puts
that section into the release body and uses its heading as the release title.
dist silently skips a version that has no section, which is why the release
task refuses to run without fragments.

## Cutting a release

From a clean `main`:

```bash
mise run release 0.2.0
```

The task moves the changelog fragments, bumps the version, commits, tags and
pushes (see `[tasks.release]` in [`mise.toml`](../mise.toml)). Then watch the
run in [Actions](https://github.com/bercly0b/rtg/actions); a full release takes
~10–15 minutes.

A version with a pre-release suffix (`mise run release 0.2.0-rc.1`) is
published as a GitHub pre-release and reaches neither the Homebrew tap nor
crates.io. `releases/latest` keeps pointing at the last stable release, so try
it with its own installer: `releases/download/v0.2.0-rc.1/rtg-tui-installer.sh`.

Only repository admins can create, move or delete `v*` tags: a tag ruleset
(`Settings → Rules → Rulesets`) restricts all three.

## Known caveats

- **The release binaries are not run in CI.** dist offers no hook after its
  build step, so `tests/static_tdlib.rs` (no `libtdjson` at runtime) runs in
  the quality gate on Linux instead of against each release binary.
