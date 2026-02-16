# Phase 1 — Layer Contracts and Data Flow

## Dependency direction

Dependencies go inward:

- `ui` depends on `usecases` contracts
- `usecases` depends on `domain` and infra contracts
- `infra` implements adapter contracts
- `telegram` remains an integration adapter (stub in Phase 1)
- `domain` has no outward dependencies on UI/infra

## Runtime flow (shell mode)

1. `main`/`app` bootstraps config + logging and builds `AppContext`.
2. `ui::shell` creates:
   - an `AppEventSource` implementation (`CrosstermEventSource`),
   - a `ShellOrchestrator` implementation (`DefaultShellOrchestrator`) wired with stub infra adapters.
3. UI render loop draws current `ShellState`.
4. UI pulls event from event source and forwards it to usecase orchestrator.
5. Orchestrator updates domain state and may call infra contracts (storage/opener) through adapters.
6. Loop exits when state switches to `stopping`.

This keeps integration boundaries explicit and allows replacing stubs with real Telegram/infra adapters later without breaking layer direction.
