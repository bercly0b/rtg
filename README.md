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
