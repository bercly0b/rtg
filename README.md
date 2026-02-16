# rtg

RTG is a Rust-based Telegram client project (CLI + TUI).

## Current bootstrap status

This repository now contains a minimal Rust workspace skeleton with explicit module boundaries.

## Module boundaries

- `src/ui` — presentation layer (CLI/TUI rendering and input handling)
- `src/domain` — domain entities and business rules
- `src/telegram` — Telegram API integration and update mapping
- `src/usecases` — application workflows orchestrating domain + integrations
- `src/infra` — infrastructure adapters (config, storage, OS/openers, logging)

The current code is intentionally minimal and focused on structure so the project can evolve incrementally without mixing responsibilities.

## Infra baseline (Task 2)

- Config loader in `src/infra/config`:
  - default values are provided in code
  - optional overrides are loaded from `config.toml` when file exists
- Tracing logging initialization in `src/infra/logging.rs`
- Typed app errors in `src/infra/error.rs` (`thiserror`) with `anyhow` at app boundary (`main`)
- Example configuration file: `config.example.toml`

## CLI + bootstrap (Task 3)

- CLI is implemented with `clap` (`src/cli.rs`)
- `rtg` defaults to `run` command, explicit `rtg run` is also supported
- Bootstrap pipeline is isolated in `src/usecases/bootstrap.rs`:
  - load config
  - init logging
  - build app context
  - start TUI shell entrypoint (`src/ui/shell.rs`, placeholder for now)
- Extension points are added via stub adapters in app context:
  - `telegram::TelegramAdapter::stub()`

## Layer contracts and stubs (Task 5)

- Usecase contracts:
  - `usecases::contracts::AppEventSource`
  - `usecases::contracts::ShellOrchestrator`
- Infra contracts:
  - `infra::contracts::ConfigAdapter`
  - `infra::contracts::StorageAdapter`
  - `infra::contracts::ExternalOpener`
- Stub implementations for shell-only runtime:
  - `infra::stubs::{StubConfigAdapter, StubStorageAdapter, NoopOpener}`
  - `ui::event_source::MockEventSource` (tests)

Data-flow note: `docs/phase1-layer-data-flow.md`
