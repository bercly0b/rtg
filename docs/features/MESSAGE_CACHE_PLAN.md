# Message Cache & Prefetch Implementation Plan

## Problem

When the user opens a chat in RTG, they see only 1 message (from TDLib's sparse local cache), then after 100-500ms the full message list loads from the network. Navigating away and back triggers the same reload cycle. This is jarring compared to clients like `tg` (Python), where chats open instantly.

## Root cause analysis

### Why tg (Python) feels instant

1. **In-memory retention**: `MsgModel.msgs` (`Dict[int, Dict[int, Dict]]`) keeps all fetched messages in RAM for the entire process lifetime. Revisiting a chat = dict lookup, zero TDLib calls.
2. **j/k renders messages**: navigating the chat list immediately renders messages for the highlighted chat (`_render_msgs()` called on every cursor move). By the time the user presses Enter, messages are already fetched and displayed.
3. **Push-based cache warming**: `updateNewMessage` handler inserts every incoming message into the in-memory dict for any chat, not just the currently open one.
4. **TDLib SQLite as a fast fallback**: with `use_message_database=true`, `getChatHistory` serves from local DB when RAM misses.

### Why RTG feels slow

1. **No application-level cache**: `OpenChatState.messages: Vec<Message>` holds messages only for the current chat. Switching chats discards the previous vector.
2. **j/k does NOT load messages**: cursor navigation only changes `selected_index`; no message fetch is triggered until Enter/l.
3. **`only_local: true` returns sparse data**: TDLib's local cache typically holds only the last message per chat unless the chat was previously opened and deeply fetched.
4. **Push updates are not cached**: `updateNewMessage` signals a chat update, but doesn't store the message for later use in any persistent structure.

### TDLib configuration comparison

Both projects use identical database settings:
- `use_message_database: true`
- `use_chat_info_database: true`
- `use_file_database: true`

The difference is entirely in the **application layer**, not TDLib configuration.

## Architecture

### New module: `domain/message_cache.rs`

A standalone struct `MessageCache` in the domain layer. It is a data store, separate from `OpenChatState` (which remains the view-state for the currently displayed chat).

```
ShellState
├── chat_list: ChatListState
├── open_chat: OpenChatState     (view state: what's currently rendered)
├── message_cache: MessageCache  (data store: all fetched messages across chats)
├── message_input: MessageInputState
└── ...
```

`OpenChatState` becomes a view into `MessageCache`: when a chat is opened, messages are read from cache; when background fetch completes, messages go into cache first, then `OpenChatState` is updated.

### MessageCache structure

```rust
pub struct MessageCache {
    /// Per-chat message storage, ordered by timestamp (newest last)
    chats: HashMap<i64, ChatMessages>,
    /// LRU tracking for eviction
    access_order: VecDeque<i64>,
    /// Max number of chats to keep in cache
    max_cached_chats: usize,
}

struct ChatMessages {
    messages: Vec<Message>,
    /// Whether a full fetch (not just local cache) has been completed
    fully_loaded: bool,
}
```

### Layer boundaries

- `MessageCache` lives in `domain/` (pure data, no I/O)
- Cache population is orchestrated by `usecases/shell.rs` (the orchestrator)
- TDLib calls remain in `telegram/` layer behind traits
- UI reads from `OpenChatState` (no direct cache access from UI)

## Implementation phases

### Phase 1: In-memory message cache (core)

**Goal**: Opening a previously visited chat is instant (zero TDLib calls).

**Changes**:

1. **Create `domain/message_cache.rs`**:
   - `MessageCache` struct with `HashMap<i64, ChatMessages>`
   - Methods: `get(chat_id) -> Option<&[Message]>`, `put(chat_id, messages)`, `add_message(chat_id, message)`, `remove_messages(chat_id, &[i64])`, `update_message_content(chat_id, msg_id, new_text)`
   - No I/O, no TDLib, pure domain logic

2. **Integrate cache into `ShellState`**:
   - Add `message_cache: MessageCache` field to `ShellState`
   - `shell_state.rs`: add accessor methods

3. **Modify `open_selected_chat()` in `shell.rs`**:
   - Before calling `try_show_cached_messages` (TDLib local), check `MessageCache` first
   - If cache hit with sufficient messages: set `Ready` immediately, skip TDLib local call
   - Always dispatch background refresh to pick up new messages

4. **Modify `handle_background_result` for `MessagesLoaded`**:
   - Store result in `MessageCache` before updating `OpenChatState`
   - On subsequent opens, cache is pre-populated

5. **Tests**:
   - Unit tests for `MessageCache` operations (insert, get, update, eviction)
   - Integration test: open chat A -> navigate to B -> return to A -> verify instant open

### Phase 2: Push-based cache warming

**Goal**: New messages arriving for any chat are stored in cache, not just the currently open chat.

**Changes**:

1. **Extend `TdLibUpdate` handling in `shell.rs`**:
   - On `ChatUpdateReceived` with a new message: insert into `MessageCache` regardless of whether the chat is currently open
   - Requires extracting full `Message` data from the update, not just a signal

2. **Modify `tdlib_client.rs` update routing**:
   - For `Update::NewMessage`: include the full `Message` in the `TdLibUpdate::NewMessage` variant (currently it only sends a signal with chat_id)
   - For `Update::DeleteMessages`: include message IDs to remove from cache
   - For `Update::MessageContent`: include updated content for in-place cache update

3. **Modify `chat_updates.rs`**:
   - Forward full `TdLibUpdate` variants (not just `Option<i64>`) to the orchestrator
   - This may require a richer event type or a second channel

4. **Modify orchestrator**:
   - `handle_chat_update` processes the full update: inserts/removes/modifies messages in `MessageCache`
   - If the updated chat is currently displayed, also update `OpenChatState`

5. **Tests**:
   - Test: receive `updateNewMessage` for chat B while chat A is open -> verify message is in cache -> open chat B -> verify instant

### Phase 3: Prefetching on chat list navigation

**Goal**: When the user navigates the chat list with j/k, the highlighted chat's messages are prefetched in the background.

**Changes**:

1. **Add prefetch dispatch in `handle_chat_list_key`**:
   - On j/k: after moving cursor, check if highlighted chat has data in `MessageCache`
   - If not: dispatch a background prefetch (lower priority than explicit open)
   - Debounce: don't dispatch if user is rapidly scrolling (e.g., skip if a prefetch for another chat is already in-flight)

2. **New background task variant: `PrefetchMessages`**:
   - Similar to `LoadMessages` but results go only into `MessageCache`, not `OpenChatState`
   - If by the time the result arrives the user has opened this chat, also update `OpenChatState`

3. **Prefetch window**: optionally prefetch not just the highlighted chat but also N neighbors (e.g., +/- 2 chats in the list)

4. **In-flight tracking**:
   - Track which chat IDs have prefetches in-flight to avoid duplicates
   - Cancel (or ignore results of) prefetches for chats that scrolled out of view

5. **Tests**:
   - Test: navigate to chat N with j/k -> verify prefetch dispatched -> wait -> press Enter -> verify instant display from cache

### Phase 4: LRU eviction and memory control

**Goal**: Prevent unbounded memory growth with many chats.

**Changes**:

1. **LRU tracking in `MessageCache`**:
   - `access_order: VecDeque<i64>` — move chat_id to front on access
   - When `chats.len() > max_cached_chats`, evict the least recently accessed chat

2. **Configurable limits**:
   - `max_cached_chats: usize` (default: 50-100)
   - `max_messages_per_chat: usize` (default: 200)
   - Expose in `config.toml` under `[cache]` section

3. **Memory-aware eviction** (optional):
   - Estimate memory usage per cached chat (message count * avg size)
   - Evict when total exceeds a threshold

4. **Tests**:
   - Test: fill cache to max -> verify LRU eviction -> verify most recently accessed chats survive

### Phase 5: UX polish

**Goal**: Eliminate the "1 message flash" artifact and add visual feedback.

**Changes**:

1. **Smart cache display threshold**:
   - If `MessageCache` returns fewer than N messages (e.g., < 5), show `Loading` state instead of the sparse preview
   - Configurable threshold

2. **Loading indicator for prefetch**:
   - In the message panel, show a subtle "loading more..." indicator at the top when background fetch is in-flight but cached messages are displayed

3. **Cache-hit indicator** (optional, debug/status bar):
   - Show whether current chat was served from cache or network

## Data flow diagrams

### Current flow (no cache)

```
Enter pressed
  └─> try_show_cached_messages (TDLib only_local, usually 1 msg)
       ├─> [1 msg shown] ─────────────────────> Render 1 msg
       └─> dispatch_load_messages (network)
            └─> [50 msgs arrive] ─────────────> Render 50 msgs (flash)
```

### Target flow (with cache)

```
j/k navigation
  └─> check MessageCache for highlighted chat
       ├─> [cache miss] ──> dispatch_prefetch (background)
       └─> [cache hit]  ──> (no action, already warm)

Enter pressed
  └─> check MessageCache
       ├─> [cache hit]  ──> set_ready(cached msgs) ──> Render instantly
       │    └─> dispatch_refresh (background, silent update)
       └─> [cache miss] ──> try TDLib only_local
            ├─> [sparse] ──> set_loading ──> dispatch_load_messages
            └─> [enough] ──> set_ready ──> dispatch_refresh

push update (updateNewMessage for any chat)
  └─> insert into MessageCache
       └─> if currently displayed chat: also update OpenChatState
```

## Migration strategy

Each phase is independently shippable and testable. The approach:

1. Phase 1 alone already solves the "re-opening a chat is slow" problem — **done**
2. Phase 2 makes the cache warmer passively — **done**
3. Phase 3 makes first-open of a chat fast during browsing — **done**
4. Phase 4 prevents memory issues at scale — **done**
5. Phase 5 polishes the UX — **done**

Phases can be implemented as separate PRs/branches.

## Risks and mitigations

| Risk | Mitigation |
|---|---|
| Memory usage grows with many chats | Phase 4 LRU eviction with configurable limits |
| Stale data in cache (edits, deletes not reflected) | Phase 2 push-based updates ensure cache stays fresh |
| Prefetch network overhead | Debounce rapid j/k; limit prefetch window; use TDLib local first |
| Race conditions (prefetch result arrives after chat changed) | In-flight tracking with chat_id guard (already exists for MessagesLoaded) |
| Breaking existing `OpenChatState` contract | OpenChatState remains the view-state; cache is a separate structure feeding into it |

## Affected files

| File | Change |
|---|---|
| `src/domain/message_cache.rs` | **New**: MessageCache struct and methods |
| `src/domain/mod.rs` | Add module export |
| `src/domain/shell_state.rs` | Add `message_cache` field |
| `src/usecases/shell.rs` | Cache-first open, prefetch on j/k, push update handling |
| `src/usecases/background.rs` | New `PrefetchMessages` task variant |
| `src/telegram/tdlib_client.rs` | Forward full message data in update loop |
| `src/telegram/tdlib_updates.rs` | Richer TdLibUpdate variants with message data |
| `src/telegram/chat_updates.rs` | Forward full updates to orchestrator |
| `src/domain/events.rs` | Richer `AppEvent` variants for cache updates |
| `src/infra/config/mod.rs` | Cache config section (Phase 4) |
| `src/ui/view.rs` | Loading indicator for cache partial state (Phase 5) |
