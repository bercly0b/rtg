# Project Structure — `src/`

## Architectural layers

```
┌──────────────────────────────────────────────┐
│              main.rs / app.rs / cli.rs       │  Entry points & dispatch
├──────────────────────────────────────────────┤
│                                              │
│   ┌───────┐   ┌──────────┐   ┌───────────┐   │
│   │  ui/  │   │ usecases/│   │ telegram/ │   │
│   │       │◄──┤          │──►│           │   │
│   │Render │   │Orchestr. │   │TDLib impl │   │
│   │Events │   │Traits    │   │Adapters   │   │
│   │Widgets│   │Dispatch  │   │Mappers    │   │
│   └───┬───┘   └────┬─────┘   └─────┬─────┘   │
│       │            │               │         │
│       └────────────┼───────────────┘         │
│                    ▼                         │
│             ┌────────────┐                   │
│             │  domain/   │                   │
│             │            │                   │
│             │ Entities   │                   │
│             │ State      │                   │
│             │ Events     │                   │
│             └────────────┘                   │
│                    ▲                         │
│             ┌──────┴─────┐                   │
│             │   infra/   │                   │
│             │            │                   │
│             │ Config     │                   │
│             │ Logging    │                   │
│             │ Storage    │                   │
│             │ Errors     │                   │
│             └────────────┘                   │
└──────────────────────────────────────────────┘
```

**Dependency direction**: `domain/` depends on nothing. `usecases/` depends on `domain/` and defines trait ports. `telegram/` and `ui/` implement or consume those ports. `infra/` provides cross-cutting support (config, logging, storage).

## Layer descriptions

### `domain/` — Pure types, no I/O

Core entities, state machines, enums, events. Zero external dependencies, no I/O, no framework imports.

Examples: `ChatSummary`, `Message`, `ShellState`, `AppEvent`, `MessageCache`, `ChatListState`.

### `usecases/` — Business logic and orchestration

Application workflows, use case functions, trait definitions (ports) for external dependencies, background task dispatching. Traits defined here act as contracts — they describe *what* the application needs without specifying *how*.

Examples: `ListChatsSource` trait, `send_message()`, `ShellOrchestrator`, `TaskDispatcher`.

### `telegram/` — TDLib integration

Implements usecase-layer traits using TDLib. TDLib client wrapper, type mappers (TDLib → domain), update monitors, pagination helpers.

Examples: `TelegramAdapter`, `TdLibClient`, `tdlib_mappers/`, `TelegramChatUpdatesMonitor`.

### `ui/` — Terminal interface (ratatui)

TUI rendering, custom widgets, event source adapters, visual styles. Consumes domain state, produces `AppEvent`s.

Examples: `view/` (layout), `chat_message_list/` (widget), `styles/`, `event_source/`, `message_rendering/`.

### `infra/` — Infrastructure and cross-cutting concerns

Configuration loading, logging setup, file system paths, error types, secret redaction, stubs for testing, external tool wrappers.

Examples: `config/`, `StorageLayout`, `logging.rs`, `secrets.rs`, `stubs.rs`.

## Entry points

| File | Role |
|------|------|
| `main.rs` | Binary entry point. Declares top-level modules, parses CLI args, installs panic hooks, delegates to `app::run()`. |
| `app.rs` | Application dispatch. Routes between `Run` (TUI startup) and `Logout` flows. Composition root. |
| `cli.rs` | CLI argument definitions (clap). Subcommands: `Run` (default — launch TUI), `Logout` (disconnect and clear session). |

When adding a new top-level command or changing the startup sequence, start from `cli.rs` → `app.rs`.

## Module conventions

Code organization rules (LOC limits, decomposition, test extraction) — see [CLAUDE.md](../CLAUDE.md) § Code organization rules.

- **Trait ports** live in `usecases/`; implementations in `telegram/` or `infra/`.
- **Stubs for testing**: every trait used for DI should have a stub/mock (see `infra/stubs.rs`).

### Directory-based module layout

When a module is decomposed into a directory, follow this structure:

```
src/layer/module/
  mod.rs             — struct, constructors, trait impls, thin delegate methods
  feature_a.rs       — free functions for feature A (pub(super) visibility)
  feature_b.rs       — free functions for feature B
  tests/
    mod.rs           — #[cfg(test)] gate, shared test doubles, helpers, factories
    feature_a.rs     — tests for feature A
    feature_b.rs     — tests for feature B
```

Sub-module names reflect **responsibility**, not the struct name. Tests exercise the module's **public API**, not internal free functions.

## Where to put new functionality — decision guide

1. **New domain concept** (entity, state, event type) → `domain/`
2. **New workflow / use case** (orchestration, business rule) → `usecases/`
3. **New Telegram API interaction** → `telegram/`
4. **New visual component or screen** → `ui/`
5. **New config option, storage path, external tool** → `infra/`
6. **New CLI subcommand** → `cli.rs` + handler in `app.rs`
7. **Crosses multiple layers?** → Define the trait in `usecases/`, implement in the appropriate layer, wire in `app.rs` / `bootstrap.rs`.
