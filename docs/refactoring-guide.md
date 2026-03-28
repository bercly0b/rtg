# Refactoring Guide — Decomposing Large Modules

This guide describes the standard approach for breaking down modules that exceed the ~200 LOC soft limit.

## Strategy: free functions + context struct

When a module grows large because its struct accumulates too many methods, extract logic into **free functions** living in sub-modules. The struct methods become thin delegates.

### Why free functions

- Each function explicitly declares what data it needs (no hidden `self` bag).
- Functions live in separate files — files stay small and focused.
- Easier to test in isolation (pass mock data without constructing the full struct).
- No coupling to a specific struct — logic is reusable.

### OrchestratorCtx pattern

When multiple free functions need overlapping subsets of mutable state, define a **context struct** that borrows the fields:

```rust
// shell/mod.rs
pub(super) struct OrchestratorCtx<'a, D: TaskDispatcher> {
    pub state: &'a mut ShellState,
    pub dispatcher: &'a D,
    pub chat_list_in_flight: &'a mut bool,
    // ... other borrowed fields
}
```

The owning struct provides a builder method:

```rust
impl<S, O, D> DefaultShellOrchestrator<S, O, D> {
    fn as_ctx(&mut self) -> OrchestratorCtx<'_, D> {
        OrchestratorCtx {
            state: &mut self.state,
            dispatcher: &self.dispatcher,
            chat_list_in_flight: &mut self.chat_list_in_flight,
            // ...
        }
    }
}
```

Struct methods delegate to free functions:

```rust
fn start_voice_recording(&mut self) {
    voice::start_voice_recording(&mut self.as_ctx());
}
```

### When NOT to use this pattern

- If the module is small (<300 LOC) and has a single responsibility — keep methods on the struct.
- If a function only touches `&self` (read-only) with 1-2 fields — a plain parameter list is fine.
- If the function is inherently tied to the struct's lifecycle (constructors, Drop) — keep it as a method.

## File layout

Convert `module.rs` into a directory `module/` with sub-modules:

```
src/layer/module/
  mod.rs             — struct definition, constructors, trait impls, OrchestratorCtx, delegates
  feature_a.rs       — free functions for feature A
  feature_b.rs       — free functions for feature B
```

### Naming conventions

- Sub-module names reflect the **responsibility**, not the struct name.
- Use `pub(super)` visibility for free functions — they are internal to the module.
- Keep `pub` only on items that are part of the module's external API.

## Test organization

### When to split tests

Tests follow the same soft limit as production code: **~200 LOC per file**. Inline `#[cfg(test)] mod tests` is fine for small modules. When a test module exceeds ~300 LOC or the parent file becomes hard to navigate, split tests into a `tests/` directory.

### Rule: tests must not inflate production modules

A production file should contain production code. If an inline test module pushes the file well beyond the soft limit, extract the tests. Never leave 4000+ LOC of tests at the bottom of a production module.

### Directory layout

```
src/layer/module/
  mod.rs
  feature_a.rs
  feature_b.rs
  tests/
    mod.rs           — #[cfg(test)] gate, shared test doubles, domain helpers, factories
    feature_a.rs     — tests for feature A
    feature_b.rs     — tests for feature B
```

`mod.rs` in the parent module declares the test sub-module:

```rust
#[cfg(test)]
mod tests;
```

`tests/mod.rs` re-exports shared infrastructure and declares sub-modules:

```rust
mod feature_a;
mod feature_b;

// shared test doubles, helpers, factories below
```

### Test infrastructure

- **Test doubles** (recording dispatchers, stub adapters) live in `tests/mod.rs`.
- **Domain helpers** (factory functions like `chat()`, `message()`) live in `tests/mod.rs`.
- **Orchestrator factories** (`make_orchestrator()`, `orchestrator_with_chats()`) live in `tests/mod.rs`.
- Each test sub-module imports from `super::*` to access the shared infrastructure.

### Test sub-module grouping

Group tests by **feature area**, not by production module. One test sub-module should cover a cohesive set of behaviors. Typical grouping:

- `lifecycle.rs` — startup, shutdown, connectivity
- `chat_list.rs` — chat list loading, selection, refresh
- `chat_open.rs` — open/close/reopen, cached display, TDLib lifecycle
- `voice.rs` — recording, playback, command popup
- etc.

### Tests call the public API

Tests exercise the module through its public trait/struct API, not by calling free functions directly. This keeps tests stable when internal structure changes.

## Step-by-step process

1. **Create the directory** — `module.rs` becomes `module/mod.rs`.
2. **Define OrchestratorCtx** (if applicable) and `as_ctx()` in `mod.rs`.
3. **Extract one group at a time** — move functions to a sub-module, add delegate methods, run quality gate.
4. **Move corresponding tests** into `tests/` sub-modules alongside the production code.
5. **Run quality gate after each extraction**: `cargo fmt --check && cargo clippy && cargo test && cargo check`.
6. **Repeat** until `mod.rs` contains only the struct, constructors, trait impl, and thin delegates.

## Target metrics

- Each sub-module: ~50-250 LOC.
- `mod.rs` after full decomposition: ~200-400 LOC (struct + constructors + trait impl + delegates).
- No single file exceeds ~400 LOC in a fully decomposed module.
