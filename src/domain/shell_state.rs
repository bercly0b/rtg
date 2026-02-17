use super::{chat_list_state::ChatListState, events::ConnectivityStatus};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellState {
    running: bool,
    connectivity_status: ConnectivityStatus,
    chat_list: ChatListState,
}

impl Default for ShellState {
    fn default() -> Self {
        Self {
            running: true,
            connectivity_status: ConnectivityStatus::Connecting,
            chat_list: ChatListState::default(),
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
}
