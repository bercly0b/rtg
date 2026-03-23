# TDLib Migration Plan

Plan for migrating RTG from grammers to TDLib for Telegram integration.

## Context and Motivation

### Current State

RTG uses **grammers** (pure Rust MTProto implementation) for Telegram API. This works but has limitations:

- No built-in data persistence — every startup fetches chat list from Telegram
- No offline support — app requires network connection to display anything
- Manual cache implementation would be complex and error-prone

### Why TDLib

**TDLib** (Telegram Database Library) is the official Telegram client library that provides:

- **Built-in SQLite cache** — chats, messages, users persisted automatically
- **Instant startup** — UI renders from cache immediately, background sync follows
- **Offline support** — cached data available without network
- **Automatic sync** — handles updates, reconnections, gap filling
- **Full API coverage** — all Telegram features, always up-to-date

### Trade-offs

| Aspect | grammers (current) | TDLib (target) |
|--------|-------------------|----------------|
| Caching | None | SQLite, automatic |
| Startup | Slow (network fetch) | Instant (from cache) |
| Offline | No | Yes |
| Build complexity | Simple | Requires TDLib system dependency |
| Binary size | ~5 MB | +20-30 MB |
| Rust integration | Native async/await | FFI via tdlib-rs |

## Technical Decisions

### Rust Bindings

Using **tdlib-rs** from FedericoBruzzone (improved fork):
- Repository: https://github.com/FedericoBruzzone/tdlib-rs
- Cross-platform: Linux, macOS, Windows (x86_64 and arm64)
- Typed Rust API generated from TDLib's TL schema
- `download-tdlib` feature: precompiled TDLib downloaded automatically
- Active maintenance, current TDLib version: 1.8.61

### TDLib Installation

With `download-tdlib` feature enabled, TDLib is downloaded automatically during build.
No manual installation required.

For manual installation (if needed):
```bash
# macOS
brew install tdlib

# Linux (Ubuntu/Debian) - build from source
# See https://tdlib.github.io/td/build.html
```

### Migration Strategy

- **Big bang replacement** — fully replace grammers with tdlib-rs in one migration
- **No parallel backends** — cleaner codebase, no maintenance overhead
- **macOS first** — Linux CI support added later

### Configuration

Existing `config.toml` fields remain compatible:
- `api_id` — passed to TDLib initialization
- `api_hash` — passed to TDLib initialization

TDLib will store its database in RTG's data directory (alongside session files).

## Migration Phases

Each phase is a separate PR. Phases are sequential — each builds on the previous.

---

### Phase 1: Infrastructure Setup ✅

**Status**: Completed (2026-03-03)

**Branch**: `feature/tdlib-infra-setup`

**Goal**: Add tdlib-rs dependency, configure build, update CI.

**Scope**:
- Add `tdlib-rs` crate to Cargo.toml with `download-tdlib` feature
- Update README with TDLib prerequisites (automatic download via tdlib-rs)
- Simplify CI workflow (no manual TDLib installation needed with download-tdlib)
- Verify project builds with new dependency

**Files changed**:
- `Cargo.toml`
- `README.md`

**Notes**: CI workflow unchanged — `download-tdlib` feature handles TDLib installation automatically during cargo build.

---

### Phase 2: TDLib Client Wrapper ✅

**Status**: Completed (2026-03-03)

**Branch**: `feature/tdlib-client-wrapper`

**Goal**: Create foundational TDLib client with lifecycle management.

**Scope**:
- New module `src/telegram/tdlib_client.rs`
- TDLib initialization with api_id, api_hash, database path
- Proper shutdown handling
- Basic update receiver loop structure

**Files changed**:
- `src/telegram/tdlib_client.rs` (new, ~230 LOC)
- `src/telegram/mod.rs`
- `src/infra/storage_layout.rs` (added `tdlib_database_dir()`, `tdlib_files_dir()`)

**Implementation**:
- `TdLibConfig` struct with custom `Debug` to redact `api_hash`
- `TdLibError` enum (Init, Request, Shutdown variants)
- `TdLibClient` with `new()`, `close()`, `Drop` warning
- 4 unit tests covering config, client creation, idempotent close

---

### Phase 3: Authentication Flow ✅

**Status**: Completed (2026-03-03)

**Branch**: `feature/tdlib-migration`

**Goal**: Implement authentication via TDLib.

**Scope**:
- TDLib auth state machine (WaitPhoneNumber → WaitCode → WaitPassword → Ready)
- Implement `TelegramAuthClient` trait for TDLib backend
- Adapt `guided_auth` to work with TDLib auth flow
- Session persistence handled by TDLib automatically

**Files changed**:
- `src/telegram/tdlib_auth.rs` (new, ~530 LOC)
- `src/telegram/tdlib_client.rs` (extended with auth methods)
- `src/telegram/mod.rs` (switched to TdLibAuthBackend)
- `src/infra/storage_layout.rs` (removed dead_code allow)

**Implementation**:
- `TdLibAuthBackend` with full auth state machine
- Synchronous update loop in dedicated thread (polls `tdlib_rs::receive()`)
- Auth state channel for dispatching `WaitTdlibParameters`, `WaitPhoneNumber`, etc.
- State caching (`last_auth_state`) to prevent race conditions
- Error mapping: TDLib errors → `AuthBackendError` variants

