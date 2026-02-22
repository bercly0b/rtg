use anyhow::Result;

use crate::{
    domain::{
        chat_list_state::ChatListUiState,
        events::AppEvent,
        shell_state::{ActivePane, ShellState},
    },
    infra::contracts::{ExternalOpener, StorageAdapter},
    usecases::{
        list_chats::{list_chats, ListChatsQuery, ListChatsSource},
        load_messages::{load_messages, LoadMessagesQuery, MessagesSource},
        send_message::{send_message, MessageSender, SendMessageCommand, SendMessageError},
    },
};

use super::contracts::ShellOrchestrator;

pub struct DefaultShellOrchestrator<S, O, C, M, MS>
where
    S: StorageAdapter,
    O: ExternalOpener,
    C: ListChatsSource,
    M: MessagesSource,
    MS: MessageSender,
{
    state: ShellState,
    storage: S,
    opener: O,
    chats_source: C,
    messages_source: M,
    message_sender: MS,
}

impl<S, O, C, M, MS> DefaultShellOrchestrator<S, O, C, M, MS>
where
    S: StorageAdapter,
    O: ExternalOpener,
    C: ListChatsSource,
    M: MessagesSource,
    MS: MessageSender,
{
    pub fn new(
        storage: S,
        opener: O,
        chats_source: C,
        messages_source: M,
        message_sender: MS,
    ) -> Self {
        Self {
            state: ShellState::default(),
            storage,
            opener,
            chats_source,
            messages_source,
            message_sender,
        }
    }

    fn refresh_chat_list(&mut self) {
        let preferred_chat_id = self
            .state
            .chat_list()
            .selected_chat()
            .map(|chat| chat.chat_id);
        tracing::debug!(
            preferred_chat_id = preferred_chat_id,
            "refreshing chat list from source"
        );
        self.state.chat_list_mut().set_loading();

        match list_chats(&self.chats_source, ListChatsQuery::default()) {
            Ok(output) => {
                tracing::debug!(
                    chat_count = output.chats.len(),
                    "chat list refresh completed"
                );
                self.state
                    .chat_list_mut()
                    .set_ready_with_selection_hint(output.chats, preferred_chat_id)
            }
            Err(error) => {
                tracing::warn!(error = ?error, "chat list refresh failed");
                self.state.chat_list_mut().set_error()
            }
        }
    }

    fn open_selected_chat(&mut self) {
        let Some(selected) = self.state.chat_list().selected_chat() else {
            return;
        };

        let chat_id = selected.chat_id;
        let chat_title = selected.title.clone();

        tracing::debug!(chat_id, chat_title = %chat_title, "opening chat");

        self.state.open_chat_mut().set_loading(chat_id, chat_title);

        match load_messages(&self.messages_source, LoadMessagesQuery::new(chat_id)) {
            Ok(output) => {
                tracing::debug!(
                    message_count = output.messages.len(),
                    "chat messages loaded"
                );
                self.state.open_chat_mut().set_ready(output.messages);
            }
            Err(error) => {
                tracing::warn!(error = ?error, "failed to load chat messages");
                self.state.open_chat_mut().set_error();
            }
        }
    }

    fn handle_chat_list_key(&mut self, key: &str) -> Result<()> {
        match key {
            "j" => self.state.chat_list_mut().select_next(),
            "k" => self.state.chat_list_mut().select_previous(),
            "r" => self.refresh_chat_list(),
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

        let Some(chat_id) = self.state.open_chat().chat_id() else {
            return;
        };

        let command = SendMessageCommand { chat_id, text };

        match send_message(&self.message_sender, command) {
            Ok(()) => {
                tracing::debug!(chat_id, "message sent successfully");
                self.state.message_input_mut().clear();
                self.refresh_open_chat_messages();
            }
            Err(SendMessageError::EmptyMessage) => {
                // Ignore empty messages silently
            }
            Err(error) => {
                tracing::warn!(error = ?error, chat_id, "failed to send message");
                // Keep text in input for retry
            }
        }
    }

    fn refresh_open_chat_messages(&mut self) {
        let Some(chat_id) = self.state.open_chat().chat_id() else {
            return;
        };

        match load_messages(&self.messages_source, LoadMessagesQuery::new(chat_id)) {
            Ok(output) => {
                tracing::debug!(
                    message_count = output.messages.len(),
                    "messages refreshed after send"
                );
                self.state.open_chat_mut().set_ready(output.messages);
            }
            Err(error) => {
                tracing::warn!(error = ?error, "failed to refresh messages after send");
                // Don't change UI state on refresh failure - messages were sent
            }
        }
    }
}

impl<S, O, C, M, MS> ShellOrchestrator for DefaultShellOrchestrator<S, O, C, M, MS>
where
    S: StorageAdapter,
    O: ExternalOpener,
    C: ListChatsSource,
    M: MessagesSource,
    MS: MessageSender,
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
                    self.refresh_chat_list();
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
                self.refresh_chat_list();
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, collections::VecDeque};

    use super::*;
    use crate::{
        domain::{
            chat::ChatSummary,
            chat_list_state::ChatListUiState,
            events::{AppEvent, ConnectivityStatus, KeyInput},
            message::Message,
            open_chat_state::OpenChatUiState,
        },
        infra::stubs::{NoopOpener, StubStorageAdapter},
        usecases::{
            list_chats::ListChatsSourceError, load_messages::MessagesSourceError,
            send_message::SendMessageSourceError,
        },
    };

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

    fn message(id: i32, text: &str) -> Message {
        Message {
            id,
            sender_name: "User".to_owned(),
            text: text.to_owned(),
            timestamp_ms: 1000,
            is_outgoing: false,
            media: crate::domain::message::MessageMedia::None,
        }
    }

    struct StubChatsSource {
        responses: RefCell<VecDeque<Result<Vec<ChatSummary>, ListChatsSourceError>>>,
    }

    impl StubChatsSource {
        fn fixed(response: Result<Vec<ChatSummary>, ListChatsSourceError>) -> Self {
            Self {
                responses: RefCell::new(VecDeque::from([response])),
            }
        }

        fn sequence(responses: Vec<Result<Vec<ChatSummary>, ListChatsSourceError>>) -> Self {
            Self {
                responses: RefCell::new(responses.into()),
            }
        }
    }

    impl ListChatsSource for StubChatsSource {
        fn list_chats(&self, _limit: usize) -> Result<Vec<ChatSummary>, ListChatsSourceError> {
            self.responses
                .borrow_mut()
                .pop_front()
                .expect("test source must have enough responses")
        }
    }

    struct StubMessagesSource {
        responses: RefCell<VecDeque<Result<Vec<Message>, MessagesSourceError>>>,
    }

    impl StubMessagesSource {
        fn fixed(response: Result<Vec<Message>, MessagesSourceError>) -> Self {
            Self {
                responses: RefCell::new(VecDeque::from([response])),
            }
        }

        fn sequence(responses: Vec<Result<Vec<Message>, MessagesSourceError>>) -> Self {
            Self {
                responses: RefCell::new(responses.into()),
            }
        }
    }

    impl MessagesSource for StubMessagesSource {
        fn list_messages(
            &self,
            _chat_id: i64,
            _limit: usize,
        ) -> Result<Vec<Message>, MessagesSourceError> {
            self.responses
                .borrow_mut()
                .pop_front()
                .expect("test source must have enough responses")
        }
    }

    struct StubMessageSender {
        responses: RefCell<VecDeque<Result<(), SendMessageSourceError>>>,
        captured_calls: RefCell<Vec<(i64, String)>>,
    }

    impl StubMessageSender {
        fn fixed(response: Result<(), SendMessageSourceError>) -> Self {
            Self {
                responses: RefCell::new(VecDeque::from([response])),
                captured_calls: RefCell::new(Vec::new()),
            }
        }

        fn always_ok() -> Self {
            Self {
                responses: RefCell::new(VecDeque::new()),
                captured_calls: RefCell::new(Vec::new()),
            }
        }

        #[allow(dead_code)]
        fn sequence(responses: Vec<Result<(), SendMessageSourceError>>) -> Self {
            Self {
                responses: RefCell::new(responses.into()),
                captured_calls: RefCell::new(Vec::new()),
            }
        }

        fn calls(&self) -> Vec<(i64, String)> {
            self.captured_calls.borrow().clone()
        }
    }

    impl MessageSender for StubMessageSender {
        fn send_message(&self, chat_id: i64, text: &str) -> Result<(), SendMessageSourceError> {
            self.captured_calls
                .borrow_mut()
                .push((chat_id, text.to_owned()));
            self.responses.borrow_mut().pop_front().unwrap_or(Ok(()))
        }
    }

    fn make_orchestrator(
        chats_response: Result<Vec<ChatSummary>, ListChatsSourceError>,
        messages_response: Result<Vec<Message>, MessagesSourceError>,
    ) -> DefaultShellOrchestrator<
        StubStorageAdapter,
        NoopOpener,
        StubChatsSource,
        StubMessagesSource,
        StubMessageSender,
    > {
        DefaultShellOrchestrator::new(
            StubStorageAdapter::default(),
            NoopOpener::default(),
            StubChatsSource::fixed(chats_response),
            StubMessagesSource::fixed(messages_response),
            StubMessageSender::always_ok(),
        )
    }

    #[test]
    fn stops_on_quit_event() {
        let mut orchestrator = make_orchestrator(Ok(vec![]), Ok(vec![]));

        orchestrator
            .handle_event(AppEvent::QuitRequested)
            .expect("event must be handled");

        assert!(!orchestrator.state().is_running());
    }

    #[test]
    fn keeps_running_on_regular_key() {
        let mut orchestrator = make_orchestrator(Ok(vec![]), Ok(vec![]));

        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("x", false)))
            .expect("event must be handled");

        assert!(orchestrator.state().is_running());
    }

    #[test]
    fn updates_connectivity_status_on_connectivity_event() {
        let mut orchestrator = make_orchestrator(Ok(vec![]), Ok(vec![]));

        orchestrator
            .handle_event(AppEvent::ConnectivityChanged(
                ConnectivityStatus::Disconnected,
            ))
            .expect("connectivity event must be handled");

        assert_eq!(
            orchestrator.state().connectivity_status(),
            ConnectivityStatus::Disconnected
        );
    }

    #[test]
    fn key_contract_navigates_chat_list_with_vim_keys() {
        let mut orchestrator = make_orchestrator(Ok(vec![]), Ok(vec![]));
        orchestrator.state.chat_list_mut().set_ready(vec![
            chat(1, "General"),
            chat(2, "Backend"),
            chat(3, "Ops"),
        ]);

        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .expect("j key should be handled");
        assert_eq!(orchestrator.state().chat_list().selected_index(), Some(1));

        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("k", false)))
            .expect("k key should be handled");
        assert_eq!(orchestrator.state().chat_list().selected_index(), Some(0));
    }

    #[test]
    fn enter_key_opens_chat_and_loads_messages() {
        let mut orchestrator = make_orchestrator(
            Ok(vec![chat(1, "General"), chat(2, "Backend")]),
            Ok(vec![message(1, "Hello"), message(2, "World")]),
        );

        orchestrator
            .handle_event(AppEvent::Tick)
            .expect("tick should load chats");

        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .expect("enter should open chat");

        assert_eq!(orchestrator.state().open_chat().chat_id(), Some(1));
        assert_eq!(orchestrator.state().open_chat().chat_title(), "General");
        assert_eq!(
            orchestrator.state().open_chat().ui_state(),
            OpenChatUiState::Ready
        );
        assert_eq!(orchestrator.state().open_chat().messages().len(), 2);
    }

    #[test]
    fn enter_key_handles_messages_load_error() {
        let mut orchestrator = make_orchestrator(
            Ok(vec![chat(1, "General")]),
            Err(MessagesSourceError::Unavailable),
        );

        orchestrator
            .handle_event(AppEvent::Tick)
            .expect("tick should load chats");

        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .expect("enter should handle error gracefully");

        assert_eq!(orchestrator.state().open_chat().chat_id(), Some(1));
        assert_eq!(
            orchestrator.state().open_chat().ui_state(),
            OpenChatUiState::Error
        );
    }

    #[test]
    fn refresh_key_preserves_selection_by_chat_id_when_possible() {
        let mut orchestrator = DefaultShellOrchestrator::new(
            StubStorageAdapter::default(),
            NoopOpener::default(),
            StubChatsSource::fixed(Ok(vec![
                chat(100, "Infra"),
                chat(2, "Backend"),
                chat(200, "Design"),
            ])),
            StubMessagesSource::fixed(Ok(vec![])),
            StubMessageSender::always_ok(),
        );

        orchestrator
            .state
            .chat_list_mut()
            .set_ready(vec![chat(1, "General"), chat(2, "Backend")]);
        orchestrator.state.chat_list_mut().select_next();

        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("r", false)))
            .expect("refresh key should be handled");

        assert_eq!(orchestrator.state().chat_list().selected_index(), Some(1));
        assert_eq!(
            orchestrator
                .state()
                .chat_list()
                .selected_chat()
                .map(|chat| chat.chat_id),
            Some(2)
        );
    }

    #[test]
    fn refresh_key_uses_deterministic_fallback_when_selected_chat_disappears() {
        let mut orchestrator = DefaultShellOrchestrator::new(
            StubStorageAdapter::default(),
            NoopOpener::default(),
            StubChatsSource::fixed(Ok(vec![chat(10, "Infra"), chat(11, "Design")])),
            StubMessagesSource::fixed(Ok(vec![])),
            StubMessageSender::always_ok(),
        );

        orchestrator
            .state
            .chat_list_mut()
            .set_ready(vec![chat(1, "General"), chat(2, "Backend")]);
        orchestrator.state.chat_list_mut().select_next();

        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("r", false)))
            .expect("refresh key should be handled");

        assert_eq!(orchestrator.state().chat_list().selected_index(), Some(0));
        assert_eq!(
            orchestrator
                .state()
                .chat_list()
                .selected_chat()
                .map(|chat| chat.chat_id),
            Some(10)
        );
    }

    #[test]
    fn refresh_adapter_errors_set_error_state_without_breaking_event_handling() {
        let mut orchestrator = DefaultShellOrchestrator::new(
            StubStorageAdapter::default(),
            NoopOpener::default(),
            StubChatsSource::sequence(vec![
                Err(ListChatsSourceError::Unavailable),
                Ok(vec![chat(1, "General")]),
            ]),
            StubMessagesSource::fixed(Ok(vec![])),
            StubMessageSender::always_ok(),
        );

        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("r", false)))
            .expect("refresh error should not break loop");
        assert_eq!(
            orchestrator.state().chat_list().ui_state(),
            ChatListUiState::Error
        );

        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("r", false)))
            .expect("subsequent refresh should still work");

        assert_eq!(
            orchestrator.state().chat_list().ui_state(),
            ChatListUiState::Ready
        );
        assert_eq!(orchestrator.state().chat_list().selected_index(), Some(0));
    }

    #[test]
    fn chat_list_update_event_triggers_full_refresh_with_selection_preservation() {
        let mut orchestrator = DefaultShellOrchestrator::new(
            StubStorageAdapter::default(),
            NoopOpener::default(),
            StubChatsSource::fixed(Ok(vec![
                chat(10, "Infra"),
                chat(2, "Backend"),
                chat(20, "Design"),
            ])),
            StubMessagesSource::fixed(Ok(vec![])),
            StubMessageSender::always_ok(),
        );

        orchestrator
            .state
            .chat_list_mut()
            .set_ready(vec![chat(1, "General"), chat(2, "Backend")]);
        orchestrator.state.chat_list_mut().select_next();

        orchestrator
            .handle_event(AppEvent::ChatListUpdateRequested)
            .expect("chat update event should trigger refresh");

        assert_eq!(orchestrator.state().chat_list().selected_index(), Some(1));
        assert_eq!(
            orchestrator
                .state()
                .chat_list()
                .selected_chat()
                .map(|chat| chat.chat_id),
            Some(2)
        );
    }

    #[test]
    fn initial_loading_state_is_refreshed_on_tick() {
        let mut orchestrator =
            make_orchestrator(Ok(vec![chat(1, "General"), chat(2, "Backend")]), Ok(vec![]));

        assert_eq!(
            orchestrator.state().chat_list().ui_state(),
            ChatListUiState::Loading
        );

        orchestrator
            .handle_event(AppEvent::Tick)
            .expect("tick should trigger initial refresh");

        assert_eq!(
            orchestrator.state().chat_list().ui_state(),
            ChatListUiState::Ready
        );
        assert_eq!(orchestrator.state().chat_list().selected_index(), Some(0));
    }

    #[test]
    fn integration_smoke_happy_path_startup_load_navigate_and_open_chat() {
        let mut orchestrator = make_orchestrator(
            Ok(vec![chat(1, "General"), chat(2, "Backend"), chat(3, "Ops")]),
            Ok(vec![message(1, "Hello")]),
        );

        assert_eq!(
            orchestrator.state().chat_list().ui_state(),
            ChatListUiState::Loading
        );

        orchestrator
            .handle_event(AppEvent::Tick)
            .expect("startup tick should load chats");
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .expect("navigation should work on ready list");
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .expect("enter should open chat");

        assert_eq!(
            orchestrator.state().chat_list().ui_state(),
            ChatListUiState::Ready
        );
        assert_eq!(orchestrator.state().chat_list().selected_index(), Some(1));
        assert_eq!(
            orchestrator
                .state()
                .chat_list()
                .selected_chat()
                .map(|chat| chat.chat_id),
            Some(2)
        );
        assert_eq!(orchestrator.state().open_chat().chat_id(), Some(2));
        assert_eq!(orchestrator.state().open_chat().chat_title(), "Backend");
        assert_eq!(orchestrator.state().open_chat().messages().len(), 1);
        assert_eq!(
            orchestrator.storage.last_action,
            Some("open_chat".to_owned())
        );
    }

    #[test]
    fn integration_smoke_fallback_error_then_empty_list_remains_stable() {
        let mut orchestrator = DefaultShellOrchestrator::new(
            StubStorageAdapter::default(),
            NoopOpener::default(),
            StubChatsSource::sequence(vec![Err(ListChatsSourceError::Unavailable), Ok(vec![])]),
            StubMessagesSource::fixed(Ok(vec![])),
            StubMessageSender::always_ok(),
        );

        orchestrator
            .handle_event(AppEvent::Tick)
            .expect("error fallback should not break event loop");
        assert_eq!(
            orchestrator.state().chat_list().ui_state(),
            ChatListUiState::Error
        );

        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("r", false)))
            .expect("retry from error should be handled");
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .expect("enter on empty list should be a no-op");

        assert_eq!(
            orchestrator.state().chat_list().ui_state(),
            ChatListUiState::Empty
        );
        assert_eq!(orchestrator.state().chat_list().selected_index(), None);
        assert_eq!(orchestrator.storage.last_action, Some("tick".to_owned()));
        assert!(orchestrator.state().is_running());
    }

    #[test]
    fn enter_key_switches_focus_to_messages_pane() {
        let mut orchestrator = make_orchestrator(
            Ok(vec![chat(1, "General")]),
            Ok(vec![message(1, "Hello"), message(2, "World")]),
        );

        orchestrator
            .handle_event(AppEvent::Tick)
            .expect("tick should load chats");

        assert_eq!(orchestrator.state().active_pane(), ActivePane::ChatList);

        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .expect("enter should open chat");

        assert_eq!(orchestrator.state().active_pane(), ActivePane::Messages);
        assert_eq!(orchestrator.state().open_chat().selected_index(), Some(1)); // Last message
    }

    #[test]
    fn l_key_opens_chat_and_switches_focus() {
        let mut orchestrator =
            make_orchestrator(Ok(vec![chat(1, "General")]), Ok(vec![message(1, "Hello")]));

        orchestrator
            .handle_event(AppEvent::Tick)
            .expect("tick should load chats");

        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("l", false)))
            .expect("l should open chat");

        assert_eq!(orchestrator.state().active_pane(), ActivePane::Messages);
        assert_eq!(orchestrator.state().open_chat().chat_id(), Some(1));
    }

    #[test]
    fn h_key_switches_focus_back_to_chat_list() {
        let mut orchestrator =
            make_orchestrator(Ok(vec![chat(1, "General")]), Ok(vec![message(1, "Hello")]));

        orchestrator
            .handle_event(AppEvent::Tick)
            .expect("tick should load chats");
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .expect("enter should open chat");

        assert_eq!(orchestrator.state().active_pane(), ActivePane::Messages);

        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
            .expect("h should switch back to chat list");

        assert_eq!(orchestrator.state().active_pane(), ActivePane::ChatList);
    }

    #[test]
    fn esc_key_switches_focus_back_to_chat_list() {
        let mut orchestrator =
            make_orchestrator(Ok(vec![chat(1, "General")]), Ok(vec![message(1, "Hello")]));

        orchestrator
            .handle_event(AppEvent::Tick)
            .expect("tick should load chats");
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .expect("enter should open chat");

        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("esc", false)))
            .expect("esc should switch back to chat list");

        assert_eq!(orchestrator.state().active_pane(), ActivePane::ChatList);
    }

    #[test]
    fn jk_keys_navigate_messages_when_in_messages_pane() {
        let mut orchestrator = make_orchestrator(
            Ok(vec![chat(1, "General")]),
            Ok(vec![message(1, "A"), message(2, "B"), message(3, "C")]),
        );

        orchestrator
            .handle_event(AppEvent::Tick)
            .expect("tick should load chats");
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .expect("enter should open chat");

        // Initially at last message (index 2)
        assert_eq!(orchestrator.state().open_chat().selected_index(), Some(2));

        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("k", false)))
            .expect("k should move up");
        assert_eq!(orchestrator.state().open_chat().selected_index(), Some(1));

        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("k", false)))
            .expect("k should move up again");
        assert_eq!(orchestrator.state().open_chat().selected_index(), Some(0));

        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .expect("j should move down");
        assert_eq!(orchestrator.state().open_chat().selected_index(), Some(1));
    }

    #[test]
    fn jk_keys_navigate_chat_list_when_in_chat_list_pane() {
        let mut orchestrator = make_orchestrator(
            Ok(vec![chat(1, "General"), chat(2, "Backend"), chat(3, "Ops")]),
            Ok(vec![]),
        );

        orchestrator
            .handle_event(AppEvent::Tick)
            .expect("tick should load chats");

        assert_eq!(orchestrator.state().active_pane(), ActivePane::ChatList);
        assert_eq!(orchestrator.state().chat_list().selected_index(), Some(0));

        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .expect("j should move down in chat list");
        assert_eq!(orchestrator.state().chat_list().selected_index(), Some(1));

        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("k", false)))
            .expect("k should move up in chat list");
        assert_eq!(orchestrator.state().chat_list().selected_index(), Some(0));
    }

    #[test]
    fn l_key_does_nothing_when_no_chat_selected() {
        let mut orchestrator = make_orchestrator(Ok(vec![]), Ok(vec![]));

        orchestrator
            .handle_event(AppEvent::Tick)
            .expect("tick should complete");

        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("l", false)))
            .expect("l on empty list should be a no-op");

        assert_eq!(orchestrator.state().active_pane(), ActivePane::ChatList);
        assert!(!orchestrator.state().open_chat().is_open());
    }

    #[test]
    fn rapid_pane_switching_maintains_consistent_state() {
        let mut orchestrator = DefaultShellOrchestrator::new(
            StubStorageAdapter::default(),
            NoopOpener::default(),
            StubChatsSource::fixed(Ok(vec![chat(1, "General"), chat(2, "Backend")])),
            StubMessagesSource::sequence(vec![
                Ok(vec![message(1, "Hello")]),
                Ok(vec![message(1, "Hello")]), // For the second open via 'l'
            ]),
            StubMessageSender::always_ok(),
        );

        orchestrator
            .handle_event(AppEvent::Tick)
            .expect("tick should load chats");

        // Rapid switching between panes
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .expect("enter should open chat");
        assert_eq!(orchestrator.state().active_pane(), ActivePane::Messages);

        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
            .expect("h should switch to chat list");
        assert_eq!(orchestrator.state().active_pane(), ActivePane::ChatList);

        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("l", false)))
            .expect("l should switch to messages");
        assert_eq!(orchestrator.state().active_pane(), ActivePane::Messages);

        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("esc", false)))
            .expect("esc should switch to chat list");
        assert_eq!(orchestrator.state().active_pane(), ActivePane::ChatList);

        // State should be consistent after rapid switching
        assert!(orchestrator.state().is_running());
        assert!(orchestrator.state().open_chat().is_open());
        assert_eq!(orchestrator.state().open_chat().chat_id(), Some(1));
    }

    #[test]
    fn i_key_switches_to_message_input_mode_when_chat_is_open() {
        let mut orchestrator =
            make_orchestrator(Ok(vec![chat(1, "General")]), Ok(vec![message(1, "Hello")]));

        orchestrator
            .handle_event(AppEvent::Tick)
            .expect("tick should load chats");
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .expect("enter should open chat");

        assert_eq!(orchestrator.state().active_pane(), ActivePane::Messages);

        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .expect("i should switch to message input");

        assert_eq!(orchestrator.state().active_pane(), ActivePane::MessageInput);
    }

    #[test]
    fn i_key_does_nothing_when_no_chat_is_open() {
        let mut orchestrator = make_orchestrator(Ok(vec![chat(1, "General")]), Ok(vec![]));

        orchestrator
            .handle_event(AppEvent::Tick)
            .expect("tick should load chats");

        // Force switch to Messages pane without opening a chat
        orchestrator.state.set_active_pane(ActivePane::Messages);
        assert!(!orchestrator.state().open_chat().is_open());

        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .expect("i should be handled");

        // Should stay in Messages pane since no chat is open
        assert_eq!(orchestrator.state().active_pane(), ActivePane::Messages);
    }

    #[test]
    fn esc_key_switches_from_message_input_to_messages_pane() {
        let mut orchestrator =
            make_orchestrator(Ok(vec![chat(1, "General")]), Ok(vec![message(1, "Hello")]));

        orchestrator
            .handle_event(AppEvent::Tick)
            .expect("tick should load chats");
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .expect("enter should open chat");
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .expect("i should switch to message input");

        assert_eq!(orchestrator.state().active_pane(), ActivePane::MessageInput);

        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("esc", false)))
            .expect("esc should switch back to messages");

        assert_eq!(orchestrator.state().active_pane(), ActivePane::Messages);
    }

    #[test]
    fn text_input_in_message_input_mode() {
        let mut orchestrator =
            make_orchestrator(Ok(vec![chat(1, "General")]), Ok(vec![message(1, "Hello")]));

        orchestrator
            .handle_event(AppEvent::Tick)
            .expect("tick should load chats");
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .expect("enter should open chat");
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .expect("i should switch to message input");

        // Type "Hi"
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("H", false)))
            .expect("H should be typed");
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .expect("i should be typed (not switch mode)");

        assert_eq!(orchestrator.state().message_input().text(), "Hi");
        assert_eq!(orchestrator.state().active_pane(), ActivePane::MessageInput);
    }

    #[test]
    fn backspace_deletes_character_in_message_input_mode() {
        let mut orchestrator =
            make_orchestrator(Ok(vec![chat(1, "General")]), Ok(vec![message(1, "Hello")]));

        orchestrator
            .handle_event(AppEvent::Tick)
            .expect("tick should load chats");
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .expect("enter should open chat");
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .expect("i should switch to message input");

        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("H", false)))
            .expect("H should be typed");
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .expect("i should be typed");
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("backspace", false)))
            .expect("backspace should delete");

        assert_eq!(orchestrator.state().message_input().text(), "H");
    }

    #[test]
    fn cursor_navigation_in_message_input_mode() {
        let mut orchestrator =
            make_orchestrator(Ok(vec![chat(1, "General")]), Ok(vec![message(1, "Hello")]));

        orchestrator
            .handle_event(AppEvent::Tick)
            .expect("tick should load chats");
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .expect("enter should open chat");
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .expect("i should switch to message input");

        // Type "abc"
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("a", false)))
            .expect("a should be typed");
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("b", false)))
            .expect("b should be typed");
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("c", false)))
            .expect("c should be typed");

        assert_eq!(orchestrator.state().message_input().cursor_position(), 3);

        // Move left
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("left", false)))
            .expect("left should move cursor");
        assert_eq!(orchestrator.state().message_input().cursor_position(), 2);

        // Move to home
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("home", false)))
            .expect("home should move cursor");
        assert_eq!(orchestrator.state().message_input().cursor_position(), 0);

        // Move to end
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("end", false)))
            .expect("end should move cursor");
        assert_eq!(orchestrator.state().message_input().cursor_position(), 3);
    }

    #[test]
    fn q_key_types_q_in_message_input_mode_instead_of_quitting() {
        let mut orchestrator =
            make_orchestrator(Ok(vec![chat(1, "General")]), Ok(vec![message(1, "Hello")]));

        orchestrator
            .handle_event(AppEvent::Tick)
            .expect("tick should load chats");
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .expect("enter should open chat");
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .expect("i should switch to message input");

        // 'q' in message input mode should type 'q', not quit
        orchestrator
            .handle_event(AppEvent::QuitRequested)
            .expect("q should be handled as input");

        assert!(orchestrator.state().is_running());
        assert_eq!(orchestrator.state().message_input().text(), "q");
    }

    #[test]
    fn message_input_state_preserved_when_switching_panes() {
        let mut orchestrator =
            make_orchestrator(Ok(vec![chat(1, "General")]), Ok(vec![message(1, "Hello")]));

        orchestrator
            .handle_event(AppEvent::Tick)
            .expect("tick should load chats");
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .expect("enter should open chat");
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .expect("i should switch to message input");

        // Type something
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("H", false)))
            .expect("H should be typed");
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .expect("i should be typed");

        // Switch back to messages
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("esc", false)))
            .expect("esc should switch to messages");

        // Text should be preserved
        assert_eq!(orchestrator.state().message_input().text(), "Hi");

        // Switch back to input mode
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .expect("i should switch to message input");

        // Text should still be there
        assert_eq!(orchestrator.state().message_input().text(), "Hi");
    }

    #[test]
    fn enter_key_sends_message_and_clears_input() {
        let mut orchestrator = DefaultShellOrchestrator::new(
            StubStorageAdapter::default(),
            NoopOpener::default(),
            StubChatsSource::fixed(Ok(vec![chat(1, "General")])),
            StubMessagesSource::sequence(vec![
                Ok(vec![message(1, "Hello")]),                   // Initial load
                Ok(vec![message(1, "Hello"), message(2, "Hi")]), // After send refresh
            ]),
            StubMessageSender::fixed(Ok(())),
        );

        // Load chats, open chat, switch to input mode
        orchestrator.handle_event(AppEvent::Tick).unwrap();
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .unwrap();

        // Type "Hi"
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("H", false)))
            .unwrap();
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .unwrap();

        assert_eq!(orchestrator.state().message_input().text(), "Hi");

        // Send message
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        // Input should be cleared
        assert_eq!(orchestrator.state().message_input().text(), "");
        // Messages should be refreshed (now has 2 messages)
        assert_eq!(orchestrator.state().open_chat().messages().len(), 2);
        // Should stay in message input mode
        assert_eq!(orchestrator.state().active_pane(), ActivePane::MessageInput);
    }

    #[test]
    fn enter_key_with_empty_input_does_nothing() {
        let mut orchestrator = DefaultShellOrchestrator::new(
            StubStorageAdapter::default(),
            NoopOpener::default(),
            StubChatsSource::fixed(Ok(vec![chat(1, "General")])),
            StubMessagesSource::fixed(Ok(vec![message(1, "Hello")])),
            StubMessageSender::fixed(Err(SendMessageSourceError::Unavailable)), // Should not be called
        );

        // Load chats, open chat, switch to input mode
        orchestrator.handle_event(AppEvent::Tick).unwrap();
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .unwrap();

        // Press enter without typing anything
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        // Nothing should happen - no error, input still empty
        assert_eq!(orchestrator.state().message_input().text(), "");
        assert_eq!(orchestrator.state().active_pane(), ActivePane::MessageInput);
    }

    #[test]
    fn enter_key_with_whitespace_only_does_nothing() {
        let mut orchestrator = DefaultShellOrchestrator::new(
            StubStorageAdapter::default(),
            NoopOpener::default(),
            StubChatsSource::fixed(Ok(vec![chat(1, "General")])),
            StubMessagesSource::fixed(Ok(vec![message(1, "Hello")])),
            StubMessageSender::fixed(Err(SendMessageSourceError::Unavailable)), // Should not be called
        );

        // Load chats, open chat, switch to input mode
        orchestrator.handle_event(AppEvent::Tick).unwrap();
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .unwrap();

        // Type only spaces
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new(" ", false)))
            .unwrap();
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new(" ", false)))
            .unwrap();

        // Press enter
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        // Input should remain unchanged (spaces still there)
        assert_eq!(orchestrator.state().message_input().text(), "  ");
    }

    #[test]
    fn send_message_error_keeps_text_in_input() {
        let mut orchestrator = DefaultShellOrchestrator::new(
            StubStorageAdapter::default(),
            NoopOpener::default(),
            StubChatsSource::fixed(Ok(vec![chat(1, "General")])),
            StubMessagesSource::fixed(Ok(vec![message(1, "Hello")])),
            StubMessageSender::fixed(Err(SendMessageSourceError::Unavailable)),
        );

        // Load chats, open chat, switch to input mode
        orchestrator.handle_event(AppEvent::Tick).unwrap();
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .unwrap();

        // Type "Test message"
        for c in "Test message".chars() {
            orchestrator
                .handle_event(AppEvent::InputKey(KeyInput::new(&c.to_string(), false)))
                .unwrap();
        }

        // Try to send - should fail
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        // Text should be preserved for retry
        assert_eq!(orchestrator.state().message_input().text(), "Test message");
        // Should stay in message input mode
        assert_eq!(orchestrator.state().active_pane(), ActivePane::MessageInput);
    }

    #[test]
    fn send_message_passes_correct_chat_id_and_text() {
        let sender = StubMessageSender::always_ok();
        let mut orchestrator = DefaultShellOrchestrator::new(
            StubStorageAdapter::default(),
            NoopOpener::default(),
            StubChatsSource::fixed(Ok(vec![chat(42, "TestChat")])),
            StubMessagesSource::sequence(vec![
                Ok(vec![message(1, "Hello")]),
                Ok(vec![message(1, "Hello")]),
            ]),
            sender,
        );

        // Load chats, open chat, switch to input mode
        orchestrator.handle_event(AppEvent::Tick).unwrap();
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
            .unwrap();

        // Type "Hello World"
        for c in "  Hello World  ".chars() {
            orchestrator
                .handle_event(AppEvent::InputKey(KeyInput::new(&c.to_string(), false)))
                .unwrap();
        }

        // Send message
        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .unwrap();

        // Verify send was called with correct parameters (trimmed text)
        let calls = orchestrator.message_sender.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, 42); // chat_id
        assert_eq!(calls[0].1, "Hello World"); // trimmed text
    }
}
