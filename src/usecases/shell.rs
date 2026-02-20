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
    },
};

use super::contracts::ShellOrchestrator;

pub struct DefaultShellOrchestrator<S, O, C, M>
where
    S: StorageAdapter,
    O: ExternalOpener,
    C: ListChatsSource,
    M: MessagesSource,
{
    state: ShellState,
    storage: S,
    opener: O,
    chats_source: C,
    messages_source: M,
}

impl<S, O, C, M> DefaultShellOrchestrator<S, O, C, M>
where
    S: StorageAdapter,
    O: ExternalOpener,
    C: ListChatsSource,
    M: MessagesSource,
{
    pub fn new(storage: S, opener: O, chats_source: C, messages_source: M) -> Self {
        Self {
            state: ShellState::default(),
            storage,
            opener,
            chats_source,
            messages_source,
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
            _ => {}
        }
        Ok(())
    }
}

impl<S, O, C, M> ShellOrchestrator for DefaultShellOrchestrator<S, O, C, M>
where
    S: StorageAdapter,
    O: ExternalOpener,
    C: ListChatsSource,
    M: MessagesSource,
{
    fn state(&self) -> &ShellState {
        &self.state
    }

    fn handle_event(&mut self, event: AppEvent) -> Result<()> {
        match event {
            AppEvent::Tick => {
                if self.state.chat_list().ui_state() == ChatListUiState::Loading {
                    self.refresh_chat_list();
                }
                self.storage.save_last_action("tick")?;
            }
            AppEvent::QuitRequested => self.state.stop(),
            AppEvent::InputKey(key) => {
                if key.ctrl && key.key == "o" {
                    self.opener.open("about:blank")?;
                    self.storage.save_last_action("open")?;
                    return Ok(());
                }

                match self.state.active_pane() {
                    ActivePane::ChatList => self.handle_chat_list_key(&key.key)?,
                    ActivePane::Messages => self.handle_messages_key(&key.key)?,
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
        usecases::{list_chats::ListChatsSourceError, load_messages::MessagesSourceError},
    };

    fn chat(chat_id: i64, title: &str) -> ChatSummary {
        ChatSummary {
            chat_id,
            title: title.to_owned(),
            unread_count: 0,
            last_message_preview: None,
            last_message_unix_ms: None,
            is_pinned: false,
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

    fn make_orchestrator(
        chats_response: Result<Vec<ChatSummary>, ListChatsSourceError>,
        messages_response: Result<Vec<Message>, MessagesSourceError>,
    ) -> DefaultShellOrchestrator<StubStorageAdapter, NoopOpener, StubChatsSource, StubMessagesSource>
    {
        DefaultShellOrchestrator::new(
            StubStorageAdapter::default(),
            NoopOpener::default(),
            StubChatsSource::fixed(chats_response),
            StubMessagesSource::fixed(messages_response),
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
}
