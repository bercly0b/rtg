# Hotkey System

RTG uses a centralized keymap module for all hotkey handling. This document
explains the architecture and how to add new hotkeys.

## Architecture

```
Terminal input (crossterm)
        |
    map_key_event()             src/ui/event_source/mod.rs
        |
    KeyInput { key, ctrl }      src/domain/events.rs
        |
    handle_event()              src/usecases/shell/mod.rs
        |
    Keymap::resolve()           src/domain/keymap/mod.rs
        |
    Action enum                 src/domain/keymap/mod.rs
        |
    key_dispatch                src/usecases/shell/key_dispatch.rs
```

### Key modules

| Module | Layer | Responsibility |
|--------|-------|----------------|
| `domain/keymap/mod.rs` | domain | `Action` enum, `KeyPattern`, `Keymap` (resolve + defaults) |
| `usecases/shell/key_dispatch.rs` | usecases | Maps `Action` to orchestrator calls |
| `usecases/shell/mod.rs` | usecases | Routes `InputKey` events through `Keymap::resolve()` |
| `infra/config/app_config.rs` | infra | `KeysConfig` for user overrides from TOML |
| `ui/help_popup.rs` | ui | Renders help popup from `Keymap::help_groups()` |

### Contexts

Hotkeys are context-aware. The `KeyContext` enum defines where a binding applies:

- **`ChatList`** — active when the chat list panel has focus.
- **`Messages`** — active when the messages panel has focus.
- **`Global`** — active in all contexts (ChatList + Messages).

`MessageInput` is handled separately (raw text input, not hotkeys) in
`usecases/shell/message_input.rs`: `Enter` sends the message, `Shift+Enter`
inserts a line break.

`Shift+Enter` is indistinguishable from `Enter` until the terminal is asked to
report modified keys, so `TerminalSession` requests it at startup and resets it
on teardown: `DISAMBIGUATE_ESCAPE_CODES` (kitty keyboard protocol) when
`supports_keyboard_enhancement()` reports support, otherwise xterm
`modifyOtherKeys=2` (`ESC[>4;2m`). Terminals that honour neither report a plain
`Enter`, which still sends the message.

### tmux

tmux implements no kitty keyboard protocol, so the `modifyOtherKeys` path is
used — but tmux only forwards the request when it is configured for extended
keys, and crossterm parses only the `csi-u` form. Required in `tmux.conf`:

```tmux
set -s extended-keys on
set -s extended-keys-format csi-u
set -as terminal-features 'xterm*:extkeys'
```

`extended-keys` defaults to `off`; `extkeys` tells tmux to request extended keys
from the outer terminal (match the pattern against the *outer* `TERM`). Reload
plus a client detach/attach is needed for the feature to take effect.

### Sequences

Key sequences like `dd` (delete message) are handled by `Keymap::resolve()`.
When the first key of a sequence is pressed, the keymap enters a **pending**
state. If the next key completes the sequence within the timeout (~1 second),
the action fires. Any other key (or timeout) cancels the pending state and
the new key is resolved normally.

## Adding a new hotkey

### Step 1: Add the action

In `src/domain/keymap/mod.rs`, add a variant to the `Action` enum:

```rust
pub enum Action {
    // ...existing variants...
    MyNewAction,
}
```

Update both `display_name()` and `from_name()` in the `impl Action` block
to include the new variant with a snake_case name:

```rust
Self::MyNewAction => "my_new_action",
```

### Step 2: Add the default binding

In the `default_bindings()` function at the bottom of `keymap/mod.rs`, add a
`KeyBinding`:

```rust
KeyBinding {
    pattern: KeyPattern::single("x"),  // or ::sequence(vec!["g", "g"])
    action: Action::MyNewAction,
    context: KeyContext::Messages,      // or ChatList / Global
},
```

### Step 3: Put the action in a hotkey group

The help popup lists hotkeys by hotkey group. In `keymap/mod.rs`, add the action to
exactly one group in `HOTKEY_GROUPS`, at the place where its row should appear:
the order of the table is the order of the help popup, whatever key the action
is bound to.

```rust
("Message", &[/* ...existing actions... */ Action::MyNewAction]),
```

A bound action that is in no group, or in two, fails the
`every_bound_action_belongs_to_exactly_one_hotkey_group` test.

### Step 4: Implement the action

In `src/usecases/shell/key_dispatch.rs`, add a match arm in the appropriate
dispatch function (`dispatch_chat_list_action` or `dispatch_messages_action`):

```rust
Action::MyNewAction => {
    // implementation here
}
```

### Step 5: Tests

1. Add a unit test in `src/domain/keymap/tests/` to verify the binding resolves correctly.
2. Add an integration test in `usecases/shell/tests/` to verify the action
   executes the expected side effects through the orchestrator.

### Step 6: Verify

```sh
mise run quality
```

## User configuration

Users can override any hotkey in `~/.config/rtg/config.toml`:

```toml
[keys]
select_next_chat = "n"
delete_message = "xx"
quit = "Q"
```

The key is the action name (snake_case), the value is the key pattern string.

### Supported key pattern formats

| Format | Example | Description |
|--------|---------|-------------|
| Single key | `"j"`, `"?"`, `"/"` | Single keypress |
| Ctrl combo | `"Ctrl+C"` | Ctrl + key |
| Sequence | `"dd"`, `"gg"` | Multiple keys pressed in order |
| Special key | `"Enter"`, `"Esc"` | Named special keys |

### How overrides work

When a user overrides a binding, the old key is unbound and the action is
rebound to the new pattern. The override applies to all contexts where the
action was originally bound. Invalid action names or key patterns are logged
as warnings and skipped.

The help popup (`?`) always shows the current (potentially overridden) bindings.
An override changes the key shown in a row, not the place of the row.
