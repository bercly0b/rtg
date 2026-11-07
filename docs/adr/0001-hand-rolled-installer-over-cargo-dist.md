---
status: superseded by ADR-0002
---

# Hand-rolled release installer instead of cargo-dist

RTG ships through three install channels (release installer, Homebrew tap, crates.io) and needs an update check plus self-update for the release installer. cargo-dist offers all of this out of the box, but we write our own `install.sh` and `rtg update` on top of the existing `release.yml`, following cargo-dist's conventions: `RTG_INSTALL_DIR`, then `$XDG_BIN_HOME`, then `~/.local/bin`; an install receipt in `~/.config/rtg/`; self-update by re-running the new release's installer into the current binary's directory.

## Considered Options

- **cargo-dist**: rejected. It replaces `release.yml` with a generated workflow, so our custom build steps (libc++ on Linux, the `ubuntu-22.04` runner for an older glibc baseline, cross-building x86_64 macOS on an ARM runner) would have to be re-expressed through its config, and it adds `axoupdater` and its dependencies to RTG. While RTG links `libtdjson` dynamically it cannot ship RTG at all: its installer and Homebrew formula install only workspace binaries and cdylibs, so a library from a build script's `OUT_DIR` never reaches `lib/` (open upstream: axodotdev/cargo-dist#542, #1233).

## Consequences

The installer script and the self-update path are ours to maintain. Reconsider cargo-dist if TDLib becomes statically linked and the custom build steps go away.
