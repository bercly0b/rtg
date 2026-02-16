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

## Controls

In the current TUI shell:
- `q` — quit
- `Ctrl+C` — quit

## Development

Quality gate commands (local and CI baseline):

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo check
```

## Contributing

Please read:
- [`CONTRIBUTING.md`](CONTRIBUTING.md)
- [`docs/phase1-layer-data-flow.md`](docs/phase1-layer-data-flow.md)

For now, contributions should keep changes focused and aligned with existing layer boundaries.
