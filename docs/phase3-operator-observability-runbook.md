# Phase 3 — Operator Observability & Troubleshooting Runbook

> Scope: real Telegram auth/login flows (phone/code/2FA), startup session checks, reconnect behavior.

## 1) Where to observe runtime state

### CLI/stdout during guided auth

Guided auth prints status snapshots in a stable format:

```text
status[<action>]: auth=<AUTH_*> connectivity=<CONNECTIVITY_*> last_error=<ERROR_CODE|none>
```

Actions currently emitted:
- `start`
- `code`
- `password`
- `session-persist`

### Structured logs (`tracing`)

Set log level with either:
- `config.toml` → `[logging] level = "info"|"debug"|...`
- or `RUST_LOG` env var (takes precedence in `tracing_subscriber`)

Example:

```bash
RUST_LOG=info cargo run -- run
```

## 2) Canonical status labels

Auth:
- `AUTH_NOT_STARTED`
- `AUTH_IN_PROGRESS`
- `AUTH_REQUIRES_2FA`
- `AUTH_SUCCESS`
- `AUTH_TRANSIENT_FAILURE`
- `AUTH_FATAL_FAILURE`

Connectivity:
- `CONNECTIVITY_UNKNOWN`
- `CONNECTIVITY_OK`
- `CONNECTIVITY_DEGRADED`
- `CONNECTIVITY_UNAVAILABLE`

Reference: [`docs/auth-connectivity-status-model.md`](auth-connectivity-status-model.md).

## 3) Key operator-facing error/warning codes

Auth flow:
- `AUTH_INVALID_PHONE`
- `AUTH_INVALID_CODE`
- `AUTH_WRONG_2FA`
- `AUTH_TIMEOUT`
- `AUTH_FLOOD_WAIT`
- `AUTH_BACKEND_UNAVAILABLE`
- `AUTH_SESSION_PERSIST_FAILED`
- `AUTH_SESSION_MISSING`
- `AUTH_SESSION_BROKEN`
- `AUTH_SESSION_REVOKED`
- `AUTH_SESSION_POLICY_INVALID`

Startup/runtime warnings:
- `AUTH_PROBE_TIMEOUT_FALLBACK`
- `AUTH_PROBE_NETWORK_FALLBACK`
- `AUTH_TUI_BOOTSTRAP_FAILED`
- `TELEGRAM_CONNECTIVITY_MONITOR_START_FAILED`
- `TELEGRAM_CONNECTIVITY_MONITOR_SHUTDOWN_FAILED`

## 4) Troubleshooting matrix

### `AUTH_BACKEND_UNAVAILABLE`
- Meaning: Telegram backend is not configured or failed to initialize.
- Check:
  1. `RTG_TELEGRAM_API_ID` / `RTG_TELEGRAM_API_HASH` are present and correct.
  2. No malformed values in `config.toml`.
  3. Outbound network to Telegram is available.
- Action: fix config/network and retry login.

### `AUTH_INVALID_CODE`
- Meaning: OTP code is wrong/expired.
- Action: request a fresh code and retry within remaining attempts.

### `AUTH_WRONG_2FA`
- Meaning: account requires password and provided password is wrong.
- Action: retry with correct 2FA password.

### `AUTH_SESSION_REVOKED` or `AUTH_SESSION_POLICY_INVALID`
- Meaning: persisted session can no longer be trusted.
- Action: run guided auth again; successful auth clears local invalid-policy marker.

### `AUTH_PROBE_*_FALLBACK`
- Meaning: remote probe failed (timeout/network), RTG temporarily trusts local session validity.
- Action: continue operation, but inspect network health and monitor subsequent auth/connectivity transitions.

### `AUTH_TUI_BOOTSTRAP_FAILED`
- Meaning: auth succeeded and session was saved, but TUI failed to start in that run.
- Action: restart RTG; do not re-enter credentials immediately.

## 5) Quick operator commands

Run app with verbose logs:

```bash
RUST_LOG=debug cargo run -- run
```

Logout/reset to clean disconnected state:

```bash
cargo run -- logout
```

Re-check quality baseline after incident fix:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo check
```
