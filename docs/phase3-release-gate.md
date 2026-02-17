# Phase 3 — Release Gate Checklist (PR-10)

This checklist defines the phase-exit gate for real Telegram auth/connectivity flows.

## 1) Mandatory test scenarios (E2E at repository test layer)

The following scenarios must pass:

1. Happy path (no 2FA)
   - `e2e_happy_path_without_2fa_authenticates_and_persists_session`
2. Wrong code (recoverable retry)
   - `e2e_wrong_code_retries_then_succeeds`
3. 2FA required
   - `e2e_2fa_required_authenticates_after_password_step`
4. Wrong password (2FA retries exhausted)
   - `e2e_wrong_password_exhausts_retries_and_exits_with_guidance`
5. Restart/reconnect
   - `e2e_restart_reconnect_reuses_persisted_session`

## 2) Exact release-gate commands

Run in this strict order from repository root:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo check
```

Optional targeted run for quick validation of listed E2E scenarios:

```bash
cargo test e2e_
```

## 3) Pass criteria

Release gate is **PASS** only if:
- all four canonical quality commands exit with code `0`;
- all required E2E scenarios listed above are green in `cargo test` output;
- operator runbook is present and up to date:
  - [`docs/phase3-operator-observability-runbook.md`](phase3-operator-observability-runbook.md)

Release gate is **FAIL** if any command fails, any required E2E scenario fails/missing, or runbook is outdated.

## 4) PR evidence template

Use this compact evidence block in PR body:

```text
Phase 3 release gate evidence
- SHA: <commit>
- fmt --check: PASS/FAIL
- clippy -D warnings: PASS/FAIL
- test: PASS/FAIL
- check: PASS/FAIL
- Required E2E scenarios: PASS/FAIL (list failures if any)
- Runbook updated: YES/NO
```
