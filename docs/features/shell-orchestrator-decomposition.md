# Shell orchestrator decomposition

Design note for splitting `DefaultShellOrchestrator` (`src/usecases/shell/`)
so its context, constructor, trait and files stop growing with every feature.
It closes the review backlog findings on `OrchestratorCtx`, the
`new_with_initial_state` constructor, file sizes, `take_pending_command_rx`,
`chat_updates.rs` and `ShellState` equality.

## Problems

- `OrchestratorCtx` has 22 `pub` fields, and every free function in every
  sub-module receives all of them. Nothing stops `message_input.rs` from
  touching download bookkeeping or the voice recorder.
- `new_with_initial_state` takes 10 arguments under
  `#[allow(clippy::too_many_arguments)]`.
- `shell/mod.rs` (~490 LOC) and `shell/background_results.rs` (~490 LOC) are
  past the ~400 LOC limit of [MODULE-DECOMPOSITION.md](../MODULE-DECOMPOSITION.md).
- `ShellOrchestrator::take_pending_command_rx` is a trait method for a single
  feature: every new "the UI must do something" need would add another one.
- `chat_updates.rs` mixes update routing with file download bookkeeping.
- `ShellState` cannot derive `PartialEq` because the notification carries an
  `Instant`.

## Design

### Zones and narrow contexts

The orchestrator's bookkeeping fields are grouped into **zones** — plain
structs owned by the orchestrator, each defined in the module that owns the
concern:

| Zone | Module | Fields |
|------|--------|--------|
| `ChatListLoads` | `chat_list.rs` | chat list in-flight / pending / force / user-requested flags, forum badge warm-up |
| `OpenChatSync` | `chat_open.rs` | chat opened in TDLib, in-flight message loads and prefetch, cached-display threshold, TDLib cache source |
| `Downloads` | `file_updates.rs` | active downloads, pending saves, auto-download size limit |
| `MediaCommands` | `command_popup.rs` | running external command, recorded file, record command, open handlers |

What every handler needs is a small base context:

```rust
struct ShellCtx<'a, D: TaskDispatcher> {
    pub state: &'a mut ShellState,
    pub dispatcher: &'a D,
    pub effects: &'a mut Vec<ShellEffect>,
}
```

A free function takes `&mut ShellCtx` plus **only the zones it touches**, so
its signature states what it can change and the compiler holds it to that:

```rust
fn dispatch_chat_list_refresh<D>(ctx: &mut ShellCtx<'_, D>, loads: &mut ChatListLoads, force: bool)
fn handle_message_input_key<D>(ctx: &mut ShellCtx<'_, D>, key: &str)
```

`OrchestratorCtx` stays as the root context, now `shell` + the four zones +
the opener. Only routers take it: `handle_event`, `key_dispatch.rs`,
`background_results.rs` and `chat_updates.rs`. They pick the zones each
handler needs and pass them down (disjoint field borrows).

Rejected: making the 22 fields private behind accessors. Every module would
still see every accessor, so the coupling stays; it only moves.

### `OrchestratorConfig`

`new_with_initial_state(storage, opener, dispatcher, initial_state,
cache_source, config)`, where `OrchestratorConfig` groups the values that come
from the app config: `min_display_messages`, `voice_record_cmd`,
`open_handlers`, `max_auto_download_bytes`, `key_overrides`. Its `Default`
holds the defaults `new()` used, so `new()` delegates to
`new_with_initial_state` and the field list lives in one place. `storage`,
`opener`, `dispatcher` and `cache_source` are ports, not configuration, and
stay separate arguments.

### Shell effects instead of `take_pending_command_rx`

```rust
pub enum ShellEffect {
    /// A command started; stream its output into the event source.
    StreamCommandOutput(Receiver<CommandEvent>),
}

trait ShellOrchestrator {
    // ...
    fn take_effects(&mut self) -> Vec<ShellEffect>;
}
```

Handlers push effects into `ShellCtx::effects`; the shell loop drains them
after each event and performs them. A future "the UI layer must do X" is a new
`ShellEffect` variant, not a new trait method.

Rejected for now: returning the effects from `handle_event`. It changes the
signature every caller and test uses, for the same behaviour.

### File split

- `popups.rs` — chat info, message info and help popups and the reaction
  picker: opening them, their keys, and their background results.
- `chat_search.rs` — chat search keys.
- `file_updates.rs` — `Downloads` zone, `FileUpdated` handling, auto-download
  and file name resolution, moved out of `chat_updates.rs`.
- `chat_list.rs` takes over the `ChatListLoaded` and forum badge results from
  `background_results.rs`.
- `key_dispatch.rs` gets the modal key routing (search, popups, message input)
  out of `handle_event`.

### `ShellState` equality

The notification becomes a `Notification { text, shown_at }` newtype whose
`PartialEq` compares the text only, the way `TypingState` already ignores its
timestamps. `ShellState` derives `PartialEq, Eq` again. A hand-written
`PartialEq` for `ShellState` was rejected: it lists 18 fields and silently
goes stale when a field is added.

## Out of scope

- Narrowing `ShellState` itself — handlers still get the whole state.
- Moving the command popup close detection in `ui/shell.rs` to an effect.
- Behaviour changes: this is a pure refactoring; the existing shell tests are
  the safety net.
