# AGENTS Rules (RTG)

## Project context

RTG is a terminal-first Telegram client in Rust (CLI + TUI).

Primary engineering direction: build in a Rust-way / Rust-like style.

## Workflow rules

1. Before starting any development task:
   - if current branch is `main`, create a feature branch first;
   - recommended naming: `feature/<short-task-name>`;
   - **recall hindsight memories** relevant to the task area (`hindsight memory recall`) to get context from previous work.

2. Planning -> implementation:
   - plan first;
   - then move to implementation of the planned scope.

3. Mandatory reviewer run:
   - after code is written, run `@code-reviewer`;
   - classify findings as `critical`, `medium`, `minor`.

4. Handling findings:
   - `critical`: fix immediately in the same task/PR;
   - `medium` and `minor`: report to the user in text with rationale and follow-up proposal, and track them in `docs/internal/RTG_REVIEW_BACKLOG.md`.

5. Test and quality gate is mandatory:
   - every code change must be covered with tests;
   - after finishing a task, run full quality gate locally:
     - `cargo fmt --check`
     - `cargo clippy`
     - `cargo test`
     - `cargo check`

6. Telegram API implementation rule:
   - before implementing features that interact with Telegram API, review `tdlib-rs` and TDLib documentation;
   - choose and document the most suitable TDLib integration approach for the task scope.

7. Commit after development task completion:
   - after finishing a development task (implementation + tests/quality gate), create a git commit;
   - commit messages must be written in English.

8. Store learnings in hindsight:
   - after completing a task, **store useful knowledge** via `hindsight memory retain`;
   - only store **high-level knowledge** that impacts future tasks: architectural decisions, design patterns chosen, discovered pitfalls, non-obvious workarounds, project conventions, integration nuances;
   - **do NOT store** low-level implementation details or trivial facts (e.g. "changed color to green", "renamed variable X to Y", "added field Z to struct") — these are visible in git history and carry no reusable value;
   - rule of thumb: if the knowledge would not influence a decision in a future task, it does not belong in hindsight;
   - be specific and include outcomes (what worked and what didn't).

## Code organization rules

1. Do not put the whole CRUD/feature into one large file.
2. Move validation to dedicated modules/directories.
3. Move utility helpers to dedicated modules/directories.
4. Keep type definitions in dedicated modules/files.
5. Soft limit: around 200 LOC per module.
6. Prefer logical modular decomposition over file growth.

## Internal documentation rule

Important things that help navigate the project must be documented in `docs/internal/`.

Project is open-source: do not add agent-specific mentions to shared repository files (code, docs, contributing guides, PR templates). Agent workflow files stay local and ignored (`docs/internal/`).

## Rust engineering style

1. Prefer strong typing and explicit domain modeling.
2. Keep error handling explicit and idiomatic.
3. Keep layer boundaries clear (`ui`, `domain`, `usecases`, `infra`, `telegram`).
4. Avoid mixing business logic with infrastructure details in one module.
