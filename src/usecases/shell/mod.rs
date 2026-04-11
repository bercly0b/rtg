mod background_results;
mod chat_list;
mod chat_open;
mod chat_updates;
mod message_actions;
mod message_input;
mod message_keys;
mod voice;

use std::sync::Arc;

use anyhow::Result;

use crate::{
    domain::{
        chat_list_state::ChatListUiState,
        events::AppEvent,
        message_cache::DEFAULT_MIN_DISPLAY_MESSAGES,
        shell_state::{ActivePane, ShellState},
    },
    infra::contracts::{ExternalOpener, StorageAdapter},
};

// Re-exported for test modules that rely on `use super::*`.
#[cfg(test)]
use crate::{
    domain::{chat::ChatType, events::ChatUpdate, open_chat_state::MessageSource},
    usecases::chat_subtitle::ChatSubtitleQuery,
};

use super::{
    background::TaskDispatcher, contracts::ShellOrchestrator, load_messages::CachedMessagesSource,
};

/// Default limit for cached message preloading.
const DEFAULT_CACHED_MESSAGES_LIMIT: usize = 50;

/// Mutable context passed to free functions extracted from `DefaultShellOrchestrator`.
///
/// Groups all the borrowed orchestrator fields so that sub-module functions
/// receive a single `&mut OrchestratorCtx` instead of 10+ individual parameters.
pub(super) struct OrchestratorCtx<'a, D: TaskDispatcher> {
    pub state: &'a mut ShellState,
    pub dispatcher: &'a D,
    pub chat_list_in_flight: &'a mut bool,
    pub chat_list_refresh_pending: &'a mut bool,
    pub chat_list_pending_force: &'a mut bool,
    pub user_requested_chat_refresh: &'a mut bool,
    pub messages_refresh_in_flight: &'a mut bool,
    pub active_downloads: &'a mut std::collections::HashMap<i32, (i64, i64)>,
    pub max_auto_download_bytes: u64,
    pub recording_handle: &'a mut Option<super::voice_recording::RecordingHandle>,
    pub recording_file_path: &'a mut Option<String>,
    pub pending_command_rx:
        &'a mut Option<std::sync::mpsc::Receiver<crate::domain::events::CommandEvent>>,
    pub voice_record_cmd: &'a str,
    pub tdlib_opened_chat_id: &'a mut Option<i64>,
    pub prefetch_in_flight: &'a mut Option<i64>,
    pub min_display_messages: usize,
    pub pending_d: &'a mut bool,
    pub cache_source: &'a Option<Arc<dyn CachedMessagesSource>>,
    pub open_handlers: &'a std::collections::HashMap<String, String>,
    pub opener: &'a dyn crate::infra::contracts::ExternalOpener,
}

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
    /// When `true`, a refresh request arrived while one was already in-flight.
    /// The in-flight result may be stale, so another refresh is dispatched
    /// automatically when the current one completes.
    chat_list_refresh_pending: bool,
    /// Accumulated `force` flag for the pending refresh. OR-merged from all
    /// refresh requests that arrived while in-flight: if any of them was
    /// `force=true`, the pending re-dispatch will also be forced.
    chat_list_pending_force: bool,
    /// When `true`, the current in-flight chat list refresh was triggered by the user (R key)
    /// so we should show a status-bar notification when it completes.
    user_requested_chat_refresh: bool,
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
    /// MIME-type → command mappings for opening message files (from config).
    open_handlers: std::collections::HashMap<String, String>,
    /// Tracks active downloads: file_id → (chat_id, message_id).
    /// Used to route `updateFile` events to the correct message in the cache.
    active_downloads: std::collections::HashMap<i32, (i64, i64)>,
    /// Maximum file size (in bytes) for auto-download.
    max_auto_download_bytes: u64,
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
            chat_list_refresh_pending: false,
            chat_list_pending_force: false,
            user_requested_chat_refresh: false,
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
            open_handlers: std::collections::HashMap::new(),
            active_downloads: std::collections::HashMap::new(),
            max_auto_download_bytes: 10_000_000,
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
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_initial_state(
        storage: S,
        opener: O,
        dispatcher: D,
        initial_state: ShellState,
        cache_source: Option<Arc<dyn CachedMessagesSource>>,
        min_display_messages: usize,
        voice_record_cmd: String,
        open_handlers: std::collections::HashMap<String, String>,
        max_auto_download_bytes: u64,
    ) -> Self {
        let initial_refresh_needed = initial_state.chat_list().ui_state() == ChatListUiState::Ready;
        Self {
            state: initial_state,
            storage,
            opener,
            dispatcher,
            cache_source,
            chat_list_in_flight: false,
            chat_list_refresh_pending: false,
            chat_list_pending_force: false,
            user_requested_chat_refresh: false,
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
            open_handlers,
            active_downloads: std::collections::HashMap::new(),
            max_auto_download_bytes,
        }
    }

    fn as_ctx(&mut self) -> OrchestratorCtx<'_, D> {
        OrchestratorCtx {
            state: &mut self.state,
            dispatcher: &self.dispatcher,
            chat_list_in_flight: &mut self.chat_list_in_flight,
            chat_list_refresh_pending: &mut self.chat_list_refresh_pending,
            chat_list_pending_force: &mut self.chat_list_pending_force,
            user_requested_chat_refresh: &mut self.user_requested_chat_refresh,
            messages_refresh_in_flight: &mut self.messages_refresh_in_flight,
            active_downloads: &mut self.active_downloads,
            max_auto_download_bytes: self.max_auto_download_bytes,
            recording_handle: &mut self.recording_handle,
            recording_file_path: &mut self.recording_file_path,
            pending_command_rx: &mut self.pending_command_rx,
            voice_record_cmd: &self.voice_record_cmd,
            tdlib_opened_chat_id: &mut self.tdlib_opened_chat_id,
            prefetch_in_flight: &mut self.prefetch_in_flight,
            min_display_messages: self.min_display_messages,
            pending_d: &mut self.pending_d,
            cache_source: &self.cache_source,
            open_handlers: &self.open_handlers,
            opener: &self.opener,
        }
    }

    fn handle_chat_list_key(&mut self, key: &str) -> Result<()> {
        match key {
            "j" => {
                self.state.chat_list_mut().select_next();
                chat_open::maybe_prefetch_selected_chat(&mut self.as_ctx());
            }
            "k" => {
                self.state.chat_list_mut().select_previous();
                chat_open::maybe_prefetch_selected_chat(&mut self.as_ctx());
            }
            "R" => {
                self.user_requested_chat_refresh = true;
                self.state.set_notification("Refreshing chat list...");
                chat_list::dispatch_chat_list_refresh(&mut self.as_ctx(), true);
            }
            "r" => chat_list::mark_selected_chat_as_read(&mut self.as_ctx()),
            "I" => chat_list::show_chat_info_popup(&mut self.as_ctx()),
            "enter" | "l" => {
                if self.state.chat_list().selected_chat().is_some() {
                    self.state.set_active_pane(ActivePane::Messages);
                    chat_open::open_selected_chat(&mut self.as_ctx());
                    self.storage.save_last_action("open_chat")?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_messages_key(&mut self, key: &str) -> Result<()> {
        message_keys::handle_messages_key(&mut self.as_ctx(), key)
    }

    #[cfg(test)]
    fn send_voice_recording(&mut self) {
        voice::send_voice_recording(&mut self.as_ctx());
    }

    #[cfg(test)]
    fn discard_voice_recording(&mut self) {
        voice::discard_voice_recording(&mut self.as_ctx());
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
                    chat_list::dispatch_chat_list_refresh(&mut self.as_ctx(), false);
                } else if self.initial_refresh_needed {
                    self.initial_refresh_needed = false;
                    chat_list::dispatch_chat_list_refresh(&mut self.as_ctx(), false);
                }
                self.storage.save_last_action("tick")?;
            }
            AppEvent::QuitRequested => {
                chat_open::close_tdlib_chat(&mut self.as_ctx());
                self.state.stop();
            }
            AppEvent::CommandOutputLine { text, replace_last } => {
                if let Some(popup) = self.state.command_popup_mut() {
                    if replace_last {
                        popup.replace_last_line(text);
                    } else {
                        popup.push_line(text);
                    }
                }
            }
            AppEvent::CommandExited { success } => {
                tracing::info!(success, "external command exited");
                voice::handle_command_exited(&mut self.as_ctx(), success);
            }
            AppEvent::InputKey(key) => {
                if self.state.command_popup().is_some() {
                    voice::handle_command_popup_key(&mut self.as_ctx(), &key.key);
                    return Ok(());
                }

                if self.state.chat_info_popup().is_some() {
                    match key.key.as_str() {
                        "q" | "esc" | "I" => self.state.close_chat_info_popup(),
                        _ => {}
                    }
                    return Ok(());
                }

                if self.state.help_visible() {
                    match key.key.as_str() {
                        "q" | "?" | "esc" => self.state.hide_help(),
                        _ => {}
                    }
                    return Ok(());
                }

                if key.key == "?" {
                    match self.state.active_pane() {
                        ActivePane::ChatList | ActivePane::Messages => {
                            self.state.show_help();
                            return Ok(());
                        }
                        ActivePane::MessageInput => {
                            message_input::handle_message_input_key(&mut self.as_ctx(), "?");
                            return Ok(());
                        }
                    }
                }

                if key.key == "q" && !key.ctrl {
                    match self.state.active_pane() {
                        ActivePane::MessageInput => {
                            message_input::handle_message_input_key(&mut self.as_ctx(), "q");
                        }
                        _ => {
                            chat_open::close_tdlib_chat(&mut self.as_ctx());
                            self.state.stop();
                        }
                    }
                    return Ok(());
                }

                match self.state.active_pane() {
                    ActivePane::ChatList => self.handle_chat_list_key(&key.key)?,
                    ActivePane::Messages => self.handle_messages_key(&key.key)?,
                    ActivePane::MessageInput => {
                        message_input::handle_message_input_key(&mut self.as_ctx(), &key.key);
                    }
                }
            }
            AppEvent::ConnectivityChanged(status) => {
                self.state.set_connectivity_status(status);
            }
            AppEvent::ChatUpdateReceived { updates } => {
                tracing::debug!(count = updates.len(), "orchestrator received chat updates");
                chat_updates::handle_chat_updates(&mut self.as_ctx(), updates);
            }
            AppEvent::BackgroundTaskCompleted(result) => {
                background_results::handle_background_result(&mut self.as_ctx(), result);
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
mod tests;
