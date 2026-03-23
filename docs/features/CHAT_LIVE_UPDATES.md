# Live Chat Updates (Phase: Full Refresh)

## Goal

Update chat list in TUI automatically when Telegram sends new updates, without requiring manual `r` refresh.

## Architecture Overview

### TDLib Update Pipeline

**Design:** TDLib delivers updates via its internal polling loop. `TdLibClient` runs a dedicated OS thread that calls `tdlib_rs::receive()` in a loop, dispatches typed updates through an `mpsc` channel, and the chat updates monitor converts them to simple UI refresh signals.

No async runtime is needed for the update pipeline — it uses synchronous OS threads and `std::sync::mpsc` channels throughout.

### Data Flow

```
TdLibClient (update thread)
         |
         +-- receive() loop (polls TDLib for updates)
         |
         +-- Dispatches typed TdLibUpdate via mpsc::Sender<TdLibUpdate>
                    |
TelegramChatUpdatesMonitor::start(update_rx, signal_tx)
         |
         +-- OS thread: run_update_monitor()
                    |
                    +-- update_rx.recv_timeout() loop
                              |
                              +-- signal_tx.send(()) on any TdLibUpdate
                                        |
ui/event_source.rs <--------------------+
         |
         +-- has_pending_refresh() -> AppEvent::ChatListUpdateRequested
                    |
usecases/shell.rs <-+
         |
         +-- refresh_chat_list() -> full list_chats refresh
```

### Key Components

1. **TDLib Client Update Thread** (`src/telegram/tdlib_client.rs`)
   - Synchronous OS thread started in `TdLibClient::new()`
   - Polls `tdlib_rs::receive()` with 1-second timeout
   - Parses raw TDLib updates into typed `TdLibUpdate` enum
   - Dispatches via `mpsc::Sender<TdLibUpdate>` to consumer
   - Exits when `AtomicBool` stop flag is set

2. **Update Types** (`src/telegram/tdlib_updates.rs`)
   - `TdLibUpdate` enum: `NewMessage`, `MessageContent`, `DeleteMessages`, `ChatLastMessage`, `ChatReadInbox`, `ChatReadOutbox`, `UserStatus`
   - Each variant carries relevant fields (e.g., `chat_id`)
   - `.kind()` method returns string label for logging

3. **Monitor** (`src/telegram/chat_updates.rs`)
   - OS thread spawned via `thread::Builder`
   - Loop: `update_rx.recv_timeout(100ms)` for any `TdLibUpdate`
   - Sends `()` signal via `std::sync::mpsc::Sender` on any update
   - Stops when update channel closes (TdLibClient shutdown)
   - Logs update kind for debugging

4. **Event Source** (`src/ui/event_source.rs`)
   - `ChatUpdatesSignalSource` trait for testing
   - `ChannelChatUpdatesSignalSource` wraps `Receiver<()>`
   - `has_pending_refresh()` checks one signal per call
   - Emits `AppEvent::ChatListUpdateRequested` immediately when signal present

5. **Orchestrator** (`src/usecases/shell.rs`)
   - Handles `ChatListUpdateRequested` -> `refresh_chat_list()`
   - Preserves selection by `chat_id` after refresh

### Stop Mechanism

- `TdLibClient::close()` sets `AtomicBool` stop flag and sends TDLib `Close` function
- Update thread exits its `receive()` loop when stop flag is set
- Update channel sender is dropped, closing the channel
- Monitor thread exits when `recv_timeout` returns `Disconnected`
- On `Drop`, `TelegramChatUpdatesMonitor` logs shutdown (does not block on join)

## Debugging

Logs are written to `~/.config/rtg/rtg.log` (non-blocking, no TUI interference):

```bash
RUST_LOG=debug cargo run -- run
tail -f ~/.config/rtg/rtg.log
```

Key log patterns:
- `TELEGRAM_CHAT_UPDATES_MONITOR_STARTED` - monitor started
- `telegram update observed by chat monitor` - received update from TDLib
- `chat updates monitor requested chat list refresh` - signal sent
- `event source emitted chat list update request` - UI received signal
- `chat list refresh completed` - refresh done
- `TELEGRAM_CHAT_UPDATES_MONITOR_STOPPED` - monitor stopped (channel closed)

## Fairness and Safety

- Keyboard input has priority (polled first)
- Chat updates emit immediately when signal present (no streak cap currently)
- Connectivity events bounded by `MAX_CONNECTIVITY_STREAK`
- Monitor startup failure degrades gracefully (manual refresh works via `r` key)

## Risks and Trade-offs

1. **Unbounded refresh rate** - every TDLib update triggers full refresh
   - Impact: bursty traffic can cause refresh storms
   - Mitigation: see backlog item about coalescing signals

2. **No error backoff** - monitor logs warning and continues immediately
   - Impact: persistent errors can cause log spam
   - Mitigation: see backlog item about backoff

3. **Monitor thread not joined on drop** - thread may briefly outlive parent
   - Impact: minimal, thread exits when channel closes
   - Mitigation: see backlog item about joining thread in Drop

## Known Follow-ups

See `docs/internal/RTG_REVIEW_BACKLOG.md` for medium/minor items:

- Error backoff in monitor on repeated update failures
- Coalescing signal semantics to prevent refresh storms
- Fairness arbitration when connectivity and chat updates are both hot
- Expand update filter beyond current TDLib update types
- Thread joining on monitor drop for graceful shutdown

## Testing Notes

When modifying this subsystem:
1. Run `cargo test telegram` - covers TDLib client and monitor
2. Run `cargo test usecases::bootstrap` - covers monitor wiring
3. Run `cargo test usecases::shell` - covers refresh on event
4. Manual test: send message in Telegram, verify live refresh in TUI
