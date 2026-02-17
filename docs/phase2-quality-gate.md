# Phase 2 — Quality Gate Execution Contract

This document is the **source of truth** for RTG quality gate execution in both local development and CI.

## Canonical gate sequence

Run gates in this strict order:

1. `cargo fmt --check`
2. `cargo clippy -- -D warnings`
3. `cargo test`
4. `cargo check`

Rationale for order:
- Fail fast on formatting/lint before heavier checks.
- Keep CI and local runs deterministic and reproducible.

## Rust environment bootstrap (local + CI)

Before running gates, the Rust environment must be bootstrapped with:

1. Stable toolchain installed.
2. Components installed: `rustfmt`, `clippy`.
3. Repository checkout completed.

Reference bootstrap commands (local):

```bash
rustup toolchain install stable
rustup component add rustfmt clippy
```

## Evidence artifacts (required in PR)

Each PR should provide quality gate evidence that maps to the canonical sequence.

Minimum evidence skeleton:

- Gate execution context:
  - commit SHA
  - execution environment (`local` and/or CI run URL)
- Ordered gate results:
  - `fmt`: pass/fail
  - `clippy`: pass/fail
  - `test`: pass/fail
  - `check`: pass/fail
- If any gate failed:
  - short root cause note
  - follow-up action/status

Recommended compact format in PR description:

```text
Quality gate evidence
- SHA: <commit>
- Env: <local/CI link>
- fmt: PASS
- clippy: PASS
- test: PASS
- check: PASS
```

## Pass / fail criteria

A quality gate run is **PASS** only when all conditions below are true:

- all four gates executed in canonical order
- each gate exited with status code `0`
- no skipped steps in the sequence

A quality gate run is **FAIL** if any condition below is true:

- at least one gate exits non-zero
- sequence is modified or partially skipped
- evidence is missing or cannot be mapped to the canonical sequence

## CI alignment contract

`.github/workflows/quality-gate.yml` must preserve the same gate names/order semantics as this document.
Any future workflow update must first update this document, then align workflow steps accordingly.
