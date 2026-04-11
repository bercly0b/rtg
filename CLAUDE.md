# AGENTS Rules (RTG)

## Project context

RTG is a terminal-first Telegram client in Rust (CLI + TUI).

Primary engineering direction: build in a Rust-way / Rust-like style.

Project structure and module layout guide: [docs/project-structure.md](docs/project-structure.md).

## Workflow rules

1. Before starting any development task:
   - if current branch is `main`, create a feature branch first;
   - recommended naming: `feature/<short-task-name>`.

2. Mandatory reviewer run:
   - after non-trivial code changes, before committing a feature, run `@code-reviewer`;
   - classify findings as `critical`, `medium`, `minor`.

3. Handling findings:
   - `critical`: fix immediately in the same task/PR;
   - `medium` and `minor`: report to the user in text with rationale and follow-up proposal, and track them in `docs/features/RTG_REVIEW_BACKLOG.md`.

4. Test and quality gate is mandatory:
   - every code change must be covered with tests;
   - after finishing a task, run full quality gate locally:
     - `cargo fmt --check`
     - `cargo clippy`
     - `cargo test`

5. Telegram API implementation rule:
   - before implementing features that interact with Telegram API, review `tdlib-rs` and TDLib documentation;
   - follow the TDLib approach — do not invent custom abstractions or workflows; use TDLib models, update handling, and interaction patterns as intended by the library;
   - choose and document the most suitable TDLib integration approach for the task scope.

6. Commit after development task completion:
   - after finishing a development task (implementation + tests/quality gate), create a git commit;
   - commit messages must be written in English.

## Documentation rules

1. `docs/` is for project-wide, high-level documentation (module organization, hotkey conventions, architectural decisions, etc.).
2. `docs/features/` is for feature-specific documentation (design docs, implementation plans, behavior specs).

## Code organization rules

1. Do not put the whole CRUD/feature into one large file.
2. Extract validation into a dedicated module once there is more than one validator or the logic grows non-trivial — not preemptively for a single check.
3. Extract utility helpers into a dedicated module once they are reused or the logic grows non-trivial — not preemptively for a single helper.
4. Keep type definitions in dedicated modules/files.
5. Soft limit: around 300–400 LOC per module. Do not fragment preemptively below this threshold.
6. Prefer logical modular decomposition over file growth.
7. When a module grows beyond the soft limit, decompose into sub-modules with free functions; struct methods stay as thin delegates. Prefer free functions over methods — they explicitly declare data dependencies and are easier to test in isolation.
8. Extract tests into a `tests/` sub-directory when inline `#[cfg(test)]` exceeds ~300 LOC. Tests must exercise the public API of the module, not internal functions. Shared test doubles and factories live in `tests/mod.rs`.

## Rust engineering style

1. Keep layer boundaries clear (`ui`, `domain`, `usecases`, `infra`, `telegram`).
2. Avoid mixing business logic with infrastructure details in one module.

## UI principles

1. Optimistic UI: display the result of a user action immediately without waiting for the server response; resolve the actual request in the background (e.g., show a sent message instantly, confirm delivery asynchronously).
2. Performance and speed are a priority: prefer fast paths in architecture and implementation decisions; minimize latency visible to the user.

## Agent instruction principles

1. Think before coding:
   - no hidden assumptions — if the task is unclear, stop and ask;
   - state assumptions explicitly before starting implementation;
   - if there are several valid interpretations, show the options instead of silently picking one;
   - if a simpler approach exists, propose it — pushback against the original framing is allowed.

2. Simplicity first:
   - write the minimum code that solves the task; nothing speculative;
   - no features beyond what was requested;
   - no abstractions for one-off code;
   - no "flexibility" or "configurability" that was not asked for;
   - no handling for scenarios that cannot happen;
   - sanity check: "would a senior engineer call this overcomplicated?" — if yes, simplify.

3. Surgical changes:
   - touch only what the task requires; clean up only your own mess;
   - do not "improve" neighbouring code, comments, or formatting in passing;
   - do not refactor code that is not broken;
   - match the existing style even if you would write it differently;
   - every changed line must directly relate to the request.

4. Goal-driven execution:
   - turn tasks into verifiable goals and loop until verified;
   - "add validation" → write tests for invalid input first, then make them pass;
   - "fix a bug" → write a test reproducing the bug first, then fix it;
   - for multi-step tasks, build a plan with a `step → verify` checkpoint at each step.
