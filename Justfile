# ── Quality gate (mirrors CLAUDE.md rules) ──────────────────────────────────

test:
    cargo test

fmt:
    cargo fmt --check

clippy:
    cargo clippy --tests

check:
    cargo check

quality: fmt clippy test check

# ── Test coverage (requires cargo-llvm-cov) ─────────────────────────────────
#
# Install once:  cargo install cargo-llvm-cov
#
# The coverage targets exclude TDLib-dependent integration paths that
# cannot run without a live Telegram session.

coverage:
    cargo llvm-cov --no-fail-fast

coverage-html:
    cargo llvm-cov --no-fail-fast --html --open

coverage-lcov:
    cargo llvm-cov --no-fail-fast --lcov --output-path lcov.info

# ── Release ─────────────────────────────────────────────────────────────────
#
# Bump Cargo.toml version, commit, tag and push. CI picks up the tag and
# publishes a GitHub Release with prebuilt binaries.
#
# Usage:  just release 0.2.0

release version:
    @if [ -n "$(git status --porcelain)" ]; then echo "error: working tree not clean"; exit 1; fi
    @if [ "$(git rev-parse --abbrev-ref HEAD)" != "main" ]; then echo "error: must be on main branch"; exit 1; fi
    sed -i.bak 's/^version = ".*"/version = "{{version}}"/' Cargo.toml && rm Cargo.toml.bak
    cargo check
    git add Cargo.toml Cargo.lock
    git commit -m "release: v{{version}}"
    git tag "v{{version}}"
    git push origin main "v{{version}}"