**Key learnings**:
- TDLib requires constant polling via `receive()` — implemented as sync thread, not async
- `AtomicBool` with `compare_exchange` for thread-safe `close()` idempotency
- Auth state can arrive before we start waiting — cache needed

---

### Phase 4: Chat List ✅

**Status**: Completed (2026-03-03)

**Branch**: `feature/tdlib-migration`

**Goal**: Implement chat list loading via TDLib.

**Scope**:
- Implement `ListChatsSource` trait for TDLib backend
- Map TDLib chat types to domain `ChatSummary`
- Handle chat ordering (TDLib provides sorted list)

**Files changed**:
- `src/telegram/tdlib_mappers.rs` (new, ~300 LOC)
- `src/telegram/tdlib_auth.rs` (added `list_chat_summaries()`)
- `src/telegram/tdlib_client.rs` (added `get_chats()`, `get_chat()`, `get_user()`)
- `src/telegram/mod.rs` (ListChatsSource routes to TdLibAuthBackend)

**Implementation**:
- `tdlib_mappers` module for TDLib → domain type conversion
- `load_chats()` + `get_chats()` pattern (load first, then fetch IDs)
- Sequential `get_chat()` + `get_user()` for each chat (performance tracked in backlog)
- Message preview extraction for 20+ content types
- User online status from `UserStatus::Online`
- Outgoing read status via `last_read_outbox_message_id` comparison

**Key learnings**:
- TDLib `get_chats()` returns only IDs — need `get_chat()` for full info
- `ChatPosition::is_pinned` only valid for `ChatList::Main`
- `Chats` enum has single variant `Chats::Chats` — pattern match to extract

---

### Phase 5: Messages ✅

**Status**: Completed (2026-03-04)

**Branch**: `feature/tdlib-migration`

**Goal**: Implement message loading and sending via TDLib.

**Scope**:
- Implement `MessagesSource` trait (getChatHistory)
- Implement `MessageSender` trait (sendMessage)
- Map TDLib message types to domain `Message`

**Files changed**:
- `src/domain/message.rs` (changed `Message.id` from `i32` to `i64`)
- `src/telegram/tdlib_client.rs` (added `get_chat_history`, `send_message`)
- `src/telegram/tdlib_auth.rs` (added `list_messages`, `send_message`)
- `src/telegram/tdlib_mappers.rs` (added `map_tdlib_message_to_domain`, `extract_message_media`, `extract_message_text`)
- `src/telegram/mod.rs` (wired `MessagesSource`, `MessageSender` to TDLib backend)

**Implementation**:
- Changed `Message.id` type from `i32` to `i64` to match TDLib's type
- `get_chat_history` returns messages newest-first; mapper reverses to oldest-first for domain
- Full message content extraction: text, photos, videos, documents, stickers, etc.
- Link preview extraction from `MessageText.link_preview` (TDLib API changed from `web_page`)
- Sender name resolution via `get_user()` for user senders

**Key learnings**:
- TDLib `Message.id` is `i64`, not `i32` like grammers
- TDLib `get_chat_history` returns messages in reverse chronological order
- TDLib `MessageText` uses `link_preview` field instead of `web_page`

---

### Phase 6: Live Updates ✅

**Status**: Completed (2026-03-04)

**Branch**: `feature/tdlib-migration`

**Goal**: Handle real-time updates from TDLib.

**Scope**:
- Process TDLib update stream (updateNewMessage, updateChatLastMessage, etc.)
- Replace current `TelegramChatUpdatesMonitor` with TDLib-based implementation
- Map TDLib updates to domain events

**Files changed**:
- `src/telegram/tdlib_updates.rs` (new, ~40 LOC) - `TdLibUpdate` enum
- `src/telegram/tdlib_client.rs` (extended update loop with typed dispatch, added `take_update_receiver`)
- `src/telegram/tdlib_auth.rs` (added `take_update_receiver`)
- `src/telegram/chat_updates.rs` (complete rewrite for TDLib)
- `src/telegram/mod.rs` (wired `start_chat_updates_monitor` to TDLib backend)

**Implementation**:
- `TdLibUpdate` enum for typed update events (`NewMessage`, `MessageContent`, `DeleteMessages`, `ChatLastMessage`, `ChatReadInbox`, `ChatReadOutbox`, `UserStatus`)
- Update loop extended to dispatch typed events via `mpsc::Sender<TdLibUpdate>`
- `take_update_receiver()` pattern for single-use extraction of update channel
- `TelegramChatUpdatesMonitor` completely rewritten: removed grammers/tokio async dependency, now uses OS thread with mpsc channel
- Typed updates collapsed to `()` signal in monitor for now (granular UI handling in future PR)

**Key learnings**:
- Removed async/tokio dependency from chat updates monitor - uses simple OS thread
- `take_update_receiver` pattern prevents multiple consumers of update stream
- Typed updates provide foundation for granular UI handling in future

---

### Phase 7: Cleanup ✅

