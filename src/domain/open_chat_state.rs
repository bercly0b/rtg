use super::message::Message;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenChatUiState {
    Empty,
    Loading,
    Ready,
    Error,
}

/// Scroll margin — number of items to keep visible above/below cursor before scrolling.
/// Used by the UI layer via `List::scroll_padding()`.
pub const SCROLL_MARGIN: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenChatState {
    chat_id: Option<i64>,
    chat_title: String,
    messages: Vec<Message>,
    ui_state: OpenChatUiState,
    selected_index: Option<usize>,
    scroll_offset: usize,
}

impl Default for OpenChatState {
    fn default() -> Self {
        Self {
            chat_id: None,
            chat_title: String::new(),
            messages: Vec::new(),
            ui_state: OpenChatUiState::Empty,
            selected_index: None,
            scroll_offset: 0,
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

    /// Returns the selected message index for scroll positioning.
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    /// Returns the current scroll offset (persisted between frames for ratatui `ListState`).
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Saves the scroll offset computed by ratatui after rendering, so it persists across frames.
    ///
    /// Note: `set_ready()` initializes this to `usize::MAX` as a sentinel to trigger
    /// scroll-to-bottom on first render; ratatui clamps it to a valid range.
    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_offset = offset;
    }

    pub fn set_loading(&mut self, chat_id: i64, chat_title: String) {
        self.chat_id = Some(chat_id);
        self.chat_title = chat_title;
        self.messages.clear();
        self.ui_state = OpenChatUiState::Loading;
        self.selected_index = None;
        self.scroll_offset = 0;
    }

    pub fn set_ready(&mut self, messages: Vec<Message>) {
        self.selected_index = if messages.is_empty() {
            None
        } else {
            Some(messages.len() - 1)
        };
        // When there are messages, set a large initial offset so ratatui places
        // the last message at the bottom of the viewport on first render.
        // The actual value will be clamped by ratatui's `get_items_bounds()`.
        self.scroll_offset = if messages.is_empty() { 0 } else { usize::MAX };
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
        self.selected_index = None;
        self.scroll_offset = 0;
    }

    pub fn is_open(&self) -> bool {
        self.chat_id.is_some()
    }

    /// Selects the next message (moves down in the list).
    pub fn select_next(&mut self) {
        if self.messages.is_empty() {
            return;
        }

        self.selected_index = match self.selected_index {
            None => Some(0),
            Some(idx) if idx + 1 < self.messages.len() => Some(idx + 1),
            Some(idx) => Some(idx), // Already at the last message
        };
    }

    /// Selects the previous message (moves up in the list).
    pub fn select_previous(&mut self) {
        if self.messages.is_empty() {
            return;
        }

        self.selected_index = match self.selected_index {
            None => Some(self.messages.len() - 1),
            Some(0) => Some(0), // Already at the first message
            Some(idx) => Some(idx - 1),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn default_state_is_empty() {
        let state = OpenChatState::default();

        assert_eq!(state.ui_state(), OpenChatUiState::Empty);
        assert!(!state.is_open());
        assert!(state.messages().is_empty());
        assert_eq!(state.selected_index(), None);
    }

    #[test]
    fn set_loading_transitions_correctly() {
        let mut state = OpenChatState::default();

        state.set_loading(42, "Test Chat".to_owned());

        assert_eq!(state.chat_id(), Some(42));
        assert_eq!(state.chat_title(), "Test Chat");
        assert_eq!(state.ui_state(), OpenChatUiState::Loading);
        assert_eq!(state.selected_index(), None);
    }

    #[test]
    fn set_ready_stores_messages_and_selects_last() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());

        state.set_ready(vec![message(1, "Hello"), message(2, "World")]);

        assert_eq!(state.ui_state(), OpenChatUiState::Ready);
        assert_eq!(state.messages().len(), 2);
        assert_eq!(state.selected_index(), Some(1));
    }

    #[test]
    fn set_ready_with_empty_messages_has_no_selection() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());

        state.set_ready(vec![]);

        assert_eq!(state.ui_state(), OpenChatUiState::Ready);
        assert!(state.messages().is_empty());
        assert_eq!(state.selected_index(), None);
        assert_eq!(state.scroll_offset(), 0);
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
        assert_eq!(state.selected_index(), None);
    }

    #[test]
    fn select_next_moves_down_in_message_list() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());
        state.set_ready(vec![message(1, "A"), message(2, "B"), message(3, "C")]);

        // Initially at last message (index 2)
        assert_eq!(state.selected_index(), Some(2));

        // Move to beginning for testing
        state.selected_index = Some(0);

        state.select_next();
        assert_eq!(state.selected_index(), Some(1));

        state.select_next();
        assert_eq!(state.selected_index(), Some(2));

        // At the end, should stay at last
        state.select_next();
        assert_eq!(state.selected_index(), Some(2));
    }

    #[test]
    fn select_previous_moves_up_in_message_list() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());
        state.set_ready(vec![message(1, "A"), message(2, "B"), message(3, "C")]);

        // Initially at last message (index 2)
        assert_eq!(state.selected_index(), Some(2));

        state.select_previous();
        assert_eq!(state.selected_index(), Some(1));

        state.select_previous();
        assert_eq!(state.selected_index(), Some(0));

        // At the beginning, should stay at first
        state.select_previous();
        assert_eq!(state.selected_index(), Some(0));
    }

    #[test]
    fn select_next_on_empty_messages_does_nothing() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());
        state.set_ready(vec![]);

        state.select_next();

        assert_eq!(state.selected_index(), None);
    }

    #[test]
    fn select_previous_on_empty_messages_does_nothing() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());
        state.set_ready(vec![]);

        state.select_previous();

        assert_eq!(state.selected_index(), None);
    }

    #[test]
    fn select_next_initializes_to_first_when_no_selection() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());
        state.set_ready(vec![message(1, "A"), message(2, "B")]);
        state.selected_index = None; // Force no selection

        state.select_next();

        assert_eq!(state.selected_index(), Some(0));
    }

    #[test]
    fn select_previous_initializes_to_last_when_no_selection() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());
        state.set_ready(vec![message(1, "A"), message(2, "B")]);
        state.selected_index = None; // Force no selection

        state.select_previous();

        assert_eq!(state.selected_index(), Some(1));
    }

    #[test]
    fn scroll_offset_starts_at_zero() {
        let state = OpenChatState::default();
        assert_eq!(state.scroll_offset(), 0);
    }

    #[test]
    fn scroll_offset_resets_on_set_loading() {
        let mut state = OpenChatState::default();
        state.scroll_offset = 10;

        state.set_loading(1, "Chat".to_owned());

        assert_eq!(state.scroll_offset(), 0);
    }

    #[test]
    fn set_ready_initializes_scroll_offset_to_max() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());

        state.set_ready(vec![message(1, "A"), message(2, "B")]);

        // Large initial offset so ratatui scrolls to show the last message
        assert_eq!(state.scroll_offset(), usize::MAX);
    }

    #[test]
    fn set_scroll_offset_persists_value() {
        let mut state = OpenChatState::default();

        state.set_scroll_offset(42);

        assert_eq!(state.scroll_offset(), 42);
    }

    #[test]
    fn scroll_margin_constant_is_five() {
        assert_eq!(SCROLL_MARGIN, 5);
    }
}
