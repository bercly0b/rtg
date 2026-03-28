# AGENTS Rules (RTG)

## Project context

RTG is a terminal-first Telegram client in Rust (CLI + TUI).

Primary engineering direction: build in a Rust-way / Rust-like style.

Project structure and module layout guide: [docs/project-structure.md](docs/project-structure.md).

## Workflow rules

1. Before starting any development task:
   - if current branch is `main`, create a feature branch first;
   - recommended naming: `feature/<short-task-name>`.

2. Planning -> implementation:
   - plan first;
   - then move to implementation of the planned scope.

3. Mandatory reviewer run:
   - after code is written, run `@code-reviewer`;
   - classify findings as `critical`, `medium`, `minor`.

4. Handling findings:
   - `critical`: fix immediately in the same task/PR;
   - `medium` and `minor`: report to the user in text with rationale and follow-up proposal, and track them in `docs/features/RTG_REVIEW_BACKLOG.md`.

5. Test and quality gate is mandatory:
   - every code change must be covered with tests;
   - after finishing a task, run full quality gate locally:
     - `cargo fmt --check`
     - `cargo clippy`
     - `cargo test`
     - `cargo check`

6. Telegram API implementation rule:
   - before implementing features that interact with Telegram API, review `tdlib-rs` and TDLib documentation;
   - follow the TDLib approach — do not invent custom abstractions or workflows; use TDLib models, update handling, and interaction patterns as intended by the library;
   - choose and document the most suitable TDLib integration approach for the task scope.

7. Commit after development task completion:
   - after finishing a development task (implementation + tests/quality gate), create a git commit;
   - commit messages must be written in English.

## Documentation rules

1. `docs/` is for project-wide, high-level documentation (module organization, hotkey conventions, architectural decisions, etc.).
2. `docs/features/` is for feature-specific documentation (design docs, implementation plans, behavior specs).
3. Keep docs concise and up-to-date; remove or update stale documentation when the underlying code changes.

## Code organization rules

1. Do not put the whole CRUD/feature into one large file.
2. Move validation to dedicated modules/directories.
3. Move utility helpers to dedicated modules/directories.
4. Keep type definitions in dedicated modules/files.
5. Soft limit: around 200 LOC per module.
6. Prefer logical modular decomposition over file growth.

## Rust engineering style

1. Prefer strong typing and explicit domain modeling.
2. Keep error handling explicit and idiomatic.
3. Keep layer boundaries clear (`ui`, `domain`, `usecases`, `infra`, `telegram`).
4. Avoid mixing business logic with infrastructure details in one module.

## UI principles

1. Optimistic UI: display the result of a user action immediately without waiting for the server response; resolve the actual request in the background (e.g., show a sent message instantly, confirm delivery asynchronously).
2. Performance and speed are a priority: prefer fast paths in architecture and implementation decisions; minimize latency visible to the user.
