use super::{
    chat::ChatSummary, chat_list_state::ChatListState, events::ConnectivityStatus,
    message_input_state::MessageInputState, open_chat_state::OpenChatState,
};

/// Represents which panel currently has focus for keyboard navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActivePane {
    /// The left panel showing the list of chats.
    #[default]
    ChatList,
    /// The right panel showing messages for the selected chat.
    Messages,
    /// The message input field for composing messages.
    MessageInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellState {
    running: bool,
    connectivity_status: ConnectivityStatus,
    chat_list: ChatListState,
    open_chat: OpenChatState,
    message_input: MessageInputState,
    active_pane: ActivePane,
}

impl Default for ShellState {
    fn default() -> Self {
        Self {
            running: true,
            connectivity_status: ConnectivityStatus::Connecting,
            chat_list: ChatListState::default(),
            open_chat: OpenChatState::default(),
            message_input: MessageInputState::default(),
            active_pane: ActivePane::default(),
        }
    }
}

impl ShellState {
    /// Creates a state pre-populated with cached chat list data.
    ///
    /// If `chats` is non-empty, `ChatListState` starts as `Ready` immediately,
    /// allowing the TUI to display cached chats on the very first frame.
    pub fn with_initial_chat_list(chats: Vec<ChatSummary>) -> Self {
        Self {
            chat_list: ChatListState::with_initial_chats(chats),
            ..Default::default()
        }
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn connectivity_status(&self) -> ConnectivityStatus {
        self.connectivity_status
    }

    pub fn set_connectivity_status(&mut self, status: ConnectivityStatus) {
        self.connectivity_status = status;
    }

    #[allow(dead_code)]
    pub fn chat_list(&self) -> &ChatListState {
        &self.chat_list
    }

    #[allow(dead_code)]
    pub fn chat_list_mut(&mut self) -> &mut ChatListState {
        &mut self.chat_list
    }

    pub fn open_chat(&self) -> &OpenChatState {
        &self.open_chat
    }

    pub fn open_chat_mut(&mut self) -> &mut OpenChatState {
        &mut self.open_chat
    }

    pub fn active_pane(&self) -> ActivePane {
        self.active_pane
    }

    pub fn set_active_pane(&mut self, pane: ActivePane) {
        self.active_pane = pane;
    }

    pub fn message_input(&self) -> &MessageInputState {
        &self.message_input
    }

    pub fn message_input_mut(&mut self) -> &mut MessageInputState {
        &mut self.message_input
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::chat::{ChatSummary, ChatType, OutgoingReadStatus};

    fn chat(id: i64, title: &str) -> ChatSummary {
        ChatSummary {
            chat_id: id,
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
        }
    }

    #[test]
    fn default_state_is_running() {
        let state = ShellState::default();
        assert!(state.is_running());
    }

    #[test]
    fn default_connectivity_is_connecting() {
        let state = ShellState::default();
        assert_eq!(state.connectivity_status(), ConnectivityStatus::Connecting);
    }

    #[test]
    fn default_active_pane_is_chat_list() {
        let state = ShellState::default();
        assert_eq!(state.active_pane(), ActivePane::ChatList);
    }

    #[test]
    fn stop_sets_running_to_false() {
        let mut state = ShellState::default();
        state.stop();
        assert!(!state.is_running());
    }

    #[test]
    fn set_connectivity_status_updates_value() {
        let mut state = ShellState::default();
        state.set_connectivity_status(ConnectivityStatus::Connected);
        assert_eq!(state.connectivity_status(), ConnectivityStatus::Connected);
    }

    #[test]
    fn set_active_pane_switches_pane() {
        let mut state = ShellState::default();
        state.set_active_pane(ActivePane::Messages);
        assert_eq!(state.active_pane(), ActivePane::Messages);

        state.set_active_pane(ActivePane::MessageInput);
        assert_eq!(state.active_pane(), ActivePane::MessageInput);
    }

    #[test]
    fn with_initial_chat_list_empty_falls_back_to_default() {
        let state = ShellState::with_initial_chat_list(vec![]);
        assert!(state.is_running());
        assert_eq!(state.connectivity_status(), ConnectivityStatus::Connecting);
        assert_eq!(state.active_pane(), ActivePane::ChatList);
    }

    #[test]
    fn with_initial_chat_list_non_empty_populates_chats() {
        let chats = vec![chat(1, "Alice"), chat(2, "Bob")];
        let state = ShellState::with_initial_chat_list(chats);
        assert!(state.is_running());
        assert_eq!(state.chat_list().chats().len(), 2);
    }

    #[test]
    fn message_input_mut_allows_mutation() {
        let mut state = ShellState::default();
        state.message_input_mut().insert_char('a');
        assert_eq!(state.message_input().text(), "a");
    }
}
