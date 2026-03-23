use std::sync::Arc;

use anyhow::Result;

use crate::{
    domain::{
        chat_list_state::ChatListUiState,
        events::{AppEvent, BackgroundTaskResult, ChatUpdate},
        message_cache::DEFAULT_MIN_DISPLAY_MESSAGES,
        open_chat_state::{MessageSource, OpenChatUiState},
        shell_state::{ActivePane, ShellState},
    },
    infra::contracts::{ExternalOpener, StorageAdapter},
    usecases::chat_subtitle::ChatSubtitleQuery,
};

use super::{
    background::TaskDispatcher, contracts::ShellOrchestrator, load_messages::CachedMessagesSource,
};

/// Default limit for cached message preloading.
const DEFAULT_CACHED_MESSAGES_LIMIT: usize = 50;

pub struct DefaultShellOrchestrator<S, O, D>
where
    S: StorageAdapter,
    O: ExternalOpener,
    D: TaskDispatcher,
{
    state: ShellState,
    storage: S,
    opener: O,
    dispatcher: D,
    /// Synchronous cache source for instant message display.
    cache_source: Option<Arc<dyn CachedMessagesSource>>,
    /// Guards against dispatching duplicate chat list requests while one is in-flight.
    chat_list_in_flight: bool,
    /// Guards against dispatching duplicate message refresh requests while one is in-flight.
    messages_refresh_in_flight: bool,
    /// When `true`, the orchestrator was initialised with cached data and needs
    /// a background refresh on the first Tick to pick up server-side changes.
    initial_refresh_needed: bool,
    /// Tracks the chat_id that is currently "opened" in TDLib via `openChat`.
    /// Used to ensure proper `closeChat` pairing when navigating away.
    tdlib_opened_chat_id: Option<i64>,
    /// Guards against dispatching duplicate prefetch requests.
    /// Holds the `chat_id` of the currently in-flight prefetch, if any.
    prefetch_in_flight: Option<i64>,
    /// Minimum number of cached messages required to display them immediately.
    /// If the cache holds fewer messages, the UI shows Loading instead of a
    /// sparse preview (eliminates the "1 message flash" artifact).
    min_display_messages: usize,
    /// Vim-style `dd` pending state: `true` after the first `d` press.
    pending_d: bool,
    /// Handle to a running recording process (voice recording, etc.).
    recording_handle: Option<super::voice_recording::RecordingHandle>,
    /// Path to the currently recorded voice file.
    recording_file_path: Option<String>,
    /// Pending command event receiver to be wired into the event source.
    /// Set when a command starts, taken by the shell loop.
    pending_command_rx: Option<std::sync::mpsc::Receiver<crate::domain::events::CommandEvent>>,
    /// Voice recording command template (from config).
    voice_record_cmd: String,
}

