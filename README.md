# RTG

RTG is an early-stage Rust Telegram client focused on a terminal-first experience (CLI + TUI).

The codebase is intentionally structured around clear architectural boundaries to support iterative development without mixing responsibilities.

## Overview

Current scope includes:
- CLI entrypoint and bootstrap flow
- basic TUI shell/event loop skeleton
- domain/usecase/infra contracts with stub adapters
- configuration + logging baseline

Layer boundaries:
- `src/ui` — terminal rendering and input handling
- `src/domain` — domain entities and typed events/state
- `src/telegram` — Telegram integration boundary
- `src/usecases` — application orchestration
- `src/infra` — config/logging/storage/opener adapters

## Prerequisites

- Rust toolchain (stable), including:
  - `cargo`
  - `rustfmt` (usually via `rustup component add rustfmt`)
  - `clippy` (usually via `rustup component add clippy`)

## Installation

```bash
git clone https://github.com/bercly0b/rtg.git
cd rtg
```

Optional local configuration:

```bash
cp config.example.toml config.toml
```

## Build

```bash
cargo build
```

## Run

Default run mode:

```bash
cargo run
```

Explicit subcommand:

```bash
cargo run -- run
```

Custom config path:

```bash
cargo run -- --config ./config.toml
```

## Configuration

Example config is provided in `config.example.toml`:
- `[logging]` — tracing verbosity (`trace|debug|info|warn|error`)
- `[telegram]` — placeholder fields for Telegram API credentials

If `config.toml` is missing, the app falls back to built-in defaults.

### Telegram API credentials setup (recommended: `.env`)

1. Open <https://my.telegram.org> and sign in with your Telegram account.
2. Go to **API development tools** and create (or view) your application.
3. Copy your `api_id` and `api_hash`.
4. Create a local `.env` file in the project root:

```bash
cat > .env << 'EOF'
RTG_TELEGRAM_API_ID=123456
RTG_TELEGRAM_API_HASH=your_api_hash_here
EOF
```

5. Run RTG as usual (`cargo run`).

Environment variable names:
- `RTG_TELEGRAM_API_ID`
- `RTG_TELEGRAM_API_HASH`

Precedence rule (explicit):
- defaults < `config.toml` < environment variables.
- if both file and env are set, env values win.

Security note:
- Never commit real credentials (`api_id`, `api_hash`) to git.
- `.env` is ignored by default via `.gitignore`.
- Keep `config.example.toml` as placeholders only.

### Telegram session storage lifecycle

RTG persists Telegram auth session state to:
- `${XDG_CONFIG_HOME}/rtg/session/session.dat` (or `~/.config/rtg/session/session.dat` when `XDG_CONFIG_HOME` is not set)

Lifecycle:
- the session file is created (if missing) during Telegram backend bootstrap;
- on successful guided auth, the authorized Telegram session is saved to this file;
- on restart, RTG reuses this persisted session for startup/session validation;
- if a session is revoked by Telegram, RTG marks policy invalid at `session/session.policy.invalid` and forces re-authorization.

## Controls

In the current TUI shell:
- `q` — quit
- `Ctrl+C` — quit

## Development

### Reproducible quality gate

Run the canonical quality gate sequence:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo check
```

Source of truth (sequence, bootstrap, evidence, pass/fail):
- [`docs/phase2-quality-gate.md`](docs/phase2-quality-gate.md)
- [`docs/phase3-release-gate.md`](docs/phase3-release-gate.md)

Planning baseline (phase rebaseline + status model):
- [`docs/phase-roadmap-rebaseline.md`](docs/phase-roadmap-rebaseline.md)
- [`docs/auth-connectivity-status-model.md`](docs/auth-connectivity-status-model.md)
- [`docs/phase3-operator-observability-runbook.md`](docs/phase3-operator-observability-runbook.md)

## Contributing

Please read:
- [`CONTRIBUTING.md`](CONTRIBUTING.md)
- [`docs/phase1-layer-data-flow.md`](docs/phase1-layer-data-flow.md)

For now, contributions should keep changes focused and aligned with existing layer boundaries.
