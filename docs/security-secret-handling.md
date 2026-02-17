# Secret handling and safe logging (Phase 2 / Task 5)

This document defines mandatory guardrails for RTG authentication flows.

## Rules

1. **2FA password input must be no-echo** in terminal mode.
2. **Secrets must never be printed** to terminal output, logs, errors, or panic payloads.
3. **Backend transient failures must expose only non-secret guidance** with a safe error code.
4. **Untrusted error codes are sanitized** before being shown to users.

## Covered sensitive data

- Login codes (OTP / Telegram code)
- 2FA passwords
- Session tokens and token-like values
- Free-form backend messages that may contain secret fragments

## Current implementation

- `AuthTerminal::prompt_secret` uses hidden input for 2FA.
- Guided auth transient failures intentionally do **not** render backend message text.
- Panic hook is overridden to redact secret-like payload fragments.
- Redaction helpers live in `src/infra/secrets.rs` and are unit-tested.

## Test expectations

- Tests must verify that secret-like strings from backend errors are not present in terminal output.
- Redaction helpers must be covered with unit tests for both valid and invalid code paths.
