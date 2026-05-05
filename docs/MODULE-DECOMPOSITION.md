# Module Decomposition

How RTG organizes modules that grow beyond a single file. This is the
codebase's standing convention — not a one-off plan.

## Pattern: free functions + context struct

When a struct accumulates many methods, the logic is extracted into **free
functions** in sub-modules; the struct's methods become thin delegates.
Multiple free functions that need overlapping subsets of mutable state share
a **context struct** that borrows the relevant fields.

### Why free functions

- Each function explicitly declares what data it needs (no hidden `self` bag).
- Functions live in separate files, so files stay small and focused.
- Easier to test in isolation — pass mock data without constructing the
  full struct.
- No coupling to a specific struct — logic is reusable.

### `OrchestratorCtx` — concrete example

The shell orchestrator (`src/usecases/shell/`) is the canonical instance of
this pattern. `OrchestratorCtx` borrows the parts of `ShellState` and the
dispatcher that the orchestrator's free functions touch:

```rust
// src/usecases/shell/mod.rs
pub(super) struct OrchestratorCtx<'a, D: TaskDispatcher> {
    pub state: &'a mut ShellState,
    pub dispatcher: &'a D,
    pub chat_list_in_flight: &'a mut bool,
    // ... other borrowed fields
}
```

The owning struct exposes a builder method:

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

Methods on the orchestrator delegate to free functions in sub-modules
(`chat_list`, `chat_open`, `voice`, `message_actions`, ...):

```rust
fn start_voice_recording(&mut self) {
    voice::start_voice_recording(&mut self.as_ctx());
}
```

### When this pattern doesn't fit

- The module is small (< ~300 LOC) with a single responsibility — keep
  methods on the struct.
- A function only takes `&self` (read-only) and touches 1–2 fields — a
  plain parameter list is fine.
- The function is tied to the struct's lifecycle (constructors, `Drop`) —
  keep it as a method.

## File layout

A decomposed module lives in a directory:

```
src/layer/module/
  mod.rs             — struct definition, constructors, trait impls,
                       context struct, delegate methods
  feature_a.rs       — free functions for feature A
  feature_b.rs       — free functions for feature B
```

Conventions:

- Sub-module names reflect the **responsibility**, not the struct name.
- Free functions are `pub(super)` — they're internal to the module.
- Reserve `pub` for items that are part of the module's external API.

See [`ARCHITECTURE.md`](ARCHITECTURE.md) for the layer organization this
slots into.

## Test layout

Tests follow the same size discipline as production code. Inline
`#[cfg(test)] mod tests` is fine for small modules. Once the test module
exceeds ~300 LOC or makes the parent file hard to navigate, tests move into
a `tests/` sub-directory.

### Tests must not inflate production modules

A production file should contain production code. If an inline test module
pushes the file well past the soft limit, extract the tests. Don't leave
thousands of LOC of tests at the bottom of a production module.

### Directory layout

```
src/layer/module/
  mod.rs
  feature_a.rs
  feature_b.rs
  tests/
    mod.rs           — #[cfg(test)] gate, shared test doubles, helpers,
                       factories
    feature_a.rs     — tests for feature A
    feature_b.rs     — tests for feature B
```

The parent `mod.rs` declares the test sub-module:

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

### Shared test infrastructure

- **Test doubles** (recording dispatchers, stub adapters) live in
  `tests/mod.rs`.
- **Domain helpers** (factory functions like `chat()`, `message()`) live
  in `tests/mod.rs`.
- **Orchestrator factories** (`make_orchestrator()`, etc.) live in
  `tests/mod.rs`.
- Each test sub-module imports from `super::*` for shared infrastructure.

### Group tests by feature area

Group tests around a cohesive set of behaviors, not by which production
file they exercise. Typical groupings:

- `lifecycle.rs` — startup, shutdown, connectivity
- `chat_list.rs` — chat list loading, selection, refresh
- `chat_open.rs` — open / close / reopen, cached display, TDLib lifecycle
- `voice.rs` — recording, playback, command popup

### Tests target the public API

Tests exercise the module through its public trait/struct API rather than
calling internal free functions directly. This keeps tests stable when
internal structure changes.

## Size guidelines

- Each sub-module: ~50–250 LOC.
- A fully decomposed `mod.rs`: ~200–400 LOC (struct, constructors, trait
  impl, delegates).
- No single file beyond ~400 LOC inside a decomposed module.

These are guidelines, not hard caps — readability and cohesion trump line
counts.