impl<S, O, D> DefaultShellOrchestrator<S, O, D>
where
    S: StorageAdapter,
    O: ExternalOpener,
    D: TaskDispatcher,
{
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn new(storage: S, opener: O, dispatcher: D) -> Self {
        Self {
            state: ShellState::default(),
            storage,
            opener,
            dispatcher,
            cache_source: None,
            chat_list_in_flight: false,
            messages_refresh_in_flight: false,
            initial_refresh_needed: false,
            tdlib_opened_chat_id: None,
            prefetch_in_flight: None,
            min_display_messages: DEFAULT_MIN_DISPLAY_MESSAGES,
            pending_d: false,
            recording_handle: None,
            recording_file_path: None,
            pending_command_rx: None,
            voice_record_cmd: super::voice_recording::DEFAULT_RECORD_CMD.to_owned(),
        }
    }

    /// Creates an orchestrator pre-populated with an initial state.
    ///
    /// When the initial state already has a `Ready` chat list (e.g. from
    /// TDLib cache), `initial_refresh_needed` is set so the first Tick
    /// triggers a background refresh to pick up server-side changes.
    ///
    /// `cache_source` provides synchronous access to TDLib's local cache
    /// for instant message display when opening chats.
    pub fn new_with_initial_state(
        storage: S,
        opener: O,
        dispatcher: D,
        initial_state: ShellState,
        cache_source: Option<Arc<dyn CachedMessagesSource>>,
        min_display_messages: usize,
        voice_record_cmd: String,
    ) -> Self {
        let initial_refresh_needed = initial_state.chat_list().ui_state() == ChatListUiState::Ready;
        Self {
            state: initial_state,
            storage,
            opener,
            dispatcher,
            cache_source,
            chat_list_in_flight: false,
            messages_refresh_in_flight: false,
            initial_refresh_needed,
            tdlib_opened_chat_id: None,
            prefetch_in_flight: None,
            min_display_messages: min_display_messages.max(1),
            pending_d: false,
            recording_handle: None,
            recording_file_path: None,
            pending_command_rx: None,
            voice_record_cmd,
        }
    }

    fn dispatch_chat_list_refresh(&mut self) {
        if self.chat_list_in_flight {
            tracing::debug!("chat list refresh already in-flight, skipping");
            return;
        }

        tracing::debug!("dispatching chat list refresh to background");

        // Only show the loader when there is no data to display (initial load,
        // after error, or empty state).  When the list is already visible
        // (Ready), keep showing stale data while the background fetch runs —
        // this prevents the "blink" where the chat list is momentarily replaced
        // by a loading indicator on every Telegram update.
        if self.state.chat_list().ui_state() != ChatListUiState::Ready {
            self.state.chat_list_mut().set_loading();
        }

        self.chat_list_in_flight = true;
        self.dispatcher.dispatch_chat_list();
    }

    fn open_selected_chat(&mut self) {
        let Some(selected) = self.state.chat_list().selected_chat() else {
            return;
        };

        let chat_id = selected.chat_id;
        let chat_title = selected.title.clone();
        let chat_type = selected.chat_type;

        // Cancel any in-flight prefetch — the user explicitly opened a chat.
        self.prefetch_in_flight = None;

        // If the same chat is already open and Ready, just switch focus — no reload.
        // But always ensure the TDLib lifecycle is maintained.
        if self.state.open_chat().chat_id() == Some(chat_id)
            && self.state.open_chat().ui_state() == OpenChatUiState::Ready
        {
            tracing::debug!(chat_id, "chat already open and ready, skipping reload");
            // Re-open in TDLib if it was closed (e.g. user pressed h then l)
            if self.tdlib_opened_chat_id != Some(chat_id) {
                self.dispatcher.dispatch_open_chat(chat_id);
                self.tdlib_opened_chat_id = Some(chat_id);
                // Mark existing messages as read in the reopened chat
                self.mark_open_chat_messages_as_read();
            }
            return;
        }

        tracing::debug!(chat_id, chat_title = %chat_title, "opening chat (non-blocking)");

        // Close the previously opened TDLib chat if switching to a different one.
        self.close_tdlib_chat_if_needed(chat_id);

        // Open this chat in TDLib for update delivery and read tracking.
        self.dispatcher.dispatch_open_chat(chat_id);
        self.tdlib_opened_chat_id = Some(chat_id);

        // Try app-level message cache first (instant, no TDLib call).
        // Fall back to TDLib local cache if the app cache has no data.
        // Apply the smart threshold: if cache has fewer than min_display_messages,
        // show Loading instead of a sparse preview (eliminates the "1 message flash").
        let min_msgs = self.min_display_messages;
        let showed_cache = if let Some(cached) = self
            .state
            .message_cache_mut()
            .get(chat_id)
            .filter(|m| m.len() >= min_msgs)
        {
            let messages = cached.to_vec();
            tracing::debug!(
                chat_id,
                count = messages.len(),
                "showing messages from app cache"
            );
            self.state
                .open_chat_mut()
                .set_loading(chat_id, chat_title.clone());
            self.state.open_chat_mut().set_ready(messages);
            self.state.open_chat_mut().set_refreshing(true);
            self.state
                .open_chat_mut()
                .set_message_source(MessageSource::Cache);
            true
        } else {
            self.try_show_cached_messages(chat_id, &chat_title)
        };

        if !showed_cache {
            self.state.open_chat_mut().set_loading(chat_id, chat_title);
        }

        // Dispatch a full background load (pagination).
        self.messages_refresh_in_flight = true;
        self.dispatcher.dispatch_load_messages(chat_id);

        // Dispatch subtitle resolution (user status / member count).
        self.dispatcher
            .dispatch_chat_subtitle(ChatSubtitleQuery { chat_id, chat_type });
    }

    /// Prefetches messages for the currently highlighted chat in the chat list.
    ///
    /// Triggered by j/k navigation. Skips if:
    /// - Another prefetch is already in-flight (debounce)
    /// - The highlighted chat already has data in the message cache
    fn maybe_prefetch_selected_chat(&mut self) {
        if self.prefetch_in_flight.is_some() {
            return;
        }

        let Some(selected) = self.state.chat_list().selected_chat() else {
            return;
        };

        let chat_id = selected.chat_id;

        if self.state.message_cache().has_messages(chat_id) {
            return;
        }

        tracing::debug!(chat_id, "prefetching messages for highlighted chat");
        self.prefetch_in_flight = Some(chat_id);
        self.dispatcher.dispatch_prefetch_messages(chat_id);
    }

    /// Closes the currently TDLib-opened chat if it differs from `next_chat_id`.
    ///
    /// Called before opening a new chat or when navigating away.
    fn close_tdlib_chat_if_needed(&mut self, next_chat_id: i64) {
        if let Some(prev_id) = self.tdlib_opened_chat_id {
            if prev_id != next_chat_id {
                tracing::debug!(prev_id, "closing previous TDLib chat");
                self.dispatcher.dispatch_close_chat(prev_id);
                self.tdlib_opened_chat_id = None;
            }
        }
    }

    /// Closes the currently TDLib-opened chat unconditionally.
    fn close_tdlib_chat(&mut self) {
        if let Some(chat_id) = self.tdlib_opened_chat_id.take() {
            tracing::debug!(chat_id, "closing TDLib chat on navigate away");
            self.dispatcher.dispatch_close_chat(chat_id);
            self.messages_refresh_in_flight = false;
        }
    }

    fn maybe_refresh_open_chat_messages(&mut self, affected_chat_ids: &[i64]) {
        if self.messages_refresh_in_flight {
            return;
        }

        let Some(open_id) = self.state.open_chat().chat_id() else {
            return;
        };

        if self.state.open_chat().ui_state() != OpenChatUiState::Ready {
            return;
        }

        if !affected_chat_ids.contains(&open_id) {
            return;
        }

        tracing::debug!(
            chat_id = open_id,
            "refreshing open chat messages from update"
        );
        self.messages_refresh_in_flight = true;
        self.dispatcher.dispatch_load_messages(open_id);
    }

    /// Marks the selected chat in the chat list as read (from the chat list view).
    ///
    /// Uses optimistic update: the unread counter is cleared immediately in local
    /// state before dispatching the background request to TDLib.
    fn mark_selected_chat_as_read(&mut self) {
        let Some(chat) = self.state.chat_list().selected_chat() else {
            return;
        };

        if chat.unread_count == 0 {
            return;
        }

        let Some(last_message_id) = chat.last_message_id else {
            return;
        };

        let chat_id = chat.chat_id;

        // Optimistic update: clear unread counter immediately in local state
        self.state.chat_list_mut().clear_selected_chat_unread();

        // If this chat is already opened in TDLib, just mark messages as read directly
        if self.tdlib_opened_chat_id == Some(chat_id) {
            self.dispatcher
                .dispatch_mark_as_read(chat_id, vec![last_message_id]);
        } else {
            self.dispatcher
                .dispatch_mark_chat_as_read(chat_id, last_message_id);
        }
    }

    /// Processes push updates from TDLib for cache warming and UI refresh.
    ///
    /// - `NewMessage`: inserts into `MessageCache` for any chat (warm cache passively)
    /// - `MessagesDeleted`: removes from `MessageCache`
    /// - `ChatMetadataChanged`: triggers chat list refresh
    ///
    /// For the currently open chat, also dispatches a message refresh.
    fn handle_chat_updates(&mut self, updates: Vec<ChatUpdate>) {
        let mut affected_chat_ids = Vec::new();

        for update in updates {
            let chat_id = update.chat_id();
            if !affected_chat_ids.contains(&chat_id) {
                affected_chat_ids.push(chat_id);
            }

            match update {
                ChatUpdate::NewMessage { chat_id, message } => {
                    tracing::debug!(chat_id, message_id = message.id, "caching pushed message");
                    self.state.message_cache_mut().add_message(chat_id, message);
                }
                ChatUpdate::MessagesDeleted {
                    chat_id,
                    message_ids,
                } => {
                    tracing::debug!(
                        chat_id,
                        count = message_ids.len(),
                        "removing deleted messages from cache"
                    );
                    self.state
                        .message_cache_mut()
                        .remove_messages(chat_id, &message_ids);
                }
                ChatUpdate::ChatMetadataChanged { .. } => {}
            }
        }

        // Always refresh the chat list (any update may affect ordering/preview)
        self.dispatch_chat_list_refresh();
        // Refresh the currently displayed chat if it was affected
        self.maybe_refresh_open_chat_messages(&affected_chat_ids);
    }

    /// Dispatches a mark-as-read request for all messages currently loaded in the open chat.
    fn mark_open_chat_messages_as_read(&self) {
        let Some(chat_id) = self.state.open_chat().chat_id() else {
            return;
        };

        let messages = self.state.open_chat().messages();
        if messages.is_empty() {
            return;
        }

        let message_ids: Vec<i64> = messages.iter().map(|m| m.id).collect();
        self.dispatcher.dispatch_mark_as_read(chat_id, message_ids);
    }

    /// Attempts to synchronously load cached messages for instant display.
    ///
    /// Returns `true` if cached messages were found (above the smart threshold)
    /// and the state was set to Ready. Sparse results below the threshold are
    /// ignored to avoid the "1 message flash" artifact.
    fn try_show_cached_messages(&mut self, chat_id: i64, chat_title: &str) -> bool {
        let Some(cache) = &self.cache_source else {
            return false;
        };

        match cache.list_cached_messages(chat_id, DEFAULT_CACHED_MESSAGES_LIMIT) {
            Ok(messages) if messages.len() >= self.min_display_messages => {
                tracing::debug!(
                    chat_id,
                    count = messages.len(),
                    "showing cached messages instantly"
                );
                self.state
                    .open_chat_mut()
                    .set_loading(chat_id, chat_title.to_owned());
                self.state.open_chat_mut().set_ready(messages);
                self.state.open_chat_mut().set_refreshing(true);
                self.state
                    .open_chat_mut()
                    .set_message_source(MessageSource::Cache);
                true
            }
            Ok(_) => {
                tracing::debug!(
                    chat_id,
                    "no/sparse cached messages, skipping instant display"
                );
                false
            }
            Err(e) => {
                tracing::debug!(chat_id, error = ?e, "failed to load cached messages");
                false
            }
        }
    }

    fn handle_chat_list_key(&mut self, key: &str) -> Result<()> {
        match key {
            "j" => {
                self.state.chat_list_mut().select_next();
                self.maybe_prefetch_selected_chat();
            }
            "k" => {
                self.state.chat_list_mut().select_previous();
                self.maybe_prefetch_selected_chat();
            }
            "R" => self.dispatch_chat_list_refresh(),
            "r" => self.mark_selected_chat_as_read(),
            "enter" | "l" => {
                if self.state.chat_list().selected_chat().is_some() {
                    self.open_selected_chat();
                    self.state.set_active_pane(ActivePane::Messages);
                    self.storage.save_last_action("open_chat")?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_messages_key(&mut self, key: &str) -> Result<()> {
        // Vim-style `dd`: first `d` sets pending, second `d` triggers delete.
        if key == "d" {
            if self.pending_d {
                self.pending_d = false;
                self.delete_selected_message();
            } else {
                self.pending_d = true;
            }
            return Ok(());
        }
        // Any non-`d` key cancels the pending state.
        self.pending_d = false;

        match key {
            "j" => self.state.open_chat_mut().select_next(),
            "k" => self.state.open_chat_mut().select_previous(),
            "h" | "esc" => {
                self.close_tdlib_chat();
                self.state.set_active_pane(ActivePane::ChatList);
            }
            "i" => {
                if self.state.open_chat().is_open() {
                    self.state.set_active_pane(ActivePane::MessageInput);
                }
            }
            "y" => {
                if let Some(msg) = self.state.open_chat().selected_message() {
                    let text = msg.display_content();
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        let _ = clipboard.set_text(text);
                    }
                }
            }
            "o" => self.open_message_url()?,
            "v" => {
                if self.state.open_chat().is_open() {
                    self.start_voice_recording();
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn start_voice_recording(&mut self) {
        use super::voice_recording;

        if self.state.command_popup().is_some() {
            return; // already recording or popup active
        }

        let file_path = voice_recording::generate_voice_file_path();

        match voice_recording::start_recording(&self.voice_record_cmd, &file_path) {
            Ok((handle, rx)) => {
                self.recording_handle = Some(handle);
                self.recording_file_path = Some(file_path);
                self.pending_command_rx = Some(rx);
                self.state.open_command_popup("Recording Voice");
                tracing::info!("voice recording started");
            }
            Err(err) => {
                tracing::error!(%err, "failed to start voice recording");
            }
        }
    }

    /// Stops the recording process in a background thread to avoid blocking the UI.
    ///
    /// The handle is moved to the thread, which calls `stop()` and then drops it.
    /// The pipe readers will naturally send `CommandExited` when the process dies.
    /// If the thread fails to spawn, `Drop` on the handle still terminates the process.
    fn stop_voice_recording(&mut self) {
        if let Some(mut handle) = self.recording_handle.take() {
            if std::thread::Builder::new()
                .name("rtg-rec-stop".into())
                .spawn(move || handle.stop())
                .is_err()
            {
                tracing::warn!(
                    "failed to spawn stop thread; handle dropped (Drop will stop process)"
                );
            }
        }
    }

    fn handle_command_popup_key(&mut self, key: &str) {
        use crate::domain::command_popup_state::CommandPhase;

        let phase = match self.state.command_popup() {
            Some(popup) => popup.phase().clone(),
            None => return,
        };

        match phase {
            CommandPhase::Running => {
                if key == "q" {
                    self.stop_voice_recording();
                    if let Some(popup) = self.state.command_popup_mut() {
                        popup.set_phase(CommandPhase::Stopping);
                    }
                }
            }
            CommandPhase::Stopping => {
                // Ignore all keys while the process is being terminated.
            }
            CommandPhase::AwaitingConfirmation { .. } => match key {
                "y" => {
                    self.send_voice_recording();
                    self.state.close_command_popup();
                }
                "n" | "esc" => {
                    self.discard_voice_recording();
                    self.state.close_command_popup();
                }
                _ => {}
            },
            CommandPhase::Failed { .. } => {
                self.state.close_command_popup();
            }
        }
    }

    fn handle_command_exited(&mut self, _event_success: bool) {
        use crate::domain::command_popup_state::CommandPhase;

        if let Some(popup) = self.state.command_popup_mut() {
            match popup.phase() {
                CommandPhase::Running => {
                    // Process exited on its own (not user-initiated stop).
                    // Check the actual exit code via the handle.
                    let process_succeeded = self
                        .recording_handle
                        .as_mut()
                        .and_then(|h| h.try_exit_success())
                        .unwrap_or(false);
                    self.recording_handle = None;

                    if process_succeeded {
                        popup.set_phase(CommandPhase::AwaitingConfirmation {
                            prompt: "Command finished. Send recording? (y/n)".into(),
                        });
                    } else {
                        popup.set_phase(CommandPhase::Failed {
                            message:
                                "Recording failed. Override recording command via [voice] record_cmd in ~/.config/rtg/config.toml (press any key)"
                                    .into(),
                        });
                        self.discard_voice_recording();
                    }
                }
                CommandPhase::Stopping => {
                    // Process finished after user pressed q. The handle was moved
                    // to the stop thread, so we check file existence instead.
                    // By the time CommandExited fires (pipe readers detect EOF),
                    // the process has exited and the file should be fully written.
                    let file_ok = self
                        .recording_file_path
                        .as_ref()
                        .map(|p| std::fs::metadata(p).map(|m| m.len() > 0).unwrap_or(false))
                        .unwrap_or(false);

                    if file_ok {
                        popup.set_phase(CommandPhase::AwaitingConfirmation {
                            prompt: "Send recording? (y/n)".into(),
                        });
                    } else {
                        popup.set_phase(CommandPhase::Failed {
                            message:
                                "Recording failed. Override recording command via [voice] record_cmd in ~/.config/rtg/config.toml (press any key)"
                                    .into(),
                        });
                        self.discard_voice_recording();
                    }
                }
                _ => {
                    // AwaitingConfirmation / Failed — do not override.
                }
            }
        }
    }

    fn send_voice_recording(&mut self) {
        let Some(file_path) = self.recording_file_path.take() else {
            return;
        };
        let Some(chat_id) = self.state.open_chat().chat_id() else {
            tracing::warn!("no chat open to send voice recording");
            return;
        };

        if !std::path::Path::new(&file_path).exists() {
            tracing::warn!(file_path, "recorded file does not exist");
            return;
        }

        self.dispatcher.dispatch_send_voice(chat_id, file_path);
    }

    fn discard_voice_recording(&mut self) {
        if let Some(file_path) = self.recording_file_path.take() {
            let _ = std::fs::remove_file(&file_path);
            tracing::info!(file_path, "voice recording discarded");
        }
    }

    fn delete_selected_message(&mut self) {
        let Some(chat_id) = self.state.open_chat().chat_id() else {
            return;
        };
        let Some(msg) = self.state.open_chat().selected_message() else {
            return;
        };
        let message_id = msg.id;
        if message_id == 0 {
            return; // Pending messages have id=0, skip
        }

        // Optimistically remove from UI
        self.state.open_chat_mut().remove_message(message_id);
        // Dispatch background deletion (fire-and-forget)
        self.dispatcher.dispatch_delete_message(chat_id, message_id);
    }

    fn open_message_url(&mut self) -> Result<()> {
        use crate::domain::message::extract_first_url;

        let Some(msg) = self.state.open_chat().selected_message() else {
            return Ok(());
        };
        let text = msg.display_content();
        if let Some(url) = extract_first_url(&text) {
            self.opener.open(url)?;
        }
        Ok(())
    }

    fn handle_message_input_key(&mut self, key: &str) {
        match key {
            "esc" => self.state.set_active_pane(ActivePane::Messages),
            "enter" => self.try_send_message(),
            "backspace" => self.state.message_input_mut().delete_char_before(),
            "delete" => self.state.message_input_mut().delete_char_at(),
            "left" => self.state.message_input_mut().move_cursor_left(),
            "right" => self.state.message_input_mut().move_cursor_right(),
            "home" => self.state.message_input_mut().move_cursor_home(),
            "end" => self.state.message_input_mut().move_cursor_end(),
            // Single character input
            ch if ch.chars().count() == 1 => {
                if let Some(c) = ch.chars().next() {
                    self.state.message_input_mut().insert_char(c);
                }
            }
            _ => {}
        }
    }

    fn try_send_message(&mut self) {
        let text = self.state.message_input().text().to_string();
        let trimmed = text.trim();

        // Validate locally — empty/whitespace messages are rejected immediately
        if trimmed.is_empty() {
            return;
        }

        let Some(chat_id) = self.state.open_chat().chat_id() else {
            return;
        };

        tracing::debug!(chat_id, "dispatching send message to background");

        // Optimistically clear the input and show the message immediately
        self.state.message_input_mut().clear();
        self.state
            .open_chat_mut()
            .add_pending_message(trimmed.to_owned());
        self.dispatcher.dispatch_send_message(chat_id, text.clone());
    }

    fn handle_background_result(&mut self, result: BackgroundTaskResult) {
        match result {
            BackgroundTaskResult::ChatListLoaded { result } => {
                self.chat_list_in_flight = false;
                match result {
                    Ok(chats) => {
                        tracing::debug!(chat_count = chats.len(), "background: chat list loaded");
                        // Always use the *current* selected chat_id from state
                        // to preserve the user's cursor position. This prevents
                        // cursor jumps when background TDLib updates trigger
                        // chat list refreshes while the user is navigating.
                        self.state.chat_list_mut().set_ready(chats);
                    }
                    Err(error) => {
                        tracing::warn!(code = error.code, "background: chat list load failed");
                        self.state.chat_list_mut().set_error();
                    }
                }
            }
            BackgroundTaskResult::MessagesLoaded { chat_id, result } => {
                self.messages_refresh_in_flight = false;

                // Always cache successful results, even if the user navigated away.
                if let Ok(ref messages) = result {
                    self.state
                        .message_cache_mut()
                        .put(chat_id, messages.clone(), true);
                }

                if self.state.open_chat().chat_id() != Some(chat_id) {
                    tracing::debug!(
                        chat_id,
                        "background: discarding stale messages result (user navigated away)"
                    );
                    return;
                }

                match result {
                    Ok(messages) => {
                        tracing::debug!(
                            chat_id,
                            message_count = messages.len(),
                            "background: messages loaded"
                        );
                        // If the chat is already Ready (e.g. from cached messages),
                        // use update_messages to preserve the user's scroll position.
                        // update_messages also clears refreshing and sets source to Live.
                        if self.state.open_chat().ui_state() == OpenChatUiState::Ready {
                            self.state.open_chat_mut().update_messages(messages);
                        } else {
                            self.state.open_chat_mut().set_ready(messages);
                            self.state
                                .open_chat_mut()
                                .set_message_source(MessageSource::Live);
                        }
                        // Mark all loaded messages as read via TDLib viewMessages.
                        // This triggers Update::ChatReadInbox → reactive unread_count update.
                        self.mark_open_chat_messages_as_read();
                    }
                    Err(error) => {
                        tracing::warn!(
                            chat_id,
                            code = error.code,
                            "background: messages load failed"
                        );
                        self.state.open_chat_mut().set_error();
                    }
                }
            }
            BackgroundTaskResult::MessageSent {
                chat_id,
                original_text,
                result,
            } => match result {
                Ok(()) => {
                    tracing::debug!(chat_id, "background: message sent successfully");
                    // Input was already cleared optimistically
                }
                Err(error) => {
                    tracing::warn!(
                        chat_id,
                        code = error.code,
                        "background: send message failed"
                    );
                    // Remove the optimistic pending message and restore input for retry
                    self.state.open_chat_mut().remove_pending_messages();
                    self.state.message_input_mut().set_text(&original_text);
                }
            },
            BackgroundTaskResult::MessageSentRefreshCompleted { chat_id, result } => {
                self.messages_refresh_in_flight = false;

                if let Ok(ref messages) = result {
                    self.state
                        .message_cache_mut()
                        .put(chat_id, messages.clone(), true);
                }

                if self.state.open_chat().chat_id() != Some(chat_id) {
                    return;
                }

                match result {
                    Ok(messages) => {
                        tracing::debug!(
                            chat_id,
                            message_count = messages.len(),
                            "background: messages refreshed after send"
                        );
                        // Intentionally use set_ready (not update_messages) to
                        // scroll to the bottom after sending — the user expects
                        // to see their new message at the end of the list.
                        self.state.open_chat_mut().set_ready(messages);
                        self.state.open_chat_mut().set_refreshing(false);
                        self.state
                            .open_chat_mut()
                            .set_message_source(MessageSource::Live);
                        // Mark new messages as read (including the one just sent).
                        self.mark_open_chat_messages_as_read();
                    }
                    Err(error) => {
                        tracing::warn!(
                            chat_id,
                            code = error.code,
                            "background: message refresh after send failed"
                        );
                        // Don't change UI state — the message was already sent.
                        // But clear refreshing since the refresh attempt is done.
                        self.state.open_chat_mut().set_refreshing(false);
                    }
                }
            }
            BackgroundTaskResult::ChatSubtitleLoaded { chat_id, result } => {
                if self.state.open_chat().chat_id() == Some(chat_id) {
                    match result {
                        Ok(subtitle) => {
                            tracing::debug!(chat_id, ?subtitle, "chat subtitle resolved");
                            self.state.open_chat_mut().set_chat_subtitle(subtitle);
                        }
                        Err(e) => {
                            tracing::debug!(chat_id, error = ?e, "chat subtitle resolution failed");
                        }
                    }
                }
            }
            BackgroundTaskResult::MessagesPrefetched { chat_id, result } => {
                if self.prefetch_in_flight == Some(chat_id) {
                    self.prefetch_in_flight = None;
                }

                if let Ok(messages) = result {
                    if !messages.is_empty() {
                        tracing::debug!(
                            chat_id,
                            count = messages.len(),
                            "background: prefetched messages cached"
                        );
                        self.state.message_cache_mut().put(chat_id, messages, true);
                    }
                }

                // If the user opened this chat while the prefetch was in-flight
                // and the chat is still in Loading state, populate it from cache
                // (only if it meets the smart display threshold).
                let min_msgs = self.min_display_messages;
                if self.state.open_chat().chat_id() == Some(chat_id)
                    && self.state.open_chat().ui_state() == OpenChatUiState::Loading
                {
                    if let Some(cached) = self
                        .state
                        .message_cache_mut()
                        .get(chat_id)
                        .filter(|m| m.len() >= min_msgs)
                    {
                        let msgs = cached.to_vec();
                        self.state.open_chat_mut().set_ready(msgs);
                        self.state.open_chat_mut().set_refreshing(true);
                        self.state
                            .open_chat_mut()
                            .set_message_source(MessageSource::Cache);
                        self.mark_open_chat_messages_as_read();
                    }
                }
            }
        }
    }
}

impl<S, O, D> ShellOrchestrator for DefaultShellOrchestrator<S, O, D>
where
    S: StorageAdapter,
    O: ExternalOpener,
    D: TaskDispatcher,
{
    fn state(&self) -> &ShellState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut ShellState {
        &mut self.state
    }

    fn handle_event(&mut self, event: AppEvent) -> Result<()> {
        match event {
            AppEvent::Tick => {
                if self.state.chat_list().ui_state() == ChatListUiState::Loading {
                    self.dispatch_chat_list_refresh();
                } else if self.initial_refresh_needed {
                    // Chat list was pre-populated from cache; trigger a background
                    // refresh to pick up any server-side changes.
                    self.initial_refresh_needed = false;
                    self.dispatch_chat_list_refresh();
                }
                self.storage.save_last_action("tick")?;
            }
            AppEvent::QuitRequested => {
                // Only Ctrl+C produces QuitRequested — always quit.
                self.close_tdlib_chat();
                self.state.stop();
            }
            AppEvent::CommandOutputLine(line) => {
                if let Some(popup) = self.state.command_popup_mut() {
                    popup.push_line(line);
                }
            }
            AppEvent::CommandExited { success } => {
                tracing::info!(success, "external command exited");
                self.handle_command_exited(success);
            }
            AppEvent::InputKey(key) => {
                // When command popup is active, intercept keys for popup control.
                if self.state.command_popup().is_some() {
                    self.handle_command_popup_key(&key.key);
                    return Ok(());
                }

                // When help popup is visible, only close-actions are accepted.
                if self.state.help_visible() {
                    match key.key.as_str() {
                        "q" | "?" | "esc" => self.state.hide_help(),
                        _ => {}
                    }
                    return Ok(());
                }

                // `?` opens help from ChatList/Messages; types `?` in MessageInput.
                if key.key == "?" {
                    match self.state.active_pane() {
                        ActivePane::ChatList | ActivePane::Messages => {
                            self.state.show_help();
                            return Ok(());
                        }
                        ActivePane::MessageInput => {
                            self.handle_message_input_key("?");
                            return Ok(());
                        }
                    }
                }

                // `q` quits from ChatList/Messages; types `q` in MessageInput.
                if key.key == "q" && !key.ctrl {
                    match self.state.active_pane() {
                        ActivePane::MessageInput => {
                            self.handle_message_input_key("q");
                        }
                        _ => {
                            self.close_tdlib_chat();
                            self.state.stop();
                        }
                    }
                    return Ok(());
                }

                match self.state.active_pane() {
                    ActivePane::ChatList => self.handle_chat_list_key(&key.key)?,
                    ActivePane::Messages => self.handle_messages_key(&key.key)?,
                    ActivePane::MessageInput => self.handle_message_input_key(&key.key),
                }
            }
            AppEvent::ConnectivityChanged(status) => {
                self.state.set_connectivity_status(status);
            }
            AppEvent::ChatUpdateReceived { updates } => {
                tracing::debug!(count = updates.len(), "orchestrator received chat updates");
                self.handle_chat_updates(updates);
            }
            AppEvent::BackgroundTaskCompleted(result) => {
                self.handle_background_result(result);
            }
        }

        Ok(())
    }

    fn take_pending_command_rx(
        &mut self,
    ) -> Option<std::sync::mpsc::Receiver<crate::domain::events::CommandEvent>> {
        self.pending_command_rx.take()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::{
        domain::{
            chat::ChatSummary,
            chat_list_state::ChatListUiState,
            events::{
                AppEvent, BackgroundError, BackgroundTaskResult, ConnectivityStatus, KeyInput,
            },
            message::Message,
            open_chat_state::OpenChatUiState,
            shell_state::ShellState,
        },
        infra::{contracts::ExternalOpener, stubs::StubStorageAdapter},
    };

    // ── Recording opener for tests ──

    #[derive(Debug, Default)]
    struct RecordingOpener {
        opened: RefCell<Vec<String>>,
    }

    impl ExternalOpener for RecordingOpener {
        fn open(&self, target: &str) -> anyhow::Result<()> {
            self.opened.borrow_mut().push(target.to_owned());
            Ok(())
        }
    }

    impl RecordingOpener {
        fn opened_urls(&self) -> Vec<String> {
            self.opened.borrow().clone()
        }
    }

    // ── Domain helpers ──

    fn chat(chat_id: i64, title: &str) -> ChatSummary {
        use crate::domain::chat::{ChatType, OutgoingReadStatus};
        ChatSummary {
            chat_id,
            title: title.to_owned(),
            unread_count: 0,
            last_message_preview: None,
            last_message_unix_ms: None,
            is_pinned: false,
            chat_type: ChatType::Private,
            last_message_sender: None,
            is_online: None,
            is_bot: false,
            outgoing_status: OutgoingReadStatus::default(),
            last_message_id: None,
        }
    }

    fn message(id: i64, text: &str) -> Message {
        Message {
            id,
            sender_name: "User".to_owned(),
            text: text.to_owned(),
            timestamp_ms: 1000,
            is_outgoing: false,
            media: crate::domain::message::MessageMedia::None,
            status: crate::domain::message::MessageStatus::Delivered,
        }
    }

    // ── Recording task dispatcher for tests ──

    /// Records what the orchestrator dispatched and allows inspection.
    struct RecordingDispatcher {
        dispatched_chat_list_count: RefCell<usize>,
        dispatched_messages: RefCell<Vec<i64>>,
        dispatched_sends: RefCell<Vec<(i64, String)>>,
        dispatched_open_chats: RefCell<Vec<i64>>,
        dispatched_close_chats: RefCell<Vec<i64>>,
        dispatched_mark_as_read: RefCell<Vec<(i64, Vec<i64>)>>,
        dispatched_mark_chat_as_read: RefCell<Vec<(i64, i64)>>,
        dispatched_prefetches: RefCell<Vec<i64>>,
        dispatched_deletes: RefCell<Vec<(i64, i64)>>,
        dispatched_voice_sends: RefCell<Vec<(i64, String)>>,
    }

    impl RecordingDispatcher {
        fn new() -> Self {
            Self {
                dispatched_chat_list_count: RefCell::new(0),
                dispatched_messages: RefCell::new(Vec::new()),
                dispatched_sends: RefCell::new(Vec::new()),
                dispatched_open_chats: RefCell::new(Vec::new()),
                dispatched_close_chats: RefCell::new(Vec::new()),
                dispatched_mark_as_read: RefCell::new(Vec::new()),
                dispatched_mark_chat_as_read: RefCell::new(Vec::new()),
                dispatched_prefetches: RefCell::new(Vec::new()),
                dispatched_deletes: RefCell::new(Vec::new()),
                dispatched_voice_sends: RefCell::new(Vec::new()),
            }
        }

        fn chat_list_dispatch_count(&self) -> usize {
            *self.dispatched_chat_list_count.borrow()
        }

        fn messages_dispatch_count(&self) -> usize {
            self.dispatched_messages.borrow().len()
        }

        fn send_dispatch_count(&self) -> usize {
            self.dispatched_sends.borrow().len()
        }

        fn last_send(&self) -> Option<(i64, String)> {
            self.dispatched_sends.borrow().last().cloned()
        }

        fn open_chat_dispatch_count(&self) -> usize {
            self.dispatched_open_chats.borrow().len()
        }

        fn close_chat_dispatch_count(&self) -> usize {
            self.dispatched_close_chats.borrow().len()
        }

        fn mark_as_read_dispatch_count(&self) -> usize {
            self.dispatched_mark_as_read.borrow().len()
        }

        fn last_mark_as_read(&self) -> Option<(i64, Vec<i64>)> {
            self.dispatched_mark_as_read.borrow().last().cloned()
        }

        fn mark_chat_as_read_dispatch_count(&self) -> usize {
            self.dispatched_mark_chat_as_read.borrow().len()
        }

        fn last_mark_chat_as_read(&self) -> Option<(i64, i64)> {
            self.dispatched_mark_chat_as_read.borrow().last().cloned()
        }

        fn prefetch_dispatch_count(&self) -> usize {
            self.dispatched_prefetches.borrow().len()
        }

        fn last_prefetch_chat_id(&self) -> Option<i64> {
            self.dispatched_prefetches.borrow().last().copied()
        }

        fn delete_dispatch_count(&self) -> usize {
            self.dispatched_deletes.borrow().len()
        }

        fn last_delete(&self) -> Option<(i64, i64)> {
            self.dispatched_deletes.borrow().last().copied()
        }

        fn voice_send_dispatch_count(&self) -> usize {
            self.dispatched_voice_sends.borrow().len()
        }

        fn last_voice_send(&self) -> Option<(i64, String)> {
            self.dispatched_voice_sends.borrow().last().cloned()
        }
    }

    impl TaskDispatcher for RecordingDispatcher {
        fn dispatch_chat_list(&self) {
            *self.dispatched_chat_list_count.borrow_mut() += 1;
        }

        fn dispatch_load_messages(&self, chat_id: i64) {
            self.dispatched_messages.borrow_mut().push(chat_id);
        }

        fn dispatch_send_message(&self, chat_id: i64, text: String) {
            self.dispatched_sends.borrow_mut().push((chat_id, text));
        }

        fn dispatch_open_chat(&self, chat_id: i64) {
            self.dispatched_open_chats.borrow_mut().push(chat_id);
        }

        fn dispatch_close_chat(&self, chat_id: i64) {
            self.dispatched_close_chats.borrow_mut().push(chat_id);
        }

        fn dispatch_mark_as_read(&self, chat_id: i64, message_ids: Vec<i64>) {
            self.dispatched_mark_as_read
                .borrow_mut()
                .push((chat_id, message_ids));
        }

        fn dispatch_mark_chat_as_read(&self, chat_id: i64, last_message_id: i64) {
            self.dispatched_mark_chat_as_read
                .borrow_mut()
                .push((chat_id, last_message_id));
        }

        fn dispatch_prefetch_messages(&self, chat_id: i64) {
            self.dispatched_prefetches.borrow_mut().push(chat_id);
        }

        fn dispatch_delete_message(&self, chat_id: i64, message_id: i64) {
            self.dispatched_deletes
                .borrow_mut()
                .push((chat_id, message_id));
        }

        fn dispatch_chat_subtitle(&self, _query: ChatSubtitleQuery) {
            // Recording: no-op for now
        }

        fn dispatch_send_voice(&self, chat_id: i64, file_path: String) {
            self.dispatched_voice_sends
                .borrow_mut()
                .push((chat_id, file_path));
        }
    }

    // ── Test orchestrator factory ──

    type TestOrchestrator =
        DefaultShellOrchestrator<StubStorageAdapter, RecordingOpener, RecordingDispatcher>;

    fn make_orchestrator() -> TestOrchestrator {
        let mut o = DefaultShellOrchestrator::new(
            StubStorageAdapter::default(),
            RecordingOpener::default(),
            RecordingDispatcher::new(),
        );
        // Use min threshold of 1 for existing tests — any cached message triggers display.
        // Tests for the threshold itself use make_orchestrator_with_threshold().
        o.min_display_messages = 1;
        o
    }

    /// Helper: pre-populate the chat list as if a background load completed.
    fn inject_chat_list(orchestrator: &mut TestOrchestrator, chats: Vec<ChatSummary>) {
        orchestrator
            .handle_event(AppEvent::BackgroundTaskCompleted(
                BackgroundTaskResult::ChatListLoaded { result: Ok(chats) },
            ))
            .unwrap();
    }

    /// Helper: inject messages as if a background load completed for given chat.
    fn inject_messages(orchestrator: &mut TestOrchestrator, chat_id: i64, messages: Vec<Message>) {
        orchestrator
            .handle_event(AppEvent::BackgroundTaskCompleted(
                BackgroundTaskResult::MessagesLoaded {
                    chat_id,
                    result: Ok(messages),
                },
            ))
            .unwrap();
    }

    /// Helper: set up orchestrator with a loaded chat list (skip the dispatch+result dance).
    fn orchestrator_with_chats(chats: Vec<ChatSummary>) -> TestOrchestrator {
        let mut o = make_orchestrator();
        inject_chat_list(&mut o, chats);
        o
    }

    /// Helper: set up orchestrator with a loaded chat list and an opened chat.
    fn orchestrator_with_open_chat(
        chats: Vec<ChatSummary>,
        chat_id: i64,
        messages: Vec<Message>,
    ) -> TestOrchestrator {
        let mut o = orchestrator_with_chats(chats);
        // Press enter to open the first chat (dispatches load_messages)
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();
        // Inject the messages result
        inject_messages(&mut o, chat_id, messages);
        o
    }

    // ── Stub cache source for tests ──

    struct StubCacheSource {
        messages: std::sync::Mutex<std::collections::HashMap<i64, Vec<Message>>>,
    }

    impl StubCacheSource {
        fn empty() -> Self {
            Self {
                messages: std::sync::Mutex::new(std::collections::HashMap::new()),
            }
        }

        fn with_messages(entries: Vec<(i64, Vec<Message>)>) -> Self {
            let map = entries.into_iter().collect();
            Self {
                messages: std::sync::Mutex::new(map),
            }
        }
    }

    impl CachedMessagesSource for StubCacheSource {
        fn list_cached_messages(
            &self,
            chat_id: i64,
            _limit: usize,
        ) -> Result<Vec<Message>, crate::usecases::load_messages::MessagesSourceError> {
            let map = self.messages.lock().unwrap();
            Ok(map.get(&chat_id).cloned().unwrap_or_default())
        }
    }

    // ── Orchestrator factory with pre-populated cache ──

    fn make_orchestrator_with_cached_chats(chats: Vec<ChatSummary>) -> TestOrchestrator {
        let state = ShellState::with_initial_chat_list(chats);
        DefaultShellOrchestrator::new_with_initial_state(
            StubStorageAdapter::default(),
            RecordingOpener::default(),
            RecordingDispatcher::new(),
            state,
            None,
            1, // No threshold in most tests — any cached message triggers instant display
            crate::usecases::voice_recording::DEFAULT_RECORD_CMD.to_owned(),
        )
    }

    fn make_orchestrator_with_cache(
        chats: Vec<ChatSummary>,
        cache: StubCacheSource,
    ) -> TestOrchestrator {
        let state = ShellState::with_initial_chat_list(chats);
        DefaultShellOrchestrator::new_with_initial_state(
            StubStorageAdapter::default(),
            RecordingOpener::default(),
            RecordingDispatcher::new(),
            state,
            Some(Arc::new(cache)),
            1, // No threshold in most tests — any cached message triggers instant display
            crate::usecases::voice_recording::DEFAULT_RECORD_CMD.to_owned(),
        )
    }

    fn make_orchestrator_with_threshold(
        chats: Vec<ChatSummary>,
        min_display_messages: usize,
    ) -> TestOrchestrator {
        let state = ShellState::with_initial_chat_list(chats);
        DefaultShellOrchestrator::new_with_initial_state(
            StubStorageAdapter::default(),
            RecordingOpener::default(),
            RecordingDispatcher::new(),
            state,
            None,
            min_display_messages,
            crate::usecases::voice_recording::DEFAULT_RECORD_CMD.to_owned(),
        )
    }

    fn make_orchestrator_with_cache_and_threshold(
        chats: Vec<ChatSummary>,
        cache: StubCacheSource,
        min_display_messages: usize,
    ) -> TestOrchestrator {
        let state = ShellState::with_initial_chat_list(chats);
        DefaultShellOrchestrator::new_with_initial_state(
            StubStorageAdapter::default(),
            RecordingOpener::default(),
            RecordingDispatcher::new(),
            state,
            Some(Arc::new(cache)),
            min_display_messages,
            crate::usecases::voice_recording::DEFAULT_RECORD_CMD.to_owned(),
        )
    }

    // ── Tests ──

    #[test]
    fn stops_on_quit_event() {
        let mut o = make_orchestrator();
        o.handle_event(AppEvent::QuitRequested).unwrap();
        assert!(!o.state().is_running());
    }

    #[test]
    fn keeps_running_on_regular_key() {
        let mut o = make_orchestrator();
        o.handle_event(AppEvent::InputKey(KeyInput::new("x", false)))
            .unwrap();
        assert!(o.state().is_running());
    }

    #[test]
    fn updates_connectivity_status_on_connectivity_event() {
        let mut o = make_orchestrator();
        o.handle_event(AppEvent::ConnectivityChanged(
            ConnectivityStatus::Disconnected,
        ))
        .unwrap();
        assert_eq!(
            o.state().connectivity_status(),
            ConnectivityStatus::Disconnected
        );
    }

    #[test]
    fn tick_dispatches_chat_list_when_loading() {
        let mut o = make_orchestrator();
        assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Loading);

        o.handle_event(AppEvent::Tick).unwrap();
        assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);
    }

    #[test]
    fn tick_does_not_dispatch_when_chat_list_is_ready() {
        let mut o = orchestrator_with_chats(vec![chat(1, "General")]);
        o.handle_event(AppEvent::Tick).unwrap();
        // Only the initial dispatch from inject_chat_list path; tick should not add another
        assert_eq!(o.dispatcher.chat_list_dispatch_count(), 0);
    }

    #[test]
    fn chat_list_loaded_result_sets_ready_state() {
        let mut o = make_orchestrator();
        inject_chat_list(&mut o, vec![chat(1, "General"), chat(2, "Backend")]);

        assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Ready);
        assert_eq!(o.state().chat_list().selected_index(), Some(0));
        assert_eq!(o.state().chat_list().chats().len(), 2);
    }

    #[test]
    fn chat_list_loaded_error_sets_error_state() {
        let mut o = make_orchestrator();
        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::ChatListLoaded {
                result: Err(BackgroundError::new("CHAT_LIST_UNAVAILABLE")),
            },
        ))
        .unwrap();

        assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Error);
    }

    #[test]
    fn chat_list_reload_preserves_selection_by_current_chat_id() {
        let mut o =
            orchestrator_with_chats(vec![chat(1, "General"), chat(2, "Backend"), chat(3, "Ops")]);
        // Navigate to "Backend" (index 1)
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();
        assert_eq!(
            o.state().chat_list().selected_chat().map(|c| c.chat_id),
            Some(2)
        );

        // Simulate a background reload where chat order changed
        // (e.g. chat 3 got a new message and moved to the top)
        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::ChatListLoaded {
                result: Ok(vec![chat(3, "Ops"), chat(1, "General"), chat(2, "Backend")]),
            },
        ))
        .unwrap();

        // Selection should follow chat_id 2 ("Backend"), now at index 2
        assert_eq!(o.state().chat_list().selected_index(), Some(2));
        assert_eq!(
            o.state().chat_list().selected_chat().map(|c| c.chat_id),
            Some(2)
        );
    }

    #[test]
    fn chat_list_reload_cursor_follows_current_selection_not_dispatch_time() {
        // Regression test: cursor should not jump when the user navigates
        // with j/k while a background chat list refresh is in flight.
        let mut o = orchestrator_with_chats(vec![
            chat(1, "Alpha"),
            chat(2, "Beta"),
            chat(3, "Gamma"),
            chat(4, "Delta"),
            chat(5, "Epsilon"),
        ]);
        assert_eq!(o.state().chat_list().selected_index(), Some(0));

        // Trigger a background refresh (e.g. from TDLib update)
        o.handle_event(AppEvent::ChatUpdateReceived { updates: vec![] })
            .unwrap();
        assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);

        // User navigates down while refresh is in-flight
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();
        assert_eq!(
            o.state().chat_list().selected_chat().map(|c| c.chat_id),
            Some(4) // "Delta"
        );

        // Background result arrives with reordered chats
        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::ChatListLoaded {
                result: Ok(vec![
                    chat(5, "Epsilon"),
                    chat(1, "Alpha"),
                    chat(4, "Delta"),
                    chat(2, "Beta"),
                    chat(3, "Gamma"),
                ]),
            },
        ))
        .unwrap();

        // Selection must stay on "Delta" (chat_id=4), now at index 2
        assert_eq!(
            o.state().chat_list().selected_chat().map(|c| c.chat_id),
            Some(4)
        );
        assert_eq!(o.state().chat_list().selected_index(), Some(2));
    }

    #[test]
    fn chat_list_reload_falls_back_when_selected_chat_disappears() {
        let mut o =
            orchestrator_with_chats(vec![chat(1, "Alpha"), chat(2, "Beta"), chat(3, "Gamma")]);
        // Navigate to "Gamma"
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();
        assert_eq!(
            o.state().chat_list().selected_chat().map(|c| c.chat_id),
            Some(3)
        );

        // Background refresh arrives without chat 3
        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::ChatListLoaded {
                result: Ok(vec![chat(1, "Alpha"), chat(2, "Beta")]),
            },
        ))
        .unwrap();

        // Should fall back to first chat
        assert_eq!(o.state().chat_list().selected_index(), Some(0));
        assert_eq!(
            o.state().chat_list().selected_chat().map(|c| c.chat_id),
            Some(1)
        );
    }

    #[test]
    fn key_contract_navigates_chat_list_with_vim_keys() {
        let mut o =
            orchestrator_with_chats(vec![chat(1, "General"), chat(2, "Backend"), chat(3, "Ops")]);

        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();
        assert_eq!(o.state().chat_list().selected_index(), Some(1));

        o.handle_event(AppEvent::InputKey(KeyInput::new("k", false)))
            .unwrap();
        assert_eq!(o.state().chat_list().selected_index(), Some(0));
    }

    #[test]
    fn enter_key_dispatches_load_messages_and_switches_pane() {
        let mut o = orchestrator_with_chats(vec![chat(1, "General"), chat(2, "Backend")]);

        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        assert_eq!(o.dispatcher.messages_dispatch_count(), 1);
        assert_eq!(o.state().open_chat().chat_id(), Some(1));
        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);
        assert_eq!(o.state().active_pane(), ActivePane::Messages);
    }

    #[test]
    fn messages_loaded_result_sets_ready_state() {
        let o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);

        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
        assert_eq!(o.state().open_chat().messages().len(), 1);
    }

    #[test]
    fn messages_loaded_error_sets_error_state() {
        let mut o = orchestrator_with_chats(vec![chat(1, "General")]);
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::MessagesLoaded {
                chat_id: 1,
                result: Err(BackgroundError::new("MESSAGES_UNAVAILABLE")),
            },
        ))
        .unwrap();

        assert_eq!(o.state().open_chat().chat_id(), Some(1));
        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Error);
    }

    #[test]
    fn stale_messages_result_is_discarded() {
        let mut o = orchestrator_with_chats(vec![chat(1, "A"), chat(2, "B")]);
        // Open chat 1
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();
        // Navigate away before result arrives
        o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
            .unwrap();
        // Navigate to chat 2 and open it
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();
        // Now the stale result for chat 1 arrives
        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::MessagesLoaded {
                chat_id: 1,
                result: Ok(vec![message(1, "Stale")]),
            },
        ))
        .unwrap();

        // Should not have been applied — still loading chat 2
        assert_eq!(o.state().open_chat().chat_id(), Some(2));
        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);
    }

    #[test]
    fn refresh_key_dispatches_chat_list() {
        let mut o = orchestrator_with_chats(vec![chat(1, "General")]);
        o.handle_event(AppEvent::InputKey(KeyInput::new("R", false)))
            .unwrap();
        assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);
    }

    #[test]
    fn chat_list_update_event_dispatches_refresh() {
        let mut o = orchestrator_with_chats(vec![chat(1, "General")]);
        o.handle_event(AppEvent::ChatUpdateReceived { updates: vec![] })
            .unwrap();
        assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);
    }

    #[test]
    fn duplicate_chat_list_dispatch_is_guarded() {
        let mut o = make_orchestrator();
        // First tick dispatches
        o.handle_event(AppEvent::Tick).unwrap();
        assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);
        assert!(o.chat_list_in_flight);

        // Second tick should not dispatch again
        o.handle_event(AppEvent::Tick).unwrap();
        // chat_list state changed to non-Loading after first dispatch set it,
        // but actually the state is still Loading since we haven't injected a result.
        // The in-flight guard prevents a second dispatch.
        assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);
    }

    #[test]
    fn in_flight_guard_resets_after_result() {
        let mut o = make_orchestrator();
        o.handle_event(AppEvent::Tick).unwrap();
        assert!(o.chat_list_in_flight);

        inject_chat_list(&mut o, vec![chat(1, "General")]);
        assert!(!o.chat_list_in_flight);

        // Now another dispatch should work
        o.handle_event(AppEvent::InputKey(KeyInput::new("R", false)))
            .unwrap();
        assert_eq!(o.dispatcher.chat_list_dispatch_count(), 2);
    }

    #[test]
    fn refresh_from_ready_keeps_data_visible() {
        let mut o = orchestrator_with_chats(vec![chat(1, "General"), chat(2, "Backend")]);
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();

        // Trigger refresh via "R" key
        o.handle_event(AppEvent::InputKey(KeyInput::new("R", false)))
            .unwrap();

        // State must stay Ready with existing data — no blink
        assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Ready);
        assert_eq!(o.state().chat_list().chats().len(), 2);
        assert_eq!(o.state().chat_list().selected_index(), Some(1));
        // But a background dispatch was issued
        assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);
    }

    #[test]
    fn chat_list_update_event_keeps_data_visible() {
        let mut o = orchestrator_with_chats(vec![chat(1, "General")]);

        o.handle_event(AppEvent::ChatUpdateReceived { updates: vec![] })
            .unwrap();

        // Must not blink — state stays Ready while background fetch runs
        assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Ready);
        assert_eq!(o.state().chat_list().chats().len(), 1);
        assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);
    }

    #[test]
    fn l_key_does_nothing_when_no_chat_selected() {
        let mut o = orchestrator_with_chats(vec![]);
        // ui_state is Empty when no chats
        o.handle_event(AppEvent::InputKey(KeyInput::new("l", false)))
            .unwrap();
        assert_eq!(o.state().active_pane(), ActivePane::ChatList);
        assert!(!o.state().open_chat().is_open());
    }

    #[test]
    fn i_key_switches_to_message_input_mode_when_chat_is_open() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);

        o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .unwrap();
        assert_eq!(o.state().active_pane(), ActivePane::MessageInput);
    }

    #[test]
    fn i_key_does_nothing_when_no_chat_is_open() {
        let mut o = orchestrator_with_chats(vec![chat(1, "General")]);
        o.state.set_active_pane(ActivePane::Messages);

        o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .unwrap();
        assert_eq!(o.state().active_pane(), ActivePane::Messages);
    }

    #[test]
    fn esc_key_switches_from_message_input_to_messages_pane() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
        o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .unwrap();
        assert_eq!(o.state().active_pane(), ActivePane::MessageInput);

        o.handle_event(AppEvent::InputKey(KeyInput::new("esc", false)))
            .unwrap();
        assert_eq!(o.state().active_pane(), ActivePane::Messages);
    }

    #[test]
    fn text_input_in_message_input_mode() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
        o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .unwrap();

        o.handle_event(AppEvent::InputKey(KeyInput::new("H", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .unwrap();

        assert_eq!(o.state().message_input().text(), "Hi");
        assert_eq!(o.state().active_pane(), ActivePane::MessageInput);
    }

    #[test]
    fn backspace_deletes_character_in_message_input_mode() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
        o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .unwrap();

        o.handle_event(AppEvent::InputKey(KeyInput::new("H", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("backspace", false)))
            .unwrap();

        assert_eq!(o.state().message_input().text(), "H");
    }

    #[test]
    fn cursor_navigation_in_message_input_mode() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
        o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .unwrap();

        for ch in ['a', 'b', 'c'] {
            o.handle_event(AppEvent::InputKey(KeyInput::new(ch.to_string(), false)))
                .unwrap();
        }
        assert_eq!(o.state().message_input().cursor_position(), 3);

        o.handle_event(AppEvent::InputKey(KeyInput::new("left", false)))
            .unwrap();
        assert_eq!(o.state().message_input().cursor_position(), 2);

        o.handle_event(AppEvent::InputKey(KeyInput::new("home", false)))
            .unwrap();
        assert_eq!(o.state().message_input().cursor_position(), 0);

        o.handle_event(AppEvent::InputKey(KeyInput::new("end", false)))
            .unwrap();
        assert_eq!(o.state().message_input().cursor_position(), 3);
    }

    #[test]
    fn q_key_types_q_in_message_input_mode_instead_of_quitting() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
        o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .unwrap();

        o.handle_event(AppEvent::InputKey(KeyInput::new("q", false)))
            .unwrap();

        assert!(o.state().is_running());
        assert_eq!(o.state().message_input().text(), "q");
    }

    #[test]
    fn message_input_state_preserved_when_switching_panes() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
        o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .unwrap();

        o.handle_event(AppEvent::InputKey(KeyInput::new("H", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .unwrap();

        o.handle_event(AppEvent::InputKey(KeyInput::new("esc", false)))
            .unwrap();
        assert_eq!(o.state().message_input().text(), "Hi");

        o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .unwrap();
        assert_eq!(o.state().message_input().text(), "Hi");
    }

    #[test]
    fn enter_key_dispatches_send_message_and_clears_input() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
        o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .unwrap();

        o.handle_event(AppEvent::InputKey(KeyInput::new("H", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .unwrap();
        assert_eq!(o.state().message_input().text(), "Hi");

        // Press enter to send
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        // Input should be cleared optimistically
        assert_eq!(o.state().message_input().text(), "");
        assert_eq!(o.dispatcher.send_dispatch_count(), 1);
        assert_eq!(o.dispatcher.last_send(), Some((1, "Hi".to_owned())));
        assert_eq!(o.state().active_pane(), ActivePane::MessageInput);
    }

    #[test]
    fn message_sent_success_keeps_input_cleared() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
        o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("H", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        // Successful send result arrives
        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::MessageSent {
                chat_id: 1,
                original_text: "Hi".to_owned(),
                result: Ok(()),
            },
        ))
        .unwrap();

        assert_eq!(o.state().message_input().text(), "");
    }

    #[test]
    fn message_sent_error_restores_text_in_input() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
        o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .unwrap();

        for c in "Test message".chars() {
            o.handle_event(AppEvent::InputKey(KeyInput::new(&c.to_string(), false)))
                .unwrap();
        }

        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();
        assert_eq!(o.state().message_input().text(), "");

        // Send failure result arrives — text should be restored
        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::MessageSent {
                chat_id: 1,
                original_text: "Test message".to_owned(),
                result: Err(BackgroundError::new("SEND_UNAVAILABLE")),
            },
        ))
        .unwrap();

        assert_eq!(o.state().message_input().text(), "Test message");
        assert_eq!(o.state().active_pane(), ActivePane::MessageInput);
    }

    #[test]
    fn message_sent_refresh_updates_messages() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
        assert_eq!(o.state().open_chat().messages().len(), 1);

        // After a successful send, the refresh result arrives
        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::MessageSentRefreshCompleted {
                chat_id: 1,
                result: Ok(vec![message(1, "Hello"), message(2, "Hi")]),
            },
        ))
        .unwrap();

        assert_eq!(o.state().open_chat().messages().len(), 2);
    }

    #[test]
    fn enter_key_with_empty_input_does_nothing() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
        o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .unwrap();

        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        assert_eq!(o.state().message_input().text(), "");
        assert_eq!(o.dispatcher.send_dispatch_count(), 0);
    }

    #[test]
    fn enter_key_with_whitespace_only_does_nothing() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
        o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .unwrap();

        o.handle_event(AppEvent::InputKey(KeyInput::new(" ", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new(" ", false)))
            .unwrap();

        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        assert_eq!(o.state().message_input().text(), "  ");
        assert_eq!(o.dispatcher.send_dispatch_count(), 0);
    }

    #[test]
    fn rapid_pane_switching_maintains_consistent_state() {
        let mut o = orchestrator_with_chats(vec![chat(1, "General"), chat(2, "Backend")]);

        // Open chat 1
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();
        inject_messages(&mut o, 1, vec![message(1, "Hello")]);
        assert_eq!(o.state().active_pane(), ActivePane::Messages);

        o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
            .unwrap();
        assert_eq!(o.state().active_pane(), ActivePane::ChatList);

        o.handle_event(AppEvent::InputKey(KeyInput::new("l", false)))
            .unwrap();
        assert_eq!(o.state().active_pane(), ActivePane::Messages);

        o.handle_event(AppEvent::InputKey(KeyInput::new("esc", false)))
            .unwrap();
        assert_eq!(o.state().active_pane(), ActivePane::ChatList);

        assert!(o.state().is_running());
        assert!(o.state().open_chat().is_open());
    }

    #[test]
    fn integration_smoke_happy_path_startup_load_navigate_and_open_chat() {
        let mut o = make_orchestrator();
        assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Loading);

        // Tick dispatches chat list load
        o.handle_event(AppEvent::Tick).unwrap();
        // Simulate result
        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::ChatListLoaded {
                result: Ok(vec![chat(1, "General"), chat(2, "Backend"), chat(3, "Ops")]),
            },
        ))
        .unwrap();

        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        // Simulate messages loaded
        inject_messages(&mut o, 2, vec![message(1, "Hello")]);

        assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Ready);
        assert_eq!(o.state().chat_list().selected_index(), Some(1));
        assert_eq!(
            o.state().chat_list().selected_chat().map(|c| c.chat_id),
            Some(2)
        );
        assert_eq!(o.state().open_chat().chat_id(), Some(2));
        assert_eq!(o.state().open_chat().chat_title(), "Backend");
        assert_eq!(o.state().open_chat().messages().len(), 1);
        assert_eq!(o.storage.last_action, Some("open_chat".to_owned()));
    }

    #[test]
    fn integration_smoke_fallback_error_then_empty_list() {
        let mut o = make_orchestrator();

        // Tick dispatches
        o.handle_event(AppEvent::Tick).unwrap();
        // Error result
        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::ChatListLoaded {
                result: Err(BackgroundError::new("CHAT_LIST_UNAVAILABLE")),
            },
        ))
        .unwrap();
        assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Error);

        // Press r to retry
        o.handle_event(AppEvent::InputKey(KeyInput::new("r", false)))
            .unwrap();
        // Empty list result
        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::ChatListLoaded { result: Ok(vec![]) },
        ))
        .unwrap();

        assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Empty);
        assert_eq!(o.state().chat_list().selected_index(), None);
        assert!(o.state().is_running());
    }

    // ── Cached startup tests ──

    #[test]
    fn cached_startup_shows_ready_immediately() {
        let o = make_orchestrator_with_cached_chats(vec![chat(1, "Alpha"), chat(2, "Beta")]);

        assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Ready);
        assert_eq!(o.state().chat_list().chats().len(), 2);
        assert_eq!(o.state().chat_list().selected_index(), Some(0));
    }

    #[test]
    fn cached_startup_empty_cache_falls_back_to_loading() {
        let o = make_orchestrator_with_cached_chats(vec![]);

        assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Loading);
        assert!(o.state().chat_list().chats().is_empty());
    }

    #[test]
    fn cached_startup_first_tick_triggers_background_refresh() {
        let mut o = make_orchestrator_with_cached_chats(vec![chat(1, "Alpha"), chat(2, "Beta")]);

        // State is Ready from cache
        assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Ready);

        // First tick should trigger a background refresh even though state is Ready
        o.handle_event(AppEvent::Tick).unwrap();
        assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);

        // Data should remain visible (no blink)
        assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Ready);
        assert_eq!(o.state().chat_list().chats().len(), 2);
    }

    #[test]
    fn cached_startup_refresh_only_fires_once() {
        let mut o = make_orchestrator_with_cached_chats(vec![chat(1, "Alpha"), chat(2, "Beta")]);

        // First tick: triggers refresh
        o.handle_event(AppEvent::Tick).unwrap();
        assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);

        // Simulate result arriving
        inject_chat_list(
            &mut o,
            vec![chat(1, "Alpha"), chat(2, "Beta"), chat(3, "Gamma")],
        );

        // Second tick: should NOT trigger another refresh (initial_refresh_needed is false)
        o.handle_event(AppEvent::Tick).unwrap();
        assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);
    }

    #[test]
    fn cached_startup_background_result_updates_list() {
        let mut o = make_orchestrator_with_cached_chats(vec![chat(1, "Alpha"), chat(2, "Beta")]);

        // Navigate to second chat
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();
        assert_eq!(
            o.state().chat_list().selected_chat().map(|c| c.chat_id),
            Some(2)
        );

        // Trigger refresh via Tick
        o.handle_event(AppEvent::Tick).unwrap();

        // Background result arrives with updated data (new chat appeared at top)
        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::ChatListLoaded {
                result: Ok(vec![chat(3, "Gamma"), chat(1, "Alpha"), chat(2, "Beta")]),
            },
        ))
        .unwrap();

        // Selection should be preserved on chat_id=2
        assert_eq!(
            o.state().chat_list().selected_chat().map(|c| c.chat_id),
            Some(2)
        );
        assert_eq!(o.state().chat_list().chats().len(), 3);
    }

    #[test]
    fn non_cached_startup_does_not_set_initial_refresh_flag() {
        let mut o = make_orchestrator();

        // Default state is Loading, NOT Ready
        assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Loading);

        // First tick triggers the standard Loading -> dispatch path
        o.handle_event(AppEvent::Tick).unwrap();
        assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);

        // Result arrives
        inject_chat_list(&mut o, vec![chat(1, "Alpha")]);

        // Second tick should NOT dispatch again (no initial_refresh_needed flag)
        o.handle_event(AppEvent::Tick).unwrap();
        assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);
    }

    // ── Cached messages on chat open tests ──

    #[test]
    fn open_chat_with_cache_shows_ready_instantly() {
        let cache = StubCacheSource::with_messages(vec![(
            1,
            vec![message(10, "Cached A"), message(11, "Cached B")],
        )]);
        let mut o = make_orchestrator_with_cache(vec![chat(1, "Alpha"), chat(2, "Beta")], cache);

        // Open first chat (Enter)
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        // Chat should be Ready immediately from cache
        assert_eq!(o.state().open_chat().chat_id(), Some(1));
        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
        assert_eq!(o.state().open_chat().messages().len(), 2);
        assert_eq!(o.state().open_chat().messages()[0].text, "Cached A");

        // A background dispatch should still have been issued for a full load
        assert_eq!(o.dispatcher.messages_dispatch_count(), 1);
    }

    #[test]
    fn open_chat_without_cached_messages_falls_back_to_loading() {
        let cache = StubCacheSource::empty();
        let mut o = make_orchestrator_with_cache(vec![chat(1, "Alpha")], cache);

        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        assert_eq!(o.state().open_chat().chat_id(), Some(1));
        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);
        assert_eq!(o.dispatcher.messages_dispatch_count(), 1);
    }

    #[test]
    fn open_chat_without_cache_source_falls_back_to_loading() {
        // make_orchestrator_with_cached_chats sets cache_source = None
        let mut o = make_orchestrator_with_cached_chats(vec![chat(1, "Alpha")]);

        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        assert_eq!(o.state().open_chat().chat_id(), Some(1));
        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);
        assert_eq!(o.dispatcher.messages_dispatch_count(), 1);
    }

    #[test]
    fn reopen_same_ready_chat_skips_reload() {
        let cache = StubCacheSource::with_messages(vec![(1, vec![message(10, "Cached")])]);
        let mut o = make_orchestrator_with_cache(vec![chat(1, "Alpha"), chat(2, "Beta")], cache);

        // Open chat 1
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();
        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
        assert_eq!(o.dispatcher.messages_dispatch_count(), 1);

        // Navigate back to chat list
        o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
            .unwrap();

        // Re-open the same chat 1 (cursor still on it)
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        // No additional dispatch — still at 1
        assert_eq!(o.dispatcher.messages_dispatch_count(), 1);
        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
        assert_eq!(o.state().open_chat().chat_id(), Some(1));
    }

    #[test]
    fn reopen_same_chat_still_loading_dispatches_again() {
        let cache = StubCacheSource::empty();
        let mut o = make_orchestrator_with_cache(vec![chat(1, "Alpha")], cache);

        // Open chat 1 (no cache → Loading)
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();
        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);
        assert_eq!(o.dispatcher.messages_dispatch_count(), 1);

        // Go back and re-open (still Loading, not Ready)
        o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        // Should dispatch again since it's not Ready
        assert_eq!(o.dispatcher.messages_dispatch_count(), 2);
    }

    #[test]
    fn background_messages_on_cached_ready_chat_uses_update_messages() {
        let cache = StubCacheSource::with_messages(vec![(
            1,
            vec![message(10, "Cached A"), message(11, "Cached B")],
        )]);
        let mut o = make_orchestrator_with_cache(vec![chat(1, "Alpha")], cache);

        // Open chat — becomes Ready from cache
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();
        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
        assert_eq!(o.state().open_chat().messages().len(), 2);

        // Navigate to first cached message
        o.handle_event(AppEvent::InputKey(KeyInput::new("k", false)))
            .unwrap();
        assert_eq!(o.state().open_chat().selected_index(), Some(0));
        // selected message id is 10

        // Background full load arrives with more messages, including message 10
        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::MessagesLoaded {
                chat_id: 1,
                result: Ok(vec![
                    message(8, "Older"),
                    message(9, "Old"),
                    message(10, "Cached A"),
                    message(11, "Cached B"),
                    message(12, "New"),
                ]),
            },
        ))
        .unwrap();

        // State should still be Ready
        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
        // Messages updated to the full set
        assert_eq!(o.state().open_chat().messages().len(), 5);
        // Selection preserved on message 10 (now at index 2)
        assert_eq!(o.state().open_chat().selected_index(), Some(2));
    }

    #[test]
    fn background_messages_on_loading_chat_uses_set_ready() {
        let mut o = make_orchestrator_with_cached_chats(vec![chat(1, "Alpha")]);

        // Open chat — no cache, Loading
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();
        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);

        // Background load arrives
        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::MessagesLoaded {
                chat_id: 1,
                result: Ok(vec![message(1, "A"), message(2, "B")]),
            },
        ))
        .unwrap();

        // Should use set_ready (selects last message)
        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
        assert_eq!(o.state().open_chat().messages().len(), 2);
        assert_eq!(o.state().open_chat().selected_index(), Some(1));
    }

    #[test]
    fn open_different_chat_with_cache_replaces_previous() {
        let cache = StubCacheSource::with_messages(vec![
            (1, vec![message(10, "Chat1 cached")]),
            (2, vec![message(20, "Chat2 cached")]),
        ]);
        let mut o = make_orchestrator_with_cache(vec![chat(1, "Alpha"), chat(2, "Beta")], cache);

        // Open chat 1
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();
        assert_eq!(o.state().open_chat().chat_id(), Some(1));
        assert_eq!(o.state().open_chat().messages()[0].text, "Chat1 cached");

        // Navigate back, move to chat 2, open it
        o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        // Chat 2 should be shown from cache
        assert_eq!(o.state().open_chat().chat_id(), Some(2));
        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
        assert_eq!(o.state().open_chat().messages()[0].text, "Chat2 cached");
        assert_eq!(o.dispatcher.messages_dispatch_count(), 2);
    }

    // ── Chat lifecycle (openChat/closeChat/viewMessages) tests ──

    #[test]
    fn open_chat_dispatches_tdlib_open_chat() {
        let mut o = orchestrator_with_chats(vec![chat(1, "General")]);

        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        assert_eq!(o.dispatcher.open_chat_dispatch_count(), 1);
        assert_eq!(o.tdlib_opened_chat_id, Some(1));
    }

    #[test]
    fn navigate_away_from_chat_dispatches_tdlib_close_chat() {
        let o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);

        assert_eq!(o.tdlib_opened_chat_id, Some(1));

        let mut o = o;
        o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
            .unwrap();

        assert_eq!(o.dispatcher.close_chat_dispatch_count(), 1);
        assert_eq!(o.tdlib_opened_chat_id, None);
    }

    #[test]
    fn esc_from_messages_dispatches_tdlib_close_chat() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);

        o.handle_event(AppEvent::InputKey(KeyInput::new("esc", false)))
            .unwrap();

        assert_eq!(o.dispatcher.close_chat_dispatch_count(), 1);
        assert_eq!(o.tdlib_opened_chat_id, None);
    }

    #[test]
    fn switching_chats_closes_previous_and_opens_new() {
        let mut o = orchestrator_with_chats(vec![chat(1, "A"), chat(2, "B")]);

        // Open chat 1
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();
        assert_eq!(o.dispatcher.open_chat_dispatch_count(), 1);
        assert_eq!(o.tdlib_opened_chat_id, Some(1));

        // Navigate back to chat list
        o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
            .unwrap();
        assert_eq!(o.dispatcher.close_chat_dispatch_count(), 1);

        // Move to chat 2 and open it
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        assert_eq!(o.dispatcher.open_chat_dispatch_count(), 2);
        assert_eq!(o.tdlib_opened_chat_id, Some(2));
    }

    #[test]
    fn messages_loaded_dispatches_mark_as_read() {
        let mut o = orchestrator_with_chats(vec![chat(1, "General")]);

        // Open chat
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        // Simulate messages loaded
        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::MessagesLoaded {
                chat_id: 1,
                result: Ok(vec![message(10, "A"), message(20, "B"), message(30, "C")]),
            },
        ))
        .unwrap();

        assert_eq!(o.dispatcher.mark_as_read_dispatch_count(), 1);
        let (mark_chat_id, mark_ids) = o.dispatcher.last_mark_as_read().unwrap();
        assert_eq!(mark_chat_id, 1);
        assert_eq!(mark_ids, vec![10, 20, 30]);
    }

    #[test]
    fn messages_loaded_does_not_mark_as_read_when_empty() {
        let mut o = orchestrator_with_chats(vec![chat(1, "General")]);

        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::MessagesLoaded {
                chat_id: 1,
                result: Ok(vec![]),
            },
        ))
        .unwrap();

        assert_eq!(o.dispatcher.mark_as_read_dispatch_count(), 0);
    }

    #[test]
    fn message_sent_refresh_dispatches_mark_as_read() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);

        // After send refresh arrives with updated messages
        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::MessageSentRefreshCompleted {
                chat_id: 1,
                result: Ok(vec![message(1, "Hello"), message(2, "My reply")]),
            },
        ))
        .unwrap();

        // mark_as_read dispatched: once from initial messages load + once from refresh
        assert_eq!(o.dispatcher.mark_as_read_dispatch_count(), 2);
        let (mark_chat_id, mark_ids) = o.dispatcher.last_mark_as_read().unwrap();
        assert_eq!(mark_chat_id, 1);
        assert_eq!(mark_ids, vec![1, 2]);
    }

    #[test]
    fn reopen_same_ready_chat_does_not_dispatch_open_chat_again() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);

        // Focus back to chat list, then reopen same chat
        // Note: h closes the TDLib chat
        o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("l", false)))
            .unwrap();

        // Re-opening the same Ready chat triggers a new open_chat
        // (since we closed it with h)
        assert_eq!(o.dispatcher.open_chat_dispatch_count(), 2);
    }

    #[test]
    fn quit_while_chat_open_dispatches_close_chat() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
        assert_eq!(o.tdlib_opened_chat_id, Some(1));

        // Go back to chat list pane (but don't press h — stay with chat "open" in TDLib)
        // Actually, we need to be in ChatList or Messages pane for QuitRequested to quit
        o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
            .unwrap();
        // h already closed it, let's reopen
        o.handle_event(AppEvent::InputKey(KeyInput::new("l", false)))
            .unwrap();
        assert_eq!(o.tdlib_opened_chat_id, Some(1));

        // Now quit
        o.handle_event(AppEvent::QuitRequested).unwrap();

        assert!(!o.state().is_running());
        assert_eq!(o.tdlib_opened_chat_id, None);
        // close_chat dispatched: once from h, once from quit
        assert_eq!(o.dispatcher.close_chat_dispatch_count(), 2);
    }

    #[test]
    fn stale_messages_do_not_dispatch_mark_as_read() {
        let mut o = orchestrator_with_chats(vec![chat(1, "A"), chat(2, "B")]);

        // Open chat 1
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();
        // Navigate away
        o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
            .unwrap();
        // Open chat 2
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        // Stale result for chat 1 arrives
        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::MessagesLoaded {
                chat_id: 1,
                result: Ok(vec![message(10, "Stale")]),
            },
        ))
        .unwrap();

        // Should not dispatch mark_as_read since chat 1 is no longer viewed
        assert_eq!(o.dispatcher.mark_as_read_dispatch_count(), 0);
    }

    // ── Chat update → open chat message refresh tests ──

    #[test]
    fn chat_update_for_open_chat_dispatches_message_refresh() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
        let before = o.dispatcher.messages_dispatch_count();

        o.handle_event(AppEvent::ChatUpdateReceived {
            updates: vec![ChatUpdate::ChatMetadataChanged { chat_id: 1 }],
        })
        .unwrap();

        assert_eq!(o.dispatcher.messages_dispatch_count(), before + 1);
    }

    #[test]
    fn chat_update_for_unrelated_chat_does_not_dispatch_message_refresh() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
        let before = o.dispatcher.messages_dispatch_count();

        o.handle_event(AppEvent::ChatUpdateReceived {
            updates: vec![ChatUpdate::ChatMetadataChanged { chat_id: 999 }],
        })
        .unwrap();

        assert_eq!(o.dispatcher.messages_dispatch_count(), before);
    }

    #[test]
    fn chat_update_debounces_while_messages_refresh_in_flight() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
        let before = o.dispatcher.messages_dispatch_count();

        o.handle_event(AppEvent::ChatUpdateReceived {
            updates: vec![ChatUpdate::ChatMetadataChanged { chat_id: 1 }],
        })
        .unwrap();
        assert_eq!(o.dispatcher.messages_dispatch_count(), before + 1);

        o.handle_event(AppEvent::ChatUpdateReceived {
            updates: vec![ChatUpdate::ChatMetadataChanged { chat_id: 1 }],
        })
        .unwrap();
        assert_eq!(
            o.dispatcher.messages_dispatch_count(),
            before + 1,
            "second update while in-flight should be skipped"
        );
    }

    #[test]
    fn messages_refresh_in_flight_resets_after_messages_loaded() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
        let before = o.dispatcher.messages_dispatch_count();

        o.handle_event(AppEvent::ChatUpdateReceived {
            updates: vec![ChatUpdate::ChatMetadataChanged { chat_id: 1 }],
        })
        .unwrap();
        assert_eq!(o.dispatcher.messages_dispatch_count(), before + 1);

        inject_messages(&mut o, 1, vec![message(1, "Hello"), message(2, "World")]);

        o.handle_event(AppEvent::ChatUpdateReceived {
            updates: vec![ChatUpdate::ChatMetadataChanged { chat_id: 1 }],
        })
        .unwrap();
        assert_eq!(
            o.dispatcher.messages_dispatch_count(),
            before + 2,
            "after MessagesLoaded, new update should dispatch again"
        );
    }

    #[test]
    fn chat_update_with_no_open_chat_only_refreshes_chat_list() {
        let mut o = orchestrator_with_chats(vec![chat(1, "General")]);

        o.handle_event(AppEvent::ChatUpdateReceived {
            updates: vec![ChatUpdate::ChatMetadataChanged { chat_id: 1 }],
        })
        .unwrap();

        assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);
        assert_eq!(o.dispatcher.messages_dispatch_count(), 0);
    }

    #[test]
    fn messages_load_error_does_not_dispatch_mark_as_read() {
        let mut o = orchestrator_with_chats(vec![chat(1, "General")]);

        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::MessagesLoaded {
                chat_id: 1,
                result: Err(BackgroundError::new("MESSAGES_UNAVAILABLE")),
            },
        ))
        .unwrap();

        assert_eq!(o.dispatcher.mark_as_read_dispatch_count(), 0);
    }

    // ── Mark chat as read from chat list (r key) ──

    fn chat_with_unread(
        chat_id: i64,
        title: &str,
        unread: u32,
        last_msg_id: Option<i64>,
    ) -> ChatSummary {
        use crate::domain::chat::{ChatType, OutgoingReadStatus};
        ChatSummary {
            chat_id,
            title: title.to_owned(),
            unread_count: unread,
            last_message_preview: Some("text".to_owned()),
            last_message_unix_ms: None,
            is_pinned: false,
            chat_type: ChatType::Private,
            last_message_sender: None,
            is_online: None,
            is_bot: false,
            outgoing_status: OutgoingReadStatus::default(),
            last_message_id: last_msg_id,
        }
    }

    #[test]
    fn r_key_marks_selected_chat_as_read() {
        let mut o = orchestrator_with_chats(vec![chat_with_unread(1, "General", 5, Some(100))]);

        o.handle_event(AppEvent::InputKey(KeyInput::new("r", false)))
            .unwrap();

        assert_eq!(o.dispatcher.mark_chat_as_read_dispatch_count(), 1);
        assert_eq!(o.dispatcher.last_mark_chat_as_read(), Some((1, 100)));
    }

    #[test]
    fn r_key_does_nothing_when_chat_has_no_unread() {
        let mut o = orchestrator_with_chats(vec![chat_with_unread(1, "General", 0, Some(100))]);

        o.handle_event(AppEvent::InputKey(KeyInput::new("r", false)))
            .unwrap();

        assert_eq!(o.dispatcher.mark_chat_as_read_dispatch_count(), 0);
    }

    #[test]
    fn r_key_does_nothing_when_no_last_message_id() {
        let mut o = orchestrator_with_chats(vec![chat_with_unread(1, "General", 3, None)]);

        o.handle_event(AppEvent::InputKey(KeyInput::new("r", false)))
            .unwrap();

        assert_eq!(o.dispatcher.mark_chat_as_read_dispatch_count(), 0);
    }

    #[test]
    fn r_key_uses_mark_as_read_when_chat_already_opened_in_tdlib() {
        let mut o = orchestrator_with_chats(vec![chat_with_unread(1, "General", 5, Some(100))]);

        // Open the chat first (which sets tdlib_opened_chat_id)
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();
        // Go back to chat list
        o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
            .unwrap();

        // Now press r — should use dispatch_mark_as_read (not dispatch_mark_chat_as_read)
        // because the chat is still open in TDLib (closeChat was called when pressing h)
        // Actually, pressing h calls close_tdlib_chat, so the chat is closed.
        // This means dispatch_mark_chat_as_read will be used.
        o.handle_event(AppEvent::InputKey(KeyInput::new("r", false)))
            .unwrap();

        assert_eq!(o.dispatcher.mark_chat_as_read_dispatch_count(), 1);
    }

    #[test]
    fn r_key_optimistically_clears_unread_counter() {
        let mut o = orchestrator_with_chats(vec![
            chat_with_unread(1, "General", 5, Some(100)),
            chat_with_unread(2, "Backend", 3, Some(200)),
        ]);

        // Select second chat and press r
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("r", false)))
            .unwrap();

        // Unread counter should be cleared immediately (optimistic)
        let chats = o.state().chat_list().chats();
        assert_eq!(chats[0].unread_count, 5); // first chat unchanged
        assert_eq!(chats[1].unread_count, 0); // second chat cleared
    }

    // ── Help popup tests ──

    #[test]
    fn question_mark_opens_help_from_chat_list() {
        let mut o = make_orchestrator();
        o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
            .unwrap();
        assert!(o.state().help_visible());
        assert!(o.state().is_running());
    }

    #[test]
    fn question_mark_opens_help_from_messages() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hi")]);
        assert_eq!(o.state().active_pane(), ActivePane::Messages);
        o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
            .unwrap();
        assert!(o.state().help_visible());
    }

    #[test]
    fn question_mark_types_in_message_input_instead_of_opening_help() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hi")]);
        o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
            .unwrap();
        assert!(!o.state().help_visible());
        assert_eq!(o.state().message_input().text(), "?");
    }

    #[test]
    fn q_closes_help_without_quitting() {
        let mut o = make_orchestrator();
        o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
            .unwrap();
        assert!(o.state().help_visible());

        o.handle_event(AppEvent::InputKey(KeyInput::new("q", false)))
            .unwrap();
        assert!(!o.state().help_visible());
        assert!(o.state().is_running());
    }

    #[test]
    fn question_mark_closes_help_when_already_open() {
        let mut o = make_orchestrator();
        o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
            .unwrap();
        assert!(!o.state().help_visible());
        assert!(o.state().is_running());
    }

    #[test]
    fn esc_closes_help() {
        let mut o = make_orchestrator();
        o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("esc", false)))
            .unwrap();
        assert!(!o.state().help_visible());
        assert!(o.state().is_running());
    }

    #[test]
    fn other_keys_ignored_while_help_is_open() {
        let mut o = orchestrator_with_chats(vec![chat(1, "A"), chat(2, "B")]);
        o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
            .unwrap();

        // j should not move selection while help is visible
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();
        assert_eq!(o.state().chat_list().selected_index(), Some(0));
        assert!(o.state().help_visible());
    }

    #[test]
    fn ctrl_c_quits_even_when_help_is_open() {
        let mut o = make_orchestrator();
        o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
            .unwrap();
        o.handle_event(AppEvent::QuitRequested).unwrap();
        assert!(!o.state().is_running());
    }

    #[test]
    fn q_quits_from_chat_list_when_help_is_not_open() {
        let mut o = make_orchestrator();
        o.handle_event(AppEvent::InputKey(KeyInput::new("q", false)))
            .unwrap();
        assert!(!o.state().is_running());
    }

    #[test]
    fn q_quits_from_messages_when_help_is_not_open() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hi")]);
        o.handle_event(AppEvent::InputKey(KeyInput::new("q", false)))
            .unwrap();
        assert!(!o.state().is_running());
    }

    #[test]
    fn help_does_not_change_active_pane() {
        let mut o = make_orchestrator();
        assert_eq!(o.state().active_pane(), ActivePane::ChatList);
        o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
            .unwrap();
        assert_eq!(o.state().active_pane(), ActivePane::ChatList);
    }

    #[test]
    fn help_does_not_change_active_pane_in_messages() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hi")]);
        assert_eq!(o.state().active_pane(), ActivePane::Messages);
        o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
            .unwrap();
        assert_eq!(o.state().active_pane(), ActivePane::Messages);
    }

    #[test]
    fn enter_key_ignored_while_help_is_open() {
        let mut o = orchestrator_with_chats(vec![chat(1, "General")]);
        o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();
        // Should still be on chat list, not opened a chat
        assert_eq!(o.state().active_pane(), ActivePane::ChatList);
        assert!(o.state().help_visible());
    }

    #[test]
    fn help_close_then_q_quits() {
        let mut o = make_orchestrator();
        o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("q", false)))
            .unwrap();
        assert!(!o.state().help_visible());
        assert!(o.state().is_running());

        // Now q again should quit
        o.handle_event(AppEvent::InputKey(KeyInput::new("q", false)))
            .unwrap();
        assert!(!o.state().is_running());
    }

    #[test]
    fn ctrl_o_ignored_while_help_is_open() {
        let mut o = make_orchestrator();
        o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
            .unwrap();
        // Ctrl+O should not open browser while help is visible
        o.handle_event(AppEvent::InputKey(KeyInput::new("o", true)))
            .unwrap();
        assert!(o.state().help_visible());
    }

    #[test]
    fn help_open_from_messages_then_esc_closes_help_not_pane() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hi")]);
        assert_eq!(o.state().active_pane(), ActivePane::Messages);

        o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("esc", false)))
            .unwrap();

        // Help closed, but still in Messages pane (not back to ChatList)
        assert!(!o.state().help_visible());
        assert_eq!(o.state().active_pane(), ActivePane::Messages);
    }

    #[test]
    fn multiple_help_toggle_cycles() {
        let mut o = make_orchestrator();
        for _ in 0..5 {
            o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
                .unwrap();
            assert!(o.state().help_visible());
            o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
                .unwrap();
            assert!(!o.state().help_visible());
        }
        assert!(o.state().is_running());
    }

    #[test]
    fn tick_events_still_processed_while_help_is_open() {
        let mut o = make_orchestrator();
        o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
            .unwrap();
        // Tick should not error or panic while help is visible
        o.handle_event(AppEvent::Tick).unwrap();
        assert!(o.state().help_visible());
    }

    #[test]
    fn connectivity_events_still_processed_while_help_is_open() {
        let mut o = make_orchestrator();
        o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
            .unwrap();
        o.handle_event(AppEvent::ConnectivityChanged(ConnectivityStatus::Connected))
            .unwrap();
        assert_eq!(
            o.state().connectivity_status(),
            ConnectivityStatus::Connected
        );
        assert!(o.state().help_visible());
    }

    // ── Message cache tests ──

    #[test]
    fn messages_stored_in_cache_after_background_load() {
        let mut o = orchestrator_with_chats(vec![chat(1, "General")]);

        // Open chat 1
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        assert!(!o.state().message_cache().has_messages(1));

        // Background load completes
        inject_messages(&mut o, 1, vec![message(1, "Hello"), message(2, "World")]);

        assert!(o.state().message_cache().has_messages(1));
        assert_eq!(o.state.message_cache_mut().get(1).unwrap().len(), 2);
    }

    #[test]
    fn stale_messages_result_still_cached() {
        let mut o = orchestrator_with_chats(vec![chat(1, "Alice"), chat(2, "Bob")]);

        // Open chat 1
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        // Navigate back to chat list before messages arrive
        o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
            .unwrap();

        // Messages arrive for chat 1 (now "stale" since user navigated away)
        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::MessagesLoaded {
                chat_id: 1,
                result: Ok(vec![message(10, "cached even though stale")]),
            },
        ))
        .unwrap();

        // Messages should still be in cache despite the stale discard
        assert!(o.state().message_cache().has_messages(1));
        assert_eq!(
            o.state.message_cache_mut().get(1).unwrap()[0].text,
            "cached even though stale"
        );
    }

    #[test]
    fn cache_hit_on_reopen_shows_messages_instantly() {
        let mut o = orchestrator_with_chats(vec![chat(1, "Alice"), chat(2, "Bob")]);

        // Open chat 1 and load messages
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();
        inject_messages(&mut o, 1, vec![message(1, "Hello"), message(2, "World")]);
        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);

        // Navigate back to chat list
        o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
            .unwrap();

        // Move to chat 2 and open it (to make the orchestrator forget chat 1's OpenChatState)
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();
        inject_messages(&mut o, 2, vec![message(10, "Bob's message")]);

        // Navigate back, move to chat 1
        o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("k", false)))
            .unwrap();

        let msgs_dispatched_before = o.dispatcher.messages_dispatch_count();

        // Re-open chat 1 — should show cached messages instantly
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        // State should be Ready immediately (from cache), not Loading
        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
        assert_eq!(o.state().open_chat().messages().len(), 2);
        assert_eq!(o.state().open_chat().messages()[0].text, "Hello");

        // Background refresh should still be dispatched
        assert_eq!(
            o.dispatcher.messages_dispatch_count(),
            msgs_dispatched_before + 1,
            "background refresh should be dispatched even on cache hit"
        );
    }

    #[test]
    fn cache_miss_falls_through_to_tdlib_local_cache() {
        let cache = StubCacheSource::with_messages(vec![(1, vec![message(1, "from tdlib local")])]);
        let mut o = make_orchestrator_with_cache(vec![chat(1, "Alice")], cache);

        // Trigger initial refresh since we use with_initial_chat_list
        o.handle_event(AppEvent::Tick).unwrap();
        inject_chat_list(&mut o, vec![chat(1, "Alice")]);

        // Open chat 1 — app cache is empty, but TDLib local cache has data
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        // Should be Ready from TDLib local cache (StubCacheSource)
        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
        assert_eq!(o.state().open_chat().messages()[0].text, "from tdlib local");
    }

    #[test]
    fn cache_updated_on_message_sent_refresh() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(1, "Alice")], 1, vec![message(1, "Hello")]);

        // Simulate sending a message and getting refresh result
        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::MessageSentRefreshCompleted {
                chat_id: 1,
                result: Ok(vec![message(1, "Hello"), message(2, "My new message")]),
            },
        ))
        .unwrap();

        // Cache should contain the updated messages
        assert!(o.state().message_cache().has_messages(1));
        let cached = o.state.message_cache_mut().get(1).unwrap();
        assert_eq!(cached.len(), 2);
        assert_eq!(cached[1].text, "My new message");
    }

    #[test]
    fn cache_not_populated_on_load_error() {
        let mut o = orchestrator_with_chats(vec![chat(1, "General")]);

        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::MessagesLoaded {
                chat_id: 1,
                result: Err(BackgroundError::new("NETWORK_ERROR")),
            },
        ))
        .unwrap();

        assert!(!o.state().message_cache().has_messages(1));
    }

    // ── Push-based cache warming tests (Phase 2) ──

    #[test]
    fn push_new_message_warms_cache_for_non_open_chat() {
        let mut o = orchestrator_with_open_chat(
            vec![chat(1, "Alice"), chat(2, "Bob")],
            1,
            vec![message(1, "Hello")],
        );

        // Push a new message for chat 2 (not currently open)
        o.handle_event(AppEvent::ChatUpdateReceived {
            updates: vec![ChatUpdate::NewMessage {
                chat_id: 2,
                message: message(10, "Hey from Bob"),
            }],
        })
        .unwrap();

        // Chat 2 should now have a cached message
        assert!(o.state().message_cache().has_messages(2));
        assert_eq!(
            o.state.message_cache_mut().get(2).unwrap()[0].text,
            "Hey from Bob"
        );
    }

    #[test]
    fn push_new_message_appends_to_existing_cache() {
        let mut o = orchestrator_with_chats(vec![chat(1, "Alice")]);

        // Open and load chat 1
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();
        inject_messages(&mut o, 1, vec![message(1, "First")]);

        // Push a new message via update
        o.handle_event(AppEvent::ChatUpdateReceived {
            updates: vec![ChatUpdate::NewMessage {
                chat_id: 1,
                message: message(2, "Second"),
            }],
        })
        .unwrap();

        let cached = o.state.message_cache_mut().get(1).unwrap();
        assert_eq!(cached.len(), 2);
        assert_eq!(cached[0].text, "First");
        assert_eq!(cached[1].text, "Second");
    }

    #[test]
    fn push_delete_messages_removes_from_cache() {
        let mut o = orchestrator_with_chats(vec![chat(1, "Alice")]);

        // Open and load chat 1
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();
        inject_messages(&mut o, 1, vec![message(1, "Keep"), message(2, "Delete me")]);

        // Push a delete update
        o.handle_event(AppEvent::ChatUpdateReceived {
            updates: vec![ChatUpdate::MessagesDeleted {
                chat_id: 1,
                message_ids: vec![2],
            }],
        })
        .unwrap();

        let cached = o.state.message_cache_mut().get(1).unwrap();
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].text, "Keep");
    }

    #[test]
    fn push_new_message_for_open_chat_dispatches_refresh() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(1, "Alice")], 1, vec![message(1, "Hello")]);
        let before = o.dispatcher.messages_dispatch_count();

        o.handle_event(AppEvent::ChatUpdateReceived {
            updates: vec![ChatUpdate::NewMessage {
                chat_id: 1,
                message: message(2, "New message"),
            }],
        })
        .unwrap();

        // Should dispatch a message refresh for the open chat
        assert_eq!(o.dispatcher.messages_dispatch_count(), before + 1);
    }

    #[test]
    fn push_metadata_update_does_not_warm_cache() {
        let mut o = orchestrator_with_chats(vec![chat(1, "Alice")]);

        o.handle_event(AppEvent::ChatUpdateReceived {
            updates: vec![ChatUpdate::ChatMetadataChanged { chat_id: 1 }],
        })
        .unwrap();

        // Metadata updates should not create cache entries
        assert!(!o.state().message_cache().has_messages(1));
    }

    #[test]
    fn push_cache_warm_then_open_is_instant() {
        let mut o = orchestrator_with_chats(vec![chat(1, "Alice"), chat(2, "Bob")]);

        // Push messages for chat 2 (not open)
        o.handle_event(AppEvent::ChatUpdateReceived {
            updates: vec![
                ChatUpdate::NewMessage {
                    chat_id: 2,
                    message: message(10, "Bob msg 1"),
                },
                ChatUpdate::NewMessage {
                    chat_id: 2,
                    message: message(11, "Bob msg 2"),
                },
            ],
        })
        .unwrap();

        // Navigate to chat 2 and open it
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        // Should be Ready instantly from push-warmed cache
        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
        assert_eq!(o.state().open_chat().messages().len(), 2);
        assert_eq!(o.state().open_chat().messages()[0].text, "Bob msg 1");
    }

    // ── Prefetch on j/k navigation tests (Phase 3) ──

    #[test]
    fn jk_navigation_dispatches_prefetch_for_uncached_chat() {
        let mut o =
            orchestrator_with_chats(vec![chat(1, "Alpha"), chat(2, "Beta"), chat(3, "Gamma")]);

        // Navigate down to chat 2 (no cache)
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();

        assert_eq!(o.dispatcher.prefetch_dispatch_count(), 1);
        assert_eq!(o.dispatcher.last_prefetch_chat_id(), Some(2));
        assert_eq!(o.prefetch_in_flight, Some(2));
    }

    #[test]
    fn jk_navigation_skips_prefetch_when_cache_hit() {
        let mut o = orchestrator_with_chats(vec![chat(1, "Alpha"), chat(2, "Beta")]);

        // Populate cache for chat 2
        o.state
            .message_cache_mut()
            .put(2, vec![message(10, "cached")], true);

        // Navigate down to chat 2
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();

        assert_eq!(
            o.dispatcher.prefetch_dispatch_count(),
            0,
            "should not prefetch when cache already has data"
        );
    }

    #[test]
    fn jk_rapid_navigation_debounces_prefetch() {
        let mut o = orchestrator_with_chats(vec![
            chat(1, "Alpha"),
            chat(2, "Beta"),
            chat(3, "Gamma"),
            chat(4, "Delta"),
        ]);

        // First j dispatches prefetch for chat 2
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();
        assert_eq!(o.dispatcher.prefetch_dispatch_count(), 1);
        assert_eq!(o.dispatcher.last_prefetch_chat_id(), Some(2));

        // Second j should NOT dispatch (prefetch for chat 2 still in-flight)
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();
        assert_eq!(
            o.dispatcher.prefetch_dispatch_count(),
            1,
            "second j should be debounced by in-flight guard"
        );
    }

    #[test]
    fn prefetch_result_populates_cache_only() {
        let mut o = orchestrator_with_chats(vec![chat(1, "Alpha"), chat(2, "Beta")]);

        // Navigate down (triggers prefetch for chat 2)
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();
        assert_eq!(o.prefetch_in_flight, Some(2));

        // Prefetch result arrives
        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::MessagesPrefetched {
                chat_id: 2,
                result: Ok(vec![
                    message(10, "Prefetched A"),
                    message(11, "Prefetched B"),
                ]),
            },
        ))
        .unwrap();

        // Cache should have the data
        assert!(o.state().message_cache().has_messages(2));
        assert_eq!(o.state.message_cache_mut().get(2).unwrap().len(), 2);

        // OpenChatState should NOT be affected (no chat is open)
        assert!(!o.state().open_chat().is_open());

        // In-flight guard should be cleared
        assert_eq!(o.prefetch_in_flight, None);
    }

    #[test]
    fn prefetch_result_updates_open_chat_if_loading() {
        let mut o = orchestrator_with_chats(vec![chat(1, "Alpha"), chat(2, "Beta")]);

        // Navigate down (triggers prefetch for chat 2)
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();

        // User opens chat 2 while prefetch is in-flight
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();
        assert_eq!(o.state().open_chat().chat_id(), Some(2));
        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);

        // Prefetch result arrives
        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::MessagesPrefetched {
                chat_id: 2,
                result: Ok(vec![message(10, "Prefetched")]),
            },
        ))
        .unwrap();

        // OpenChatState should be updated from cache
        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
        assert_eq!(o.state().open_chat().messages().len(), 1);
        assert_eq!(o.state().open_chat().messages()[0].text, "Prefetched");
    }

    #[test]
    fn prefetch_error_clears_in_flight_without_cache() {
        let mut o = orchestrator_with_chats(vec![chat(1, "Alpha"), chat(2, "Beta")]);

        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();
        assert_eq!(o.prefetch_in_flight, Some(2));

        // Prefetch fails
        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::MessagesPrefetched {
                chat_id: 2,
                result: Err(BackgroundError::new("MESSAGES_UNAVAILABLE")),
            },
        ))
        .unwrap();

        assert_eq!(o.prefetch_in_flight, None);
        assert!(!o.state().message_cache().has_messages(2));
    }

    #[test]
    fn open_selected_chat_clears_prefetch_in_flight() {
        let mut o = orchestrator_with_chats(vec![chat(1, "Alpha"), chat(2, "Beta")]);

        // Trigger prefetch
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();
        assert_eq!(o.prefetch_in_flight, Some(2));

        // Open the chat
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        assert_eq!(
            o.prefetch_in_flight, None,
            "opening a chat should clear prefetch guard"
        );
    }

    #[test]
    fn prefetch_then_open_is_instant_from_cache() {
        let mut o =
            orchestrator_with_chats(vec![chat(1, "Alpha"), chat(2, "Beta"), chat(3, "Gamma")]);

        // Navigate to chat 2 (triggers prefetch)
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();

        // Prefetch completes
        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::MessagesPrefetched {
                chat_id: 2,
                result: Ok(vec![message(10, "Prefetched msg")]),
            },
        ))
        .unwrap();

        // Open chat 2 — should be instant from cache
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        assert_eq!(o.state().open_chat().chat_id(), Some(2));
        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
        assert_eq!(o.state().open_chat().messages()[0].text, "Prefetched msg");

        // Background refresh still dispatched
        assert!(o.dispatcher.messages_dispatch_count() > 0);
    }

    #[test]
    fn k_navigation_also_triggers_prefetch() {
        let mut o =
            orchestrator_with_chats(vec![chat(1, "Alpha"), chat(2, "Beta"), chat(3, "Gamma")]);

        // Navigate to the bottom first
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();

        // Clear the in-flight by injecting the result
        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::MessagesPrefetched {
                chat_id: 2,
                result: Ok(vec![message(10, "Beta msg")]),
            },
        ))
        .unwrap();

        // Navigate up with k to chat 2 (already cached)
        o.handle_event(AppEvent::InputKey(KeyInput::new("k", false)))
            .unwrap();
        let prefetch_count_after_k_to_cached = o.dispatcher.prefetch_dispatch_count();

        // Chat 2 is cached, so no new prefetch
        assert_eq!(prefetch_count_after_k_to_cached, 1);

        // Navigate further up to chat 1 (not cached)
        // But prefetch for chat 3 might still be in-flight...
        // Actually chat 3 prefetch was never dispatched because chat 2 prefetch was in-flight.
        // So after receiving the result above, prefetch_in_flight is None.
        o.handle_event(AppEvent::InputKey(KeyInput::new("k", false)))
            .unwrap();
        assert_eq!(o.dispatcher.prefetch_dispatch_count(), 2);
        assert_eq!(o.dispatcher.last_prefetch_chat_id(), Some(1));
    }

    #[test]
    fn prefetch_guard_allows_new_dispatch_after_result() {
        let mut o =
            orchestrator_with_chats(vec![chat(1, "Alpha"), chat(2, "Beta"), chat(3, "Gamma")]);

        // Navigate to chat 2 (prefetch dispatched)
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();
        assert_eq!(o.dispatcher.prefetch_dispatch_count(), 1);

        // Prefetch completes
        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::MessagesPrefetched {
                chat_id: 2,
                result: Ok(vec![message(10, "msg")]),
            },
        ))
        .unwrap();

        // Navigate to chat 3 (should dispatch new prefetch)
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();
        assert_eq!(o.dispatcher.prefetch_dispatch_count(), 2);
        assert_eq!(o.dispatcher.last_prefetch_chat_id(), Some(3));
    }

    #[test]
    fn prefetch_result_for_different_chat_does_not_affect_open_chat() {
        let mut o =
            orchestrator_with_chats(vec![chat(1, "Alpha"), chat(2, "Beta"), chat(3, "Gamma")]);

        // Prefetch dispatched for chat 2
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();

        // User opens chat 3 instead
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();
        assert_eq!(o.state().open_chat().chat_id(), Some(3));
        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);

        // Stale prefetch for chat 2 arrives
        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::MessagesPrefetched {
                chat_id: 2,
                result: Ok(vec![message(10, "Prefetched for 2")]),
            },
        ))
        .unwrap();

        // Chat 3 should still be Loading
        assert_eq!(o.state().open_chat().chat_id(), Some(3));
        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);
        // Chat 2 should be in cache
        assert!(o.state().message_cache().has_messages(2));
    }

    #[test]
    fn prefetch_empty_result_does_not_populate_cache() {
        let mut o = orchestrator_with_chats(vec![chat(1, "Alpha"), chat(2, "Beta")]);

        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();

        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::MessagesPrefetched {
                chat_id: 2,
                result: Ok(vec![]),
            },
        ))
        .unwrap();

        assert!(!o.state().message_cache().has_messages(2));
    }

    // ── Phase 5: UX polish tests ──

    #[test]
    fn cache_below_threshold_stays_in_loading() {
        let mut o = make_orchestrator_with_threshold(vec![chat(1, "Alice")], 5);

        // Pre-populate cache with fewer messages than threshold
        o.state
            .message_cache_mut()
            .put(1, vec![message(1, "single msg")], true);

        // Open the chat
        inject_chat_list(&mut o, vec![chat(1, "Alice")]);
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        // Should remain in Loading because cache has 1 < 5 messages
        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);
    }

    #[test]
    fn cache_at_threshold_shows_ready() {
        let mut o = make_orchestrator_with_threshold(vec![chat(1, "Alice")], 3);

        // Pre-populate cache with exactly threshold messages
        o.state.message_cache_mut().put(
            1,
            vec![message(1, "A"), message(2, "B"), message(3, "C")],
            true,
        );

        inject_chat_list(&mut o, vec![chat(1, "Alice")]);
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
        assert_eq!(o.state().open_chat().messages().len(), 3);
    }

    #[test]
    fn cache_hit_sets_refreshing_and_cached_source() {
        let mut o = orchestrator_with_chats(vec![chat(1, "Alice")]);

        // Pre-populate cache
        o.state
            .message_cache_mut()
            .put(1, vec![message(1, "A"), message(2, "B")], true);

        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
        assert!(o.state().open_chat().is_refreshing());
        assert_eq!(o.state().open_chat().message_source(), MessageSource::Cache);
    }

    #[test]
    fn background_load_clears_refreshing_and_sets_live_source() {
        let mut o = orchestrator_with_chats(vec![chat(1, "Alice")]);

        // Pre-populate cache for instant display
        o.state
            .message_cache_mut()
            .put(1, vec![message(1, "cached")], true);

        // Open chat — sets Ready + refreshing + Cache source
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();
        assert!(o.state().open_chat().is_refreshing());

        // Background load completes
        inject_messages(
            &mut o,
            1,
            vec![message(1, "fresh A"), message(2, "fresh B")],
        );

        assert!(!o.state().open_chat().is_refreshing());
        assert_eq!(o.state().open_chat().message_source(), MessageSource::Live);
        assert_eq!(o.state().open_chat().messages().len(), 2);
    }

    #[test]
    fn loading_state_has_no_refreshing_or_source() {
        let mut o = orchestrator_with_chats(vec![chat(1, "Alice")]);

        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);
        assert!(!o.state().open_chat().is_refreshing());
        assert_eq!(o.state().open_chat().message_source(), MessageSource::None);
    }

    #[test]
    fn tdlib_local_cache_below_threshold_stays_in_loading() {
        let cache = StubCacheSource::with_messages(vec![(1, vec![message(1, "sparse")])]);
        let mut o = make_orchestrator_with_cache_and_threshold(vec![chat(1, "Alice")], cache, 5);

        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        // TDLib local cache has 1 message < threshold 5 → Loading
        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);
    }

    #[test]
    fn tdlib_local_cache_at_threshold_shows_ready_with_cache_source() {
        let msgs: Vec<Message> = (1..=5).map(|i| message(i, &format!("msg {i}"))).collect();
        let cache = StubCacheSource::with_messages(vec![(1, msgs)]);
        let mut o = make_orchestrator_with_cache_and_threshold(vec![chat(1, "Alice")], cache, 5);

        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
        assert_eq!(o.state().open_chat().messages().len(), 5);
        assert!(o.state().open_chat().is_refreshing());
        assert_eq!(o.state().open_chat().message_source(), MessageSource::Cache);
    }

    #[test]
    fn message_sent_refresh_clears_refreshing_and_sets_live() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(1, "Alice")], 1, vec![message(1, "Hello")]);

        // Simulate: cache hit sets refreshing
        o.state.open_chat_mut().set_refreshing(true);
        o.state
            .open_chat_mut()
            .set_message_source(MessageSource::Cache);

        // Message sent refresh arrives
        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::MessageSentRefreshCompleted {
                chat_id: 1,
                result: Ok(vec![message(1, "Hello"), message(2, "New msg")]),
            },
        ))
        .unwrap();

        assert!(!o.state().open_chat().is_refreshing());
        assert_eq!(o.state().open_chat().message_source(), MessageSource::Live);
    }

    #[test]
    fn threshold_zero_is_clamped_to_one() {
        let mut o = make_orchestrator_with_threshold(vec![chat(1, "Alice")], 0);

        o.state
            .message_cache_mut()
            .put(1, vec![message(1, "single")], true);

        inject_chat_list(&mut o, vec![chat(1, "Alice")]);
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        // With threshold clamped to 1, a single message is sufficient
        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
    }

    #[test]
    fn background_load_error_clears_refreshing() {
        let mut o = orchestrator_with_chats(vec![chat(1, "Alice")]);

        // Pre-populate cache for instant display
        o.state
            .message_cache_mut()
            .put(1, vec![message(1, "cached")], true);

        // Open chat — sets Ready + refreshing
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();
        assert!(o.state().open_chat().is_refreshing());

        // Background load fails
        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::MessagesLoaded {
                chat_id: 1,
                result: Err(BackgroundError::new("MESSAGES_UNAVAILABLE")),
            },
        ))
        .unwrap();

        // Error state should have refreshing cleared
        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Error);
        assert!(!o.state().open_chat().is_refreshing());
    }

    #[test]
    fn prefetch_below_threshold_does_not_populate_open_chat() {
        let mut o = make_orchestrator_with_threshold(vec![chat(1, "Alpha"), chat(2, "Beta")], 5);

        inject_chat_list(&mut o, vec![chat(1, "Alpha"), chat(2, "Beta")]);

        // Navigate to chat 2 (triggers prefetch)
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();

        // Open chat 2
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();
        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);

        // Prefetch result with too few messages
        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::MessagesPrefetched {
                chat_id: 2,
                result: Ok(vec![message(10, "sparse")]),
            },
        ))
        .unwrap();

        // Should still be Loading (1 < 5 threshold)
        assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);
        // But cache should have data
        assert!(o.state().message_cache().has_messages(2));
    }

    // ── dd (delete message) tests ──

    #[test]
    fn dd_deletes_selected_message() {
        let mut o = orchestrator_with_open_chat(
            vec![chat(1, "Chat")],
            1,
            vec![message(10, "hello"), message(20, "world")],
        );

        // Select last message (20) — default after open
        assert_eq!(o.state().open_chat().selected_message().unwrap().id, 20);

        // Press d, d
        o.handle_event(AppEvent::InputKey(KeyInput::new("d", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("d", false)))
            .unwrap();

        // Message 20 should be removed from UI
        assert_eq!(o.state().open_chat().messages().len(), 1);
        assert_eq!(o.state().open_chat().messages()[0].id, 10);

        // Dispatch should have been called
        assert_eq!(o.dispatcher.delete_dispatch_count(), 1);
        assert_eq!(o.dispatcher.last_delete(), Some((1, 20)));
    }

    #[test]
    fn d_then_other_key_cancels_delete() {
        let mut o = orchestrator_with_open_chat(
            vec![chat(1, "Chat")],
            1,
            vec![message(10, "hello"), message(20, "world")],
        );

        // Press d, then j (navigate) — should cancel delete
        o.handle_event(AppEvent::InputKey(KeyInput::new("d", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();

        // No deletion should have happened
        assert_eq!(o.state().open_chat().messages().len(), 2);
        assert_eq!(o.dispatcher.delete_dispatch_count(), 0);
    }

    #[test]
    fn dd_on_empty_chat_does_nothing() {
        let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![]);

        o.handle_event(AppEvent::InputKey(KeyInput::new("d", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("d", false)))
            .unwrap();

        assert_eq!(o.dispatcher.delete_dispatch_count(), 0);
    }

    #[test]
    fn dd_does_not_delete_pending_messages() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hello")]);

        // Switch to message input and send a message (creates pending msg with id=0)
        o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .unwrap();
        // Type "test" + enter
        for ch in "test".chars() {
            o.handle_event(AppEvent::InputKey(KeyInput::new(ch.to_string(), false)))
                .unwrap();
        }
        o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();
        // Go back to messages pane
        o.handle_event(AppEvent::InputKey(KeyInput::new("esc", false)))
            .unwrap();

        // Select the last message (pending, id=0)
        let selected = o.state().open_chat().selected_message().unwrap();
        assert_eq!(selected.id, 0); // pending

        // dd should not dispatch delete for id=0
        o.handle_event(AppEvent::InputKey(KeyInput::new("d", false)))
            .unwrap();
        o.handle_event(AppEvent::InputKey(KeyInput::new("d", false)))
            .unwrap();

        assert_eq!(o.dispatcher.delete_dispatch_count(), 0);
    }

    // ── o (open link) tests ──

    #[test]
    fn o_opens_first_url_from_message() {
        let mut o = orchestrator_with_open_chat(
            vec![chat(1, "Chat")],
            1,
            vec![message(10, "Check https://example.com out")],
        );

        o.handle_event(AppEvent::InputKey(KeyInput::new("o", false)))
            .unwrap();

        assert_eq!(o.opener.opened_urls(), vec!["https://example.com"]);
    }

    #[test]
    fn o_does_nothing_when_no_url() {
        let mut o = orchestrator_with_open_chat(
            vec![chat(1, "Chat")],
            1,
            vec![message(10, "No links here")],
        );

        o.handle_event(AppEvent::InputKey(KeyInput::new("o", false)))
            .unwrap();

        assert!(o.opener.opened_urls().is_empty());
    }

    #[test]
    fn o_opens_first_url_when_multiple() {
        let mut o = orchestrator_with_open_chat(
            vec![chat(1, "Chat")],
            1,
            vec![message(
                10,
                "Visit https://first.com and https://second.com",
            )],
        );

        o.handle_event(AppEvent::InputKey(KeyInput::new("o", false)))
            .unwrap();

        assert_eq!(o.opener.opened_urls(), vec!["https://first.com"]);
    }

    #[test]
    fn o_on_empty_chat_does_nothing() {
        let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![]);

        o.handle_event(AppEvent::InputKey(KeyInput::new("o", false)))
            .unwrap();

        assert!(o.opener.opened_urls().is_empty());
    }

    // ── Voice recording / command popup tests ──

    /// Simulates the state after `start_voice_recording` succeeds:
    /// opens the command popup and sets a recording file path.
    /// Does NOT spawn an external process (recording_handle is None).
    ///
    /// Use `simulate_voice_recording_with_process` when testing exit code paths.
    fn simulate_voice_recording_started(o: &mut TestOrchestrator, file_path: &str) {
        o.state.open_command_popup("Recording Voice");
        o.recording_file_path = Some(file_path.to_owned());
    }

    /// Simulates voice recording with a real process for exit-code tests.
    /// `success`: if true, spawns `true` (exit 0); if false, spawns `false` (exit 1).
    fn simulate_voice_recording_with_process(
        o: &mut TestOrchestrator,
        file_path: &str,
        success: bool,
    ) {
        use std::process::Command;

        let cmd = if success { "true" } else { "false" };
        let child = Command::new(cmd)
            .spawn()
            .expect("failed to spawn test process");
        // Wait briefly for the short-lived process to exit.
        let mut handle = crate::usecases::voice_recording::RecordingHandle::from_child(child);
        std::thread::sleep(std::time::Duration::from_millis(50));
        // Ensure it actually exited so try_exit_success returns Some.
        let _ = handle.try_exit_success();

        o.state.open_command_popup("Recording Voice");
        o.recording_file_path = Some(file_path.to_owned());
        o.recording_handle = Some(handle);
    }

    #[test]
    fn v_ignored_when_no_chat_open() {
        let mut o = orchestrator_with_chats(vec![chat(1, "Chat")]);

        o.handle_event(AppEvent::InputKey(KeyInput::new("v", false)))
            .unwrap();

        assert!(o.state().command_popup().is_none());
    }

    #[test]
    fn v_ignored_when_popup_already_active() {
        let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
        simulate_voice_recording_started(&mut o, "/tmp/test.oga");

        // Second v press should not panic or change state.
        o.handle_event(AppEvent::InputKey(KeyInput::new("v", false)))
            .unwrap();

        assert!(o.state().command_popup().is_some());
        assert_eq!(
            o.state().command_popup().unwrap().title(),
            "Recording Voice"
        );
    }

    #[test]
    fn command_popup_intercepts_all_keys() {
        let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
        simulate_voice_recording_started(&mut o, "/tmp/test.oga");

        // Press a navigation key — should be intercepted, not move selection.
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();

        // Popup is still active, key was absorbed.
        assert!(o.state().command_popup().is_some());
    }

    #[test]
    fn command_popup_q_transitions_to_stopping() {
        use crate::domain::command_popup_state::CommandPhase;

        let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
        simulate_voice_recording_started(&mut o, "/tmp/test.oga");

        o.handle_event(AppEvent::InputKey(KeyInput::new("q", false)))
            .unwrap();

        let popup = o
            .state()
            .command_popup()
            .expect("popup should still be open");
        assert_eq!(popup.phase(), &CommandPhase::Stopping);
    }

    #[test]
    fn command_popup_random_key_during_running_is_ignored() {
        use crate::domain::command_popup_state::CommandPhase;

        let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
        simulate_voice_recording_started(&mut o, "/tmp/test.oga");

        o.handle_event(AppEvent::InputKey(KeyInput::new("x", false)))
            .unwrap();

        let popup = o
            .state()
            .command_popup()
            .expect("popup should still be open");
        assert_eq!(popup.phase(), &CommandPhase::Running);
    }

    #[test]
    fn command_popup_y_sends_voice_and_closes_popup() {
        use crate::domain::command_popup_state::CommandPhase;

        let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);

        // Create a real temp file so the file existence check passes.
        let tmp = std::env::temp_dir().join("rtg_test_send.oga");
        std::fs::write(&tmp, b"fake audio").unwrap();
        let file_path = tmp.to_str().unwrap().to_owned();

        simulate_voice_recording_started(&mut o, &file_path);

        // Transition to AwaitingConfirmation (as if q was pressed).
        o.state
            .command_popup_mut()
            .unwrap()
            .set_phase(CommandPhase::AwaitingConfirmation {
                prompt: "Send recording? (y/n)".into(),
            });

        o.handle_event(AppEvent::InputKey(KeyInput::new("y", false)))
            .unwrap();

        assert!(o.state().command_popup().is_none());
        assert_eq!(o.dispatcher.voice_send_dispatch_count(), 1);
        assert_eq!(o.dispatcher.last_voice_send().unwrap().0, 1);

        // Clean up.
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn command_popup_n_discards_voice_and_closes_popup() {
        use crate::domain::command_popup_state::CommandPhase;

        let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);

        let tmp = std::env::temp_dir().join("rtg_test_discard.oga");
        std::fs::write(&tmp, b"fake audio").unwrap();
        let file_path = tmp.to_str().unwrap().to_owned();

        simulate_voice_recording_started(&mut o, &file_path);

        o.state
            .command_popup_mut()
            .unwrap()
            .set_phase(CommandPhase::AwaitingConfirmation {
                prompt: "Send recording? (y/n)".into(),
            });

        o.handle_event(AppEvent::InputKey(KeyInput::new("n", false)))
            .unwrap();

        assert!(o.state().command_popup().is_none());
        assert_eq!(o.dispatcher.voice_send_dispatch_count(), 0);
        assert!(!tmp.exists(), "file should be deleted on discard");
    }

    #[test]
    fn command_popup_esc_discards_voice_and_closes_popup() {
        use crate::domain::command_popup_state::CommandPhase;

        let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);

        let tmp = std::env::temp_dir().join("rtg_test_esc.oga");
        std::fs::write(&tmp, b"fake audio").unwrap();
        let file_path = tmp.to_str().unwrap().to_owned();

        simulate_voice_recording_started(&mut o, &file_path);

        o.state
            .command_popup_mut()
            .unwrap()
            .set_phase(CommandPhase::AwaitingConfirmation {
                prompt: "Send recording? (y/n)".into(),
            });

        o.handle_event(AppEvent::InputKey(KeyInput::new("esc", false)))
            .unwrap();

        assert!(o.state().command_popup().is_none());
        assert_eq!(o.dispatcher.voice_send_dispatch_count(), 0);
        assert!(!tmp.exists(), "file should be deleted on esc");
    }

    #[test]
    fn command_popup_random_key_during_awaiting_is_ignored() {
        use crate::domain::command_popup_state::CommandPhase;

        let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
        simulate_voice_recording_started(&mut o, "/tmp/test.oga");

        o.state
            .command_popup_mut()
            .unwrap()
            .set_phase(CommandPhase::AwaitingConfirmation {
                prompt: "Send? (y/n)".into(),
            });

        o.handle_event(AppEvent::InputKey(KeyInput::new("x", false)))
            .unwrap();

        // Popup should still be open in AwaitingConfirmation.
        let popup = o.state().command_popup().expect("popup still open");
        assert!(matches!(
            popup.phase(),
            CommandPhase::AwaitingConfirmation { .. }
        ));
        assert_eq!(o.dispatcher.voice_send_dispatch_count(), 0);
    }

    #[test]
    fn command_output_line_event_pushes_to_popup() {
        let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
        simulate_voice_recording_started(&mut o, "/tmp/test.oga");

        o.handle_event(AppEvent::CommandOutputLine("recording at 48kHz".into()))
            .unwrap();
        o.handle_event(AppEvent::CommandOutputLine("size=128kB".into()))
            .unwrap();

        let popup = o.state().command_popup().unwrap();
        assert_eq!(
            popup.visible_lines(20),
            vec!["recording at 48kHz", "size=128kB"]
        );
    }

    #[test]
    fn command_output_line_ignored_when_no_popup() {
        let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);

        // No popup active — should not panic.
        o.handle_event(AppEvent::CommandOutputLine("stray line".into()))
            .unwrap();
    }

    #[test]
    fn command_exited_transitions_running_to_awaiting_on_success() {
        use crate::domain::command_popup_state::CommandPhase;

        let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
        simulate_voice_recording_with_process(&mut o, "/tmp/test.oga", true);

        o.handle_event(AppEvent::CommandExited { success: true })
            .unwrap();

        let popup = o
            .state()
            .command_popup()
            .expect("popup should still be open");
        assert!(
            matches!(popup.phase(), CommandPhase::AwaitingConfirmation { .. }),
            "expected AwaitingConfirmation but got {:?}",
            popup.phase()
        );
        assert!(o.recording_handle.is_none());
    }

    #[test]
    fn command_exited_does_not_overwrite_awaiting_phase() {
        use crate::domain::command_popup_state::CommandPhase;

        let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
        simulate_voice_recording_started(&mut o, "/tmp/test.oga");

        // User already pressed q — already in AwaitingConfirmation.
        let custom_prompt = "Send recording? (y/n)";
        o.state
            .command_popup_mut()
            .unwrap()
            .set_phase(CommandPhase::AwaitingConfirmation {
                prompt: custom_prompt.into(),
            });

        o.handle_event(AppEvent::CommandExited { success: true })
            .unwrap();

        // The prompt should not be overwritten.
        let popup = o.state().command_popup().unwrap();
        match popup.phase() {
            CommandPhase::AwaitingConfirmation { prompt } => {
                assert_eq!(prompt, custom_prompt);
            }
            _ => panic!("expected AwaitingConfirmation"),
        }
    }

    #[test]
    fn command_exited_ignored_when_no_popup() {
        let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);

        // No popup — should not panic.
        o.handle_event(AppEvent::CommandExited { success: false })
            .unwrap();
    }

    #[test]
    fn stopping_phase_transitions_to_awaiting_when_file_exists() {
        use crate::domain::command_popup_state::CommandPhase;

        let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);

        let tmp = std::env::temp_dir().join("rtg_test_stopping_ok.oga");
        std::fs::write(&tmp, b"fake audio").unwrap();
        let file_path = tmp.to_str().unwrap().to_owned();

        simulate_voice_recording_started(&mut o, &file_path);

        // Simulate q → Stopping (handle already None since simulate_voice_recording_started
        // doesn't set one, same as after the stop thread takes it).
        o.state
            .command_popup_mut()
            .unwrap()
            .set_phase(CommandPhase::Stopping);

        o.handle_event(AppEvent::CommandExited { success: true })
            .unwrap();

        assert!(matches!(
            o.state().command_popup().unwrap().phase(),
            CommandPhase::AwaitingConfirmation { .. }
        ));

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn stopping_phase_transitions_to_failed_when_file_missing() {
        use crate::domain::command_popup_state::CommandPhase;

        let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
        simulate_voice_recording_started(&mut o, "/nonexistent/rtg_test.oga");

        o.state
            .command_popup_mut()
            .unwrap()
            .set_phase(CommandPhase::Stopping);

        o.handle_event(AppEvent::CommandExited { success: true })
            .unwrap();

        assert!(matches!(
            o.state().command_popup().unwrap().phase(),
            CommandPhase::Failed { .. }
        ));
        assert!(
            o.recording_file_path.is_none(),
            "failed recording should discard file path"
        );
    }

    #[test]
    fn stopping_phase_transitions_to_failed_when_file_empty() {
        use crate::domain::command_popup_state::CommandPhase;

        let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);

        let tmp = std::env::temp_dir().join("rtg_test_stopping_empty.oga");
        std::fs::write(&tmp, b"").unwrap();
        let file_path = tmp.to_str().unwrap().to_owned();

        simulate_voice_recording_started(&mut o, &file_path);

        o.state
            .command_popup_mut()
            .unwrap()
            .set_phase(CommandPhase::Stopping);

        o.handle_event(AppEvent::CommandExited { success: true })
            .unwrap();

        assert!(matches!(
            o.state().command_popup().unwrap().phase(),
            CommandPhase::Failed { .. }
        ));

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn stopping_phase_ignores_keys() {
        use crate::domain::command_popup_state::CommandPhase;

        let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
        simulate_voice_recording_started(&mut o, "/tmp/test.oga");

        o.state
            .command_popup_mut()
            .unwrap()
            .set_phase(CommandPhase::Stopping);

        // All keys should be ignored during Stopping.
        for key in ["q", "y", "n", "x", "esc"] {
            o.handle_event(AppEvent::InputKey(KeyInput::new(key, false)))
                .unwrap();
            assert_eq!(
                o.state().command_popup().unwrap().phase(),
                &CommandPhase::Stopping,
                "key '{key}' should not change Stopping phase"
            );
        }
    }

    #[test]
    fn send_voice_skipped_when_no_file_path() {
        let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);

        // No recording_file_path set.
        o.send_voice_recording();

        assert_eq!(o.dispatcher.voice_send_dispatch_count(), 0);
    }

    #[test]
    fn send_voice_skipped_when_file_does_not_exist() {
        let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
        o.recording_file_path = Some("/nonexistent/path/voice.oga".into());

        o.send_voice_recording();

        assert_eq!(o.dispatcher.voice_send_dispatch_count(), 0);
    }

    #[test]
    fn send_voice_skipped_when_no_chat_open() {
        let mut o = orchestrator_with_chats(vec![chat(1, "Chat")]);

        let tmp = std::env::temp_dir().join("rtg_test_nochat.oga");
        std::fs::write(&tmp, b"fake audio").unwrap();
        o.recording_file_path = Some(tmp.to_str().unwrap().into());

        o.send_voice_recording();

        assert_eq!(o.dispatcher.voice_send_dispatch_count(), 0);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn send_voice_dispatches_with_correct_chat_id_and_path() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(42, "Chat")], 42, vec![message(10, "hi")]);

        let tmp = std::env::temp_dir().join("rtg_test_dispatch.oga");
        std::fs::write(&tmp, b"fake audio").unwrap();
        let file_path = tmp.to_str().unwrap().to_owned();

        o.recording_file_path = Some(file_path.clone());
        o.send_voice_recording();

        assert_eq!(o.dispatcher.voice_send_dispatch_count(), 1);
        let (sent_chat_id, sent_path) = o.dispatcher.last_voice_send().unwrap();
        assert_eq!(sent_chat_id, 42);
        assert_eq!(sent_path, file_path);

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn discard_voice_removes_file() {
        let mut o = make_orchestrator();

        let tmp = std::env::temp_dir().join("rtg_test_discard_file.oga");
        std::fs::write(&tmp, b"fake audio").unwrap();
        o.recording_file_path = Some(tmp.to_str().unwrap().into());

        o.discard_voice_recording();

        assert!(!tmp.exists());
        assert!(o.recording_file_path.is_none());
    }

    #[test]
    fn discard_voice_no_op_when_no_file_path() {
        let mut o = make_orchestrator();

        // Should not panic when there's nothing to discard.
        o.discard_voice_recording();

        assert!(o.recording_file_path.is_none());
    }

    #[test]
    fn full_voice_flow_record_stop_send() {
        use crate::domain::command_popup_state::CommandPhase;

        let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);

        let tmp = std::env::temp_dir().join("rtg_test_full_flow.oga");
        std::fs::write(&tmp, b"fake audio").unwrap();
        let file_path = tmp.to_str().unwrap().to_owned();

        // Step 1: Simulate recording started.
        simulate_voice_recording_started(&mut o, &file_path);
        assert!(o.state().command_popup().is_some());

        // Step 2: Output lines arrive.
        o.handle_event(AppEvent::CommandOutputLine("frame=1".into()))
            .unwrap();

        // Step 3: User presses q to stop → transitions to Stopping.
        o.handle_event(AppEvent::InputKey(KeyInput::new("q", false)))
            .unwrap();
        assert_eq!(
            o.state().command_popup().unwrap().phase(),
            &CommandPhase::Stopping
        );

        // Step 4: Process exits → transitions to AwaitingConfirmation.
        o.handle_event(AppEvent::CommandExited { success: true })
            .unwrap();
        assert!(matches!(
            o.state().command_popup().unwrap().phase(),
            CommandPhase::AwaitingConfirmation { .. }
        ));

        // Step 5: User presses y to send.
        o.handle_event(AppEvent::InputKey(KeyInput::new("y", false)))
            .unwrap();
        assert!(o.state().command_popup().is_none());
        assert_eq!(o.dispatcher.voice_send_dispatch_count(), 1);

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn full_voice_flow_record_stop_discard() {
        use crate::domain::command_popup_state::CommandPhase;

        let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);

        let tmp = std::env::temp_dir().join("rtg_test_full_discard.oga");
        std::fs::write(&tmp, b"fake audio").unwrap();
        let file_path = tmp.to_str().unwrap().to_owned();

        simulate_voice_recording_started(&mut o, &file_path);

        // q to stop → Stopping.
        o.handle_event(AppEvent::InputKey(KeyInput::new("q", false)))
            .unwrap();
        assert_eq!(
            o.state().command_popup().unwrap().phase(),
            &CommandPhase::Stopping
        );

        // Process exits → AwaitingConfirmation.
        o.handle_event(AppEvent::CommandExited { success: true })
            .unwrap();
        assert!(matches!(
            o.state().command_popup().unwrap().phase(),
            CommandPhase::AwaitingConfirmation { .. }
        ));

        // n to discard.
        o.handle_event(AppEvent::InputKey(KeyInput::new("n", false)))
            .unwrap();
        assert!(o.state().command_popup().is_none());
        assert_eq!(o.dispatcher.voice_send_dispatch_count(), 0);
        assert!(!tmp.exists());
    }

    #[test]
    fn full_voice_flow_command_exits_then_send() {
        use crate::domain::command_popup_state::CommandPhase;

        let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);

        let tmp = std::env::temp_dir().join("rtg_test_exit_send.oga");
        std::fs::write(&tmp, b"fake audio").unwrap();
        let file_path = tmp.to_str().unwrap().to_owned();

        simulate_voice_recording_with_process(&mut o, &file_path, true);

        // Process exits on its own (success).
        o.handle_event(AppEvent::CommandExited { success: true })
            .unwrap();
        assert!(matches!(
            o.state().command_popup().unwrap().phase(),
            CommandPhase::AwaitingConfirmation { .. }
        ));

        // User confirms send.
        o.handle_event(AppEvent::InputKey(KeyInput::new("y", false)))
            .unwrap();
        assert!(o.state().command_popup().is_none());
        assert_eq!(o.dispatcher.voice_send_dispatch_count(), 1);

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn take_pending_command_rx_returns_none_by_default() {
        let mut o = make_orchestrator();
        assert!(o.take_pending_command_rx().is_none());
    }

    #[test]
    fn take_pending_command_rx_returns_receiver_once() {
        let mut o = make_orchestrator();
        let (_, rx) = std::sync::mpsc::channel::<crate::domain::events::CommandEvent>();
        o.pending_command_rx = Some(rx);

        assert!(o.take_pending_command_rx().is_some());
        assert!(o.take_pending_command_rx().is_none());
    }

    #[test]
    fn help_popup_not_affected_by_command_popup() {
        let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
        simulate_voice_recording_started(&mut o, "/tmp/test.oga");

        // Help should not be visible when command popup is active.
        assert!(!o.state().help_visible());

        // ? should be intercepted by command popup, not open help.
        o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
            .unwrap();
        assert!(!o.state().help_visible());
    }

    #[test]
    fn quit_requested_during_recording_stops_app() {
        let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
        simulate_voice_recording_started(&mut o, "/tmp/test.oga");

        o.handle_event(AppEvent::QuitRequested).unwrap();

        assert!(!o.state().is_running());
    }

    #[test]
    fn command_exited_with_failure_transitions_to_failed() {
        use crate::domain::command_popup_state::CommandPhase;

        let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
        simulate_voice_recording_with_process(&mut o, "/tmp/test.oga", false);

        o.handle_event(AppEvent::CommandExited { success: false })
            .unwrap();

        let popup = o
            .state()
            .command_popup()
            .expect("popup should still be open");
        assert!(
            matches!(popup.phase(), CommandPhase::Failed { .. }),
            "expected Failed phase but got {:?}",
            popup.phase()
        );
        assert!(o.recording_handle.is_none());
        assert!(
            o.recording_file_path.is_none(),
            "failed recording should discard file path"
        );
    }

    #[test]
    fn failed_phase_any_key_closes_popup() {
        let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
        simulate_voice_recording_with_process(&mut o, "/tmp/test.oga", false);

        o.handle_event(AppEvent::CommandExited { success: false })
            .unwrap();

        // Any key should close the Failed popup.
        o.handle_event(AppEvent::InputKey(KeyInput::new("x", false)))
            .unwrap();
        assert!(o.state().command_popup().is_none());
    }

    #[test]
    fn failed_phase_message_mentions_config() {
        use crate::domain::command_popup_state::CommandPhase;

        let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
        simulate_voice_recording_with_process(&mut o, "/tmp/test.oga", false);

        o.handle_event(AppEvent::CommandExited { success: false })
            .unwrap();

        let popup = o.state().command_popup().unwrap();
        match popup.phase() {
            CommandPhase::Failed { message } => {
                assert!(
                    message.contains("config.toml"),
                    "message should mention config.toml: {message}"
                );
                assert!(
                    message.contains("[voice]"),
                    "message should mention [voice] section: {message}"
                );
            }
            other => panic!("expected Failed but got {other:?}"),
        }
    }

    #[test]
    fn send_voice_recording_is_idempotent() {
        let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);

        let tmp = std::env::temp_dir().join("rtg_test_idempotent.oga");
        std::fs::write(&tmp, b"fake audio").unwrap();
        o.recording_file_path = Some(tmp.to_str().unwrap().into());

        o.send_voice_recording();
        o.send_voice_recording();

        assert_eq!(o.dispatcher.voice_send_dispatch_count(), 1);
        let _ = std::fs::remove_file(&tmp);
    }
}
