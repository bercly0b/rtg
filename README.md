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

Planning baseline (phase rebaseline + status model):
- [`docs/phase-roadmap-rebaseline.md`](docs/phase-roadmap-rebaseline.md)
- [`docs/auth-connectivity-status-model.md`](docs/auth-connectivity-status-model.md)

## Contributing

Please read:
- [`CONTRIBUTING.md`](CONTRIBUTING.md)
- [`docs/phase1-layer-data-flow.md`](docs/phase1-layer-data-flow.md)

For now, contributions should keep changes focused and aligned with existing layer boundaries.
