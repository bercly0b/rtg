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
