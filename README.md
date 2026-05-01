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

TDLib (Telegram Database Library) is downloaded automatically during build via the `download-tdlib` feature of `tdlib-rs`. No manual installation required.

> **Note:** First build may take longer as TDLib (~50MB) is downloaded. Subsequent builds use the cached version.

Supported platforms for automatic TDLib download:
- Linux x86_64 / arm64
- macOS Intel / Apple Silicon
- Windows x86_64 / arm64

### Linux runtime dependencies

The prebuilt `libtdjson.so` shipped via `download-tdlib` is linked against **libc++** (LLVM C++ runtime), not libstdc++. On a fresh Ubuntu/Debian install these libraries are missing and linking will fail with `libc++.so.1 ... not found` and undefined references in the `std::__1::` namespace.

Install the runtime libraries before building:

```bash
sudo apt install libc++1 libc++abi1 libunwind8
```

For other distributions install the equivalent libc++ runtime packages.

For manual TDLib installation or other platforms, see [TDLib build instructions](https://tdlib.github.io/td/build.html).

## Installation

```bash
git clone https://github.com/bercly0b/rtg.git
cd rtg
```

Copy the example config to the default location:

```bash
mkdir -p ~/.config/rtg
cp config.example.toml ~/.config/rtg/config.toml
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

### Telegram API credentials setup

1. Open <https://my.telegram.org> and sign in with your Telegram account.
2. Go to **API development tools** and create (or view) your application.
3. Copy your `api_id` and `api_hash`.
4. Add them to `~/.config/rtg/config.toml`:

```toml
[telegram]
api_id = 123456
api_hash = "your_api_hash_here"
```

5. Run RTG as usual (`cargo run`).

Security note:
- Never commit real credentials (`api_id`, `api_hash`) to git.
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
