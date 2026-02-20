use super::{
    chat_list_state::ChatListState, events::ConnectivityStatus,
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
