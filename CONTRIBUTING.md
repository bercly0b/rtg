# Contributing to RTG

Thanks for contributing.

## Development setup

1. Install stable Rust toolchain.
2. Clone the repository.
3. Optionally copy `config.example.toml` to `config.toml`.

## Local quality gate (required before PR)

Source of truth: [`docs/phase2-quality-gate.md`](docs/phase2-quality-gate.md).

Run the canonical sequence:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo check
```

When opening a PR, include quality gate evidence using the artifact skeleton defined in the source-of-truth document.

## Pull request expectations

- Keep PR scope focused on one task.
- Include tests for new or changed logic when feasible.
- Ensure formatting/lint/tests are green.

### Review anti-loop policy

To avoid endless review loops:
- Fix **CRITICAL** issues in the current PR.
- Track **MAJOR/MINOR** follow-ups in `RTG_REVIEW_BACKLOG.md`.
- Do not expand PR scope with non-critical refactors.

## Architecture notes

Keep module responsibilities clear:
- `ui` for terminal presentation and input
- `domain` for core models/events/state
- `usecases` for orchestration
- `infra` for adapters/config/logging
- `telegram` for integration boundary

Avoid mixing business rules with infrastructure concerns in the same module.
