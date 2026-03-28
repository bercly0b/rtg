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

**Put here**: new entity struct, state machine, event type, value object, domain enum.

**Do not put here**: anything that performs I/O, calls external APIs, or depends on framework crates.

Examples: `ChatSummary`, `Message`, `ShellState`, `AppEvent`, `MessageCache`, `ChatListState`.

### `usecases/` — Business logic and orchestration

Application workflows, use case functions, trait definitions (ports) for external dependencies, background task dispatching.

Traits defined here act as contracts — they describe *what* the application needs from the outside world without specifying *how*.

**Put here**: new use case / workflow, new trait for an external dependency, orchestration logic, background task definition.

**Do not put here**: TDLib-specific code, rendering logic, raw config parsing.

Examples: `ListChatsSource` trait, `send_message()`, `ShellOrchestrator`, `TaskDispatcher`.

### `telegram/` — TDLib integration

Implements usecase-layer traits using TDLib. Contains the TDLib client wrapper, type mappers (TDLib → domain), update monitors, pagination helpers.

**Put here**: anything that talks to TDLib — API calls, response mapping, update processing, TDLib configuration.

**Do not put here**: business logic decisions, rendering, generic infrastructure.

Examples: `TelegramAdapter`, `TdLibClient`, `tdlib_mappers`, `TelegramChatUpdatesMonitor`.

### `ui/` — Terminal interface (ratatui)

TUI rendering, custom widgets, event source adapters, visual styles. Consumes domain state, produces `AppEvent`s.

**Put here**: new widget, visual component, render function, style definition, terminal event handling.

**Do not put here**: business logic, API calls, state mutation logic.

Examples: `view.rs` (layout), `chat_message_list.rs` (widget), `styles.rs`, `event_source.rs`.

### `infra/` — Infrastructure and cross-cutting concerns

Configuration loading, logging setup, file system paths, error types, secret redaction, stubs for testing, external tool wrappers (browser opener).

**Put here**: config structs and loaders, logging init, storage path resolution, infrastructure error types, test stubs, external utility adapters (clipboard, browser).

**Do not put here**: business logic, Telegram-specific code, UI rendering.

Examples: `AppConfig`, `StorageLayout`, `logging.rs`, `secrets.rs`, `BrowserOpener`.

## Entry points

| File | Role |
|------|------|
| `main.rs` | Binary entry point. Declares top-level modules, parses CLI args, installs panic hooks, delegates to `app::run()`. |
| `app.rs` | Application dispatch. Routes between `Run` (TUI startup) and `Logout` flows. Composition root. |
| `cli.rs` | CLI argument definitions (clap). Subcommands: `Run` (default — launch TUI), `Logout` (disconnect and clear session). |

When adding a new top-level command or changing the startup sequence, start from `cli.rs` → `app.rs`.

## Module conventions

- **~200 LOC soft limit** per module. When a file grows beyond this, split by responsibility.
- **Dedicated files for types**: keep struct/enum definitions in their own modules rather than mixing them with logic.
- **Validation in separate modules**: extract input validation out of the main workflow code.
- **Utility helpers in dedicated modules**: don't bury reusable helpers inside large files.
- **Trait ports in `usecases/`**, implementations in `telegram/` or `infra/`.
- **Tests are inline** by default: use `#[cfg(test)] mod tests` within the module. For large decomposed modules (directory-based), tests may live in a `tests/` submodule — see [refactoring-guide.md](refactoring-guide.md).
- **Stubs for testing**: every trait used for dependency injection should have a stub/mock implementation (see `infra/stubs.rs` and `TelegramAdapter::stub()`).

## Where to put new functionality — decision guide

1. **New domain concept** (entity, state, event type) → `domain/`
2. **New workflow / use case** (orchestration, business rule) → `usecases/`
3. **New Telegram API interaction** → `telegram/`
4. **New visual component or screen** → `ui/`
5. **New config option, storage path, external tool** → `infra/`
6. **New CLI subcommand** → `cli.rs` + handler in `app.rs`
7. **Crosses multiple layers?** → Define the trait in `usecases/`, implement in the appropriate layer, wire in `app.rs` / `bootstrap.rs`.