**Status**: Completed (2026-03-04)

**Branch**: `feature/tdlib-cleanup`

**Goal**: Remove grammers and obsolete code.

**Scope**:
- Remove `grammers-client`, `grammers-session` from Cargo.toml
- Delete `src/telegram/auth.rs` (1,224 LOC legacy grammers backend)
- Remove session file management (`session_dir`, `session_file()`, `session_policy_invalid_file()`)
- Rename session lock to instance lock (`SessionLockGuard` → `InstanceLockGuard`, lock file moved to `config_dir/rtg.lock`)
- Replace grammers session detection with TDLib database directory check (`tdlib_session_exists()`)
- Remove `StartupConfig` / `session_probe_timeout_ms` (no more protocol probes)
- Remove `persist_authorized_session` from `TelegramAuthClient` trait (TDLib handles persistence)
- Remove `session_path` parameter from `run_guided_auth()`
- Remove policy invalid marker mechanism entirely
- Slim tokio features to `rt-multi-thread` only
- Update internal documentation

**Files changed**:
- `Cargo.toml`, `Cargo.lock` — removed grammers deps, slimmed tokio features
- `config.example.toml` — removed `[startup]` section
- `src/telegram/auth.rs` — deleted (1,224 LOC)
- `src/telegram/mod.rs` — removed `mod auth`, removed `persist_authorized_session` impl
- `src/telegram/tdlib_auth.rs` — removed `persist_authorized_session`, cleaned `#[allow]`
- `src/telegram/tdlib_client.rs` — cleaned `#[allow(dead_code)]` comments, renamed `update_thread` → `_update_thread`
- `src/telegram/tdlib_updates.rs`, `src/telegram/tdlib_mappers.rs` — updated `#[allow]` comments
- `src/infra/storage_layout.rs` — removed session_dir, added `instance_lock_file()`, `tdlib_session_exists()`
- `src/infra/error.rs` — renamed `SessionStoreBusy` → `InstanceBusy`, `SessionLockCreate` → `InstanceLockCreate`, removed `SessionProbe`, added `TdlibDataCleanup`
- `src/infra/logging.rs` — removed grammers log filters, added `tdlib_rs`
- `src/infra/config/app_config.rs`, `src/infra/config/file_config.rs`, `src/infra/config/mod.rs` — removed `StartupConfig`
- `src/usecases/startup.rs` — rewrote session detection, instance lock, all tests
- `src/usecases/logout.rs` — rewrote for TDLib data cleanup
- `src/usecases/guided_auth.rs` — removed session_path, persist_session_marker, policy marker
- `src/app.rs` — updated for new API
- `docs/internal/CHAT_LIVE_UPDATES.md` — rewrote for TDLib architecture
- `docs/internal/RTG_REVIEW_BACKLOG.md` — marked grammers entries as wontfix

---

## Type Mapping Reference

Quick reference for mapping between current domain types and TDLib types.

### ChatSummary

| Domain field | TDLib source |
|--------------|--------------|
| `chat_id` | `Chat::id` |
| `title` | `Chat::title` |
| `unread_count` | `Chat::unread_count` |
| `last_message_preview` | `Chat::last_message.content` |
| `last_message_unix_ms` | `Chat::last_message.date` * 1000 |
| `is_pinned` | `ChatPosition::is_pinned` |
| `chat_type` | `Chat::type_` (Private/BasicGroup/Supergroup/Channel) |
| `last_message_sender` | `Message::sender_id` → user/chat name |
| `is_online` | `User::status` |
| `outgoing_status` | `Message::is_outgoing`, `Chat::last_read_outbox_message_id` |

### Message

| Domain field | TDLib source |
|--------------|--------------|
| `id` | `Message::id` |
| `sender_name` | `Message::sender_id` → resolve name |
| `text` | `MessageContent::MessageText::text` |
| `timestamp_ms` | `Message::date` * 1000 |
| `is_outgoing` | `Message::is_outgoing` |
| `media` | `MessageContent` variant |

### MessageMedia

| Domain variant | TDLib MessageContent |
|----------------|---------------------|
| `None` | `MessageText` |
| `Photo` | `MessagePhoto` |
| `Voice` | `MessageVoiceNote` |
| `Video` | `MessageVideo` |
| `VideoNote` | `MessageVideoNote` |
| `Sticker` | `MessageSticker` |
| `Document` | `MessageDocument` |
| `Audio` | `MessageAudio` |
| `Animation` | `MessageAnimation` |
| `Contact` | `MessageContact` |
| `Location` | `MessageLocation` |
| `Poll` | `MessagePoll` |

---

## Migration Complete

All 7 phases completed. RTG now uses TDLib exclusively for Telegram API integration.

**Key outcomes**:
- grammers dependencies fully removed
- No legacy session file management code remains
- TDLib handles session persistence, caching, and sync automatically
- Chat updates use synchronous OS threads + mpsc channels (no async runtime for updates)

---

## Notes

- TDLib handles session persistence internally — no need for manual session file management
- TDLib's SQLite database will be stored in `~/.local/share/rtg/tdlib/` (or platform equivalent)
- First run after migration will require re-authentication (new session format)
