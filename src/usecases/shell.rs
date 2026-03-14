use anyhow::Result;

use crate::{
    domain::{
        chat_list_state::ChatListUiState,
        events::{AppEvent, BackgroundTaskResult},
        shell_state::{ActivePane, ShellState},
    },
    infra::contracts::{ExternalOpener, StorageAdapter},
};

use super::{background::TaskDispatcher, contracts::ShellOrchestrator};

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
    /// Guards against dispatching duplicate chat list requests while one is in-flight.
    chat_list_in_flight: bool,
}

impl<S, O, D> DefaultShellOrchestrator<S, O, D>
where
    S: StorageAdapter,
    O: ExternalOpener,
    D: TaskDispatcher,
{
    pub fn new(storage: S, opener: O, dispatcher: D) -> Self {
        Self {
            state: ShellState::default(),
            storage,
            opener,
            dispatcher,
            chat_list_in_flight: false,
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

        tracing::debug!(chat_id, chat_title = %chat_title, "opening chat (non-blocking)");

        self.state.open_chat_mut().set_loading(chat_id, chat_title);
        self.dispatcher.dispatch_load_messages(chat_id);
    }

    fn handle_chat_list_key(&mut self, key: &str) -> Result<()> {
        match key {
            "j" => self.state.chat_list_mut().select_next(),
            "k" => self.state.chat_list_mut().select_previous(),
            "r" => self.dispatch_chat_list_refresh(),
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
        match key {
            "j" => self.state.open_chat_mut().select_next(),
            "k" => self.state.open_chat_mut().select_previous(),
            "h" | "esc" => self.state.set_active_pane(ActivePane::ChatList),
            "i" => {
                if self.state.open_chat().is_open() {
                    self.state.set_active_pane(ActivePane::MessageInput);
                }
            }
            _ => {}
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

        // Optimistically clear the input; text will be restored on failure
        self.state.message_input_mut().clear();
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
                // Only apply result if the user is still looking at the same chat
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
                        self.state.open_chat_mut().set_ready(messages);
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
                    // Restore the original text for retry
                    self.state.message_input_mut().set_text(&original_text);
                }
            },
            BackgroundTaskResult::MessageSentRefreshCompleted { chat_id, result } => {
                // Only apply if user is still viewing the same chat
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
                        self.state.open_chat_mut().set_ready(messages);
                    }
                    Err(error) => {
                        tracing::warn!(
                            chat_id,
                            code = error.code,
                            "background: message refresh after send failed"
                        );
                        // Don't change UI state — the message was already sent
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
                }
                self.storage.save_last_action("tick")?;
            }
            AppEvent::QuitRequested => {
                // In message input mode, 'q' is handled as text input, not quit
                // QuitRequested is only sent for 'q' and Ctrl+C from event_source
                if self.state.active_pane() == ActivePane::MessageInput {
                    self.handle_message_input_key("q");
                } else {
                    self.state.stop();
                }
            }
            AppEvent::InputKey(key) => {
                if key.ctrl && key.key == "o" {
                    self.opener.open("about:blank")?;
                    self.storage.save_last_action("open")?;
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
            AppEvent::ChatListUpdateRequested => {
                tracing::debug!("orchestrator received chat list update request");
                self.dispatch_chat_list_refresh();
            }
            AppEvent::BackgroundTaskCompleted(result) => {
                self.handle_background_result(result);
            }
        }

        Ok(())
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
        },
        infra::stubs::{NoopOpener, StubStorageAdapter},
    };

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
            outgoing_status: OutgoingReadStatus::default(),
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
        }
    }

    // ── Recording task dispatcher for tests ──

    /// Records what the orchestrator dispatched and allows inspection.
    struct RecordingDispatcher {
        dispatched_chat_list_count: RefCell<usize>,
        dispatched_messages: RefCell<Vec<i64>>,
        dispatched_sends: RefCell<Vec<(i64, String)>>,
    }

    impl RecordingDispatcher {
        fn new() -> Self {
            Self {
                dispatched_chat_list_count: RefCell::new(0),
                dispatched_messages: RefCell::new(Vec::new()),
                dispatched_sends: RefCell::new(Vec::new()),
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
    }

    // ── Test orchestrator factory ──

    type TestOrchestrator =
        DefaultShellOrchestrator<StubStorageAdapter, NoopOpener, RecordingDispatcher>;

    fn make_orchestrator() -> TestOrchestrator {
        DefaultShellOrchestrator::new(
            StubStorageAdapter::default(),
            NoopOpener::default(),
            RecordingDispatcher::new(),
        )
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
        o.handle_event(AppEvent::ChatListUpdateRequested).unwrap();
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
        o.handle_event(AppEvent::InputKey(KeyInput::new("r", false)))
            .unwrap();
        assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);
    }

    #[test]
    fn chat_list_update_event_dispatches_refresh() {
        let mut o = orchestrator_with_chats(vec![chat(1, "General")]);
        o.handle_event(AppEvent::ChatListUpdateRequested).unwrap();
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
        o.handle_event(AppEvent::InputKey(KeyInput::new("r", false)))
            .unwrap();
        assert_eq!(o.dispatcher.chat_list_dispatch_count(), 2);
    }

    #[test]
    fn refresh_from_ready_keeps_data_visible() {
        let mut o = orchestrator_with_chats(vec![chat(1, "General"), chat(2, "Backend")]);
        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();

        // Trigger refresh via "r" key
        o.handle_event(AppEvent::InputKey(KeyInput::new("r", false)))
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

        o.handle_event(AppEvent::ChatListUpdateRequested).unwrap();

        // Must not blink — state stays Ready while background fetch runs
        assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Ready);
        assert_eq!(o.state().chat_list().chats().len(), 1);
        assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);
    }

    #[test]
    fn refresh_from_error_shows_loader() {
        let mut o = make_orchestrator();
        // Simulate an error state
        o.handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::ChatListLoaded {
                result: Err(BackgroundError::new("CHAT_LIST_UNAVAILABLE")),
            },
        ))
        .unwrap();
        assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Error);

        // Refresh from error — should show loader since no data to display
        o.handle_event(AppEvent::InputKey(KeyInput::new("r", false)))
            .unwrap();
        assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Loading);
    }

    #[test]
    fn refresh_from_empty_shows_loader() {
        let mut o = orchestrator_with_chats(vec![]);
        assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Empty);

        // Refresh from empty — should show loader since no data to display
        o.handle_event(AppEvent::InputKey(KeyInput::new("r", false)))
            .unwrap();
        assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Loading);
    }

    #[test]
    fn l_key_opens_chat_and_switches_focus() {
        let mut o = orchestrator_with_chats(vec![chat(1, "General")]);
        o.handle_event(AppEvent::InputKey(KeyInput::new("l", false)))
            .unwrap();

        assert_eq!(o.state().active_pane(), ActivePane::Messages);
        assert_eq!(o.state().open_chat().chat_id(), Some(1));
        assert_eq!(o.dispatcher.messages_dispatch_count(), 1);
    }

    #[test]
    fn h_key_switches_focus_back_to_chat_list() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);

        o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
            .unwrap();
        assert_eq!(o.state().active_pane(), ActivePane::ChatList);
    }

    #[test]
    fn esc_key_switches_focus_back_to_chat_list() {
        let mut o =
            orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);

        o.handle_event(AppEvent::InputKey(KeyInput::new("esc", false)))
            .unwrap();
        assert_eq!(o.state().active_pane(), ActivePane::ChatList);
    }

    #[test]
    fn jk_keys_navigate_messages_when_in_messages_pane() {
        let mut o = orchestrator_with_open_chat(
            vec![chat(1, "General")],
            1,
            vec![message(1, "A"), message(2, "B"), message(3, "C")],
        );

        assert_eq!(o.state().open_chat().selected_index(), Some(2));

        o.handle_event(AppEvent::InputKey(KeyInput::new("k", false)))
            .unwrap();
        assert_eq!(o.state().open_chat().selected_index(), Some(1));

        o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .unwrap();
        assert_eq!(o.state().open_chat().selected_index(), Some(2));
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

        o.handle_event(AppEvent::QuitRequested).unwrap();

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
}
