use super::message::Message;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenChatUiState {
    Empty,
    Loading,
    Ready,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenChatState {
    chat_id: Option<i64>,
    chat_title: String,
    messages: Vec<Message>,
    ui_state: OpenChatUiState,
}

impl Default for OpenChatState {
    fn default() -> Self {
        Self {
            chat_id: None,
            chat_title: String::new(),
            messages: Vec::new(),
            ui_state: OpenChatUiState::Empty,
        }
    }
}

impl OpenChatState {
    #[allow(dead_code)]
    pub fn chat_id(&self) -> Option<i64> {
        self.chat_id
    }

    pub fn chat_title(&self) -> &str {
        &self.chat_title
    }

    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    pub fn ui_state(&self) -> OpenChatUiState {
        self.ui_state.clone()
    }

    pub fn set_loading(&mut self, chat_id: i64, chat_title: String) {
        self.chat_id = Some(chat_id);
        self.chat_title = chat_title;
        self.messages.clear();
        self.ui_state = OpenChatUiState::Loading;
    }

    pub fn set_ready(&mut self, messages: Vec<Message>) {
        self.messages = messages;
        self.ui_state = OpenChatUiState::Ready;
    }

    pub fn set_error(&mut self) {
        self.ui_state = OpenChatUiState::Error;
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.chat_id = None;
        self.chat_title.clear();
        self.messages.clear();
        self.ui_state = OpenChatUiState::Empty;
    }

    pub fn is_open(&self) -> bool {
        self.chat_id.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(id: i32, text: &str) -> Message {
        Message {
            id,
            sender_name: "User".to_owned(),
            text: text.to_owned(),
            timestamp_ms: 1000,
            is_outgoing: false,
        }
    }

    #[test]
    fn default_state_is_empty() {
        let state = OpenChatState::default();

        assert_eq!(state.ui_state(), OpenChatUiState::Empty);
        assert!(!state.is_open());
        assert!(state.messages().is_empty());
    }

    #[test]
    fn set_loading_transitions_correctly() {
        let mut state = OpenChatState::default();

        state.set_loading(42, "Test Chat".to_owned());

        assert_eq!(state.chat_id(), Some(42));
        assert_eq!(state.chat_title(), "Test Chat");
        assert_eq!(state.ui_state(), OpenChatUiState::Loading);
    }

    #[test]
    fn set_ready_stores_messages() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());

        state.set_ready(vec![message(1, "Hello"), message(2, "World")]);

        assert_eq!(state.ui_state(), OpenChatUiState::Ready);
        assert_eq!(state.messages().len(), 2);
    }

    #[test]
    fn set_error_transitions_to_error() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());

        state.set_error();

        assert_eq!(state.ui_state(), OpenChatUiState::Error);
    }

    #[test]
    fn clear_resets_to_empty() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());
        state.set_ready(vec![message(1, "Hi")]);

        state.clear();

        assert_eq!(state.ui_state(), OpenChatUiState::Empty);
        assert!(!state.is_open());
        assert!(state.messages().is_empty());
    }
}
