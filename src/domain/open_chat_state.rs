use super::chat_subtitle::ChatSubtitle;
use super::message::{Message, MessageStatus};

// ─── Scroll offset ──────────────────────────────────────────────────────────

/// Scroll offset with line-level precision.
///
/// `(item, line)` — the index of the first visible item and how many of its
/// top lines are clipped (hidden above the viewport).
///
/// Defined in the domain layer because it is part of `OpenChatState` and must
/// be independent of any specific UI widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollOffset {
    pub item: usize,
    pub line: usize,
}

impl ScrollOffset {
    pub const ZERO: Self = Self { item: 0, line: 0 };

    /// Sentinel value meaning "compute bottom-aligned offset on first render".
    pub const BOTTOM: Self = Self {
        item: usize::MAX,
        line: 0,
    };

    pub fn is_bottom_sentinel(self) -> bool {
        self.item == usize::MAX
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenChatUiState {
    Empty,
    Loading,
    Ready,
    Error,
}

/// Scroll margin — number of items to keep visible above/below cursor before scrolling.
/// Used by the UI layer via `ChatMessageList::scroll_padding()`.
pub const SCROLL_MARGIN: usize = 5;

/// Source of the currently displayed messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageSource {
    /// No data loaded yet (initial state).
    None,
    /// Messages were served from the in-memory app cache.
    Cache,
    /// Messages were fetched from the network (TDLib remote).
    Live,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenChatState {
    chat_id: Option<i64>,
    chat_title: String,
    chat_subtitle: ChatSubtitle,
    messages: Vec<Message>,
    ui_state: OpenChatUiState,
    selected_index: Option<usize>,
    scroll_offset: ScrollOffset,
    /// Whether a background refresh is in-flight while cached messages are shown.
    refreshing: bool,
    /// How the currently displayed messages were obtained.
    message_source: MessageSource,
}

impl Default for OpenChatState {
    fn default() -> Self {
        Self {
            chat_id: None,
            chat_title: String::new(),
            chat_subtitle: ChatSubtitle::None,
            messages: Vec::new(),
            ui_state: OpenChatUiState::Empty,
            selected_index: None,
            scroll_offset: ScrollOffset::ZERO,
            refreshing: false,
            message_source: MessageSource::None,
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

    pub fn chat_subtitle(&self) -> &ChatSubtitle {
        &self.chat_subtitle
    }

    pub fn set_chat_subtitle(&mut self, subtitle: ChatSubtitle) {
        self.chat_subtitle = subtitle;
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

    /// Returns the current scroll offset (persisted between frames).
    pub fn scroll_offset(&self) -> ScrollOffset {
        self.scroll_offset
    }

    /// Saves the scroll offset computed after rendering, so it persists across frames.
    ///
    /// Note: `set_ready()` initializes this to `ScrollOffset::BOTTOM` as a sentinel
    /// to trigger scroll-to-bottom on first render.
    pub fn set_scroll_offset(&mut self, offset: ScrollOffset) {
        self.scroll_offset = offset;
    }

    pub fn is_refreshing(&self) -> bool {
        self.refreshing
    }

    pub fn set_refreshing(&mut self, refreshing: bool) {
        self.refreshing = refreshing;
    }

    pub fn message_source(&self) -> MessageSource {
        self.message_source
    }

    pub fn set_message_source(&mut self, source: MessageSource) {
        self.message_source = source;
    }

    pub fn set_loading(&mut self, chat_id: i64, chat_title: String) {
        self.chat_id = Some(chat_id);
        self.chat_title = chat_title;
        self.chat_subtitle = ChatSubtitle::None;
        self.messages.clear();
        self.ui_state = OpenChatUiState::Loading;
        self.selected_index = None;
        self.scroll_offset = ScrollOffset::ZERO;
        self.refreshing = false;
        self.message_source = MessageSource::None;
    }

    /// Transitions to `Ready` with the given messages.
    ///
    /// Does NOT modify `refreshing` or `message_source` — callers must
    /// set these explicitly after calling `set_ready()` to indicate
    /// whether the data is cached/live and if a refresh is in-flight.
    pub fn set_ready(&mut self, messages: Vec<Message>) {
        self.selected_index = if messages.is_empty() {
            None
        } else {
            Some(messages.len() - 1)
        };
        // When there are messages, use the BOTTOM sentinel so the custom
        // ChatMessageList widget computes a bottom-aligned offset on first render.
        self.scroll_offset = if messages.is_empty() {
            ScrollOffset::ZERO
        } else {
            ScrollOffset::BOTTOM
        };
        self.messages = messages;
        self.ui_state = OpenChatUiState::Ready;
    }

    /// Updates messages in an already-`Ready` chat without resetting scroll.
    ///
    /// If the previously selected message (by ID) still exists in the new
    /// list, the selection stays on it. Otherwise, the selection moves to
    /// the last (newest) message and scroll resets to bottom.
    ///
    /// This is used when background-refreshed messages replace an initially
    /// cached snapshot, avoiding jarring scroll jumps.
    pub fn update_messages(&mut self, messages: Vec<Message>) {
        let previous_message_id = self
            .selected_index
            .and_then(|idx| self.messages.get(idx))
            .map(|m| m.id);

        self.messages = messages;
        self.ui_state = OpenChatUiState::Ready;
        self.refreshing = false;
        self.message_source = MessageSource::Live;

        // Try to preserve selection by message ID
        if let Some(prev_id) = previous_message_id {
            if let Some(new_idx) = self.messages.iter().position(|m| m.id == prev_id) {
                self.selected_index = Some(new_idx);
                // Keep current scroll_offset — the UI will adjust
                return;
            }
        }

        // Fallback: select last message and scroll to bottom
        self.selected_index = if self.messages.is_empty() {
            None
        } else {
            Some(self.messages.len() - 1)
        };
        self.scroll_offset = if self.messages.is_empty() {
            ScrollOffset::ZERO
        } else {
            ScrollOffset::BOTTOM
        };
    }

    /// Adds a pending (optimistically displayed) message at the end of the list.
    ///
    /// The message is shown immediately with `MessageStatus::Sending`.
    /// It will be replaced by real messages when `set_ready()` is called
    /// after the server confirms delivery.
    pub fn add_pending_message(&mut self, text: String, media: super::message::MessageMedia) {
        let now_ms = chrono::Local::now().timestamp_millis();
        let pending = Message {
            id: 0, // Temporary ID — will be replaced by server messages
            sender_name: String::new(),
            text,
            timestamp_ms: now_ms,
            is_outgoing: true,
            media,
            status: MessageStatus::Sending,
            file_info: None,
        };
        self.messages.push(pending);
        self.selected_index = Some(self.messages.len() - 1);
        self.scroll_offset = ScrollOffset::BOTTOM;
    }

    /// Removes all pending (sending) messages from the list.
    ///
    /// Used when a send fails to roll back the optimistic display.
    pub fn remove_pending_messages(&mut self) {
        self.messages.retain(|m| m.status != MessageStatus::Sending);
        // Fix selection after removal
        if self.messages.is_empty() {
            self.selected_index = None;
        } else if let Some(idx) = self.selected_index {
            if idx >= self.messages.len() {
                self.selected_index = Some(self.messages.len() - 1);
            }
        }
    }

    /// Removes a message by ID from the current list (optimistic deletion).
    ///
    /// Adjusts `selected_index` so the cursor stays on a valid message.
    pub fn remove_message(&mut self, message_id: i64) {
        let Some(pos) = self.messages.iter().position(|m| m.id == message_id) else {
            return;
        };
        self.messages.remove(pos);

        if self.messages.is_empty() {
            self.selected_index = None;
        } else if let Some(idx) = self.selected_index {
            if idx >= self.messages.len() {
                self.selected_index = Some(self.messages.len() - 1);
            }
        }
    }

    pub fn set_error(&mut self) {
        self.ui_state = OpenChatUiState::Error;
        self.refreshing = false;
        self.message_source = MessageSource::None;
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.chat_id = None;
        self.chat_title.clear();
        self.chat_subtitle = ChatSubtitle::None;
        self.messages.clear();
        self.ui_state = OpenChatUiState::Empty;
        self.selected_index = None;
        self.scroll_offset = ScrollOffset::ZERO;
        self.refreshing = false;
        self.message_source = MessageSource::None;
    }

    pub fn is_open(&self) -> bool {
        self.chat_id.is_some()
    }

    /// Returns the currently selected message, if any.
    pub fn selected_message(&self) -> Option<&Message> {
        self.selected_index.and_then(|idx| self.messages.get(idx))
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
            status: crate::domain::message::MessageStatus::Delivered,
            file_info: None,
        }
    }

    #[test]
    fn default_state_is_empty() {
        let state = OpenChatState::default();

        assert_eq!(state.ui_state(), OpenChatUiState::Empty);
        assert!(!state.is_open());
        assert!(state.messages().is_empty());
        assert_eq!(state.selected_index(), None);
        assert!(!state.is_refreshing());
        assert_eq!(state.message_source(), MessageSource::None);
    }

    #[test]
    fn set_loading_transitions_correctly() {
        let mut state = OpenChatState::default();

        state.set_loading(42, "Test Chat".to_owned());

        assert_eq!(state.chat_id(), Some(42));
        assert_eq!(state.chat_title(), "Test Chat");
        assert_eq!(state.ui_state(), OpenChatUiState::Loading);
        assert_eq!(state.selected_index(), None);
        assert!(!state.is_refreshing());
        assert_eq!(state.message_source(), MessageSource::None);
    }

    #[test]
    fn set_loading_resets_refreshing_and_source() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());
        state.set_ready(vec![message(1, "A")]);
        state.set_refreshing(true);
        state.set_message_source(MessageSource::Cache);

        state.set_loading(2, "Other Chat".to_owned());

        assert!(!state.is_refreshing());
        assert_eq!(state.message_source(), MessageSource::None);
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
        assert_eq!(state.scroll_offset(), ScrollOffset::ZERO);
    }

    #[test]
    fn set_error_transitions_to_error() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());

        state.set_error();

        assert_eq!(state.ui_state(), OpenChatUiState::Error);
    }

    #[test]
    fn set_error_resets_refreshing_and_source() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());
        state.set_ready(vec![message(1, "A")]);
        state.set_refreshing(true);
        state.set_message_source(MessageSource::Cache);

        state.set_error();

        assert!(!state.is_refreshing());
        assert_eq!(state.message_source(), MessageSource::None);
    }

    #[test]
    fn clear_resets_to_empty() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());
        state.set_ready(vec![message(1, "Hi")]);
        state.set_refreshing(true);
        state.set_message_source(MessageSource::Cache);

        state.clear();

        assert_eq!(state.ui_state(), OpenChatUiState::Empty);
        assert!(!state.is_open());
        assert!(state.messages().is_empty());
        assert_eq!(state.selected_index(), None);
        assert!(!state.is_refreshing());
        assert_eq!(state.message_source(), MessageSource::None);
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
        assert_eq!(state.scroll_offset(), ScrollOffset::ZERO);
    }

    #[test]
    fn scroll_offset_resets_on_set_loading() {
        let mut state = OpenChatState::default();
        state.scroll_offset = ScrollOffset { item: 5, line: 2 };

        state.set_loading(1, "Chat".to_owned());

        assert_eq!(state.scroll_offset(), ScrollOffset::ZERO);
    }

    #[test]
    fn set_ready_initializes_scroll_offset_to_bottom() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());

        state.set_ready(vec![message(1, "A"), message(2, "B")]);

        assert_eq!(state.scroll_offset(), ScrollOffset::BOTTOM);
    }

    #[test]
    fn set_scroll_offset_persists_value() {
        let mut state = OpenChatState::default();
        let offset = ScrollOffset { item: 3, line: 1 };

        state.set_scroll_offset(offset);

        assert_eq!(state.scroll_offset(), offset);
    }

    #[test]
    fn scroll_margin_constant_is_five() {
        assert_eq!(SCROLL_MARGIN, 5);
    }

    // ── update_messages tests ──

    #[test]
    fn update_messages_clears_refreshing_and_sets_live_source() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());
        state.set_ready(vec![message(1, "A")]);
        state.set_refreshing(true);
        state.set_message_source(MessageSource::Cache);

        state.update_messages(vec![message(1, "A"), message(2, "B")]);

        assert!(!state.is_refreshing());
        assert_eq!(state.message_source(), MessageSource::Live);
    }

    #[test]
    fn update_messages_preserves_selection_by_message_id() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());
        state.set_ready(vec![message(1, "A"), message(2, "B"), message(3, "C")]);

        // Select message 2 (index 1)
        state.selected_index = Some(1);
        let saved_offset = ScrollOffset { item: 2, line: 3 };
        state.set_scroll_offset(saved_offset);

        // Update with reordered messages — message 2 is now at index 2
        state.update_messages(vec![message(4, "D"), message(1, "A"), message(2, "B")]);

        assert_eq!(state.selected_index(), Some(2));
        assert_eq!(state.ui_state(), OpenChatUiState::Ready);
        // Scroll offset preserved (not reset)
        assert_eq!(state.scroll_offset(), saved_offset);
    }

    #[test]
    fn update_messages_falls_back_to_last_when_selected_message_disappears() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());
        state.set_ready(vec![message(1, "A"), message(2, "B"), message(3, "C")]);

        // Select message 3 (index 2)
        assert_eq!(state.selected_index(), Some(2));

        // Update without message 3
        state.update_messages(vec![message(1, "A"), message(2, "B")]);

        // Should fall back to last message (index 1)
        assert_eq!(state.selected_index(), Some(1));
        assert_eq!(state.scroll_offset(), ScrollOffset::BOTTOM);
    }

    #[test]
    fn update_messages_handles_empty_replacement() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());
        state.set_ready(vec![message(1, "A"), message(2, "B")]);

        state.update_messages(vec![]);

        assert_eq!(state.selected_index(), None);
        assert_eq!(state.scroll_offset(), ScrollOffset::ZERO);
        assert_eq!(state.ui_state(), OpenChatUiState::Ready);
    }

    #[test]
    fn update_messages_on_empty_state_with_new_messages() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());
        state.set_ready(vec![]);

        state.update_messages(vec![message(1, "A"), message(2, "B")]);

        // No previous selection, so falls back to last message
        assert_eq!(state.selected_index(), Some(1));
        assert_eq!(state.scroll_offset(), ScrollOffset::BOTTOM);
        assert_eq!(state.ui_state(), OpenChatUiState::Ready);
    }

    #[test]
    fn update_messages_preserves_selection_same_position() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());
        state.set_ready(vec![message(1, "A"), message(2, "B")]);

        // Selected last message (index 1, id 2)
        assert_eq!(state.selected_index(), Some(1));
        let saved_offset = ScrollOffset { item: 1, line: 0 };
        state.set_scroll_offset(saved_offset);

        // Update with same messages + one new one at the end
        state.update_messages(vec![message(1, "A"), message(2, "B"), message(3, "C")]);

        // Message 2 is still at index 1
        assert_eq!(state.selected_index(), Some(1));
        // Scroll offset preserved
        assert_eq!(state.scroll_offset(), saved_offset);
    }

    // ── pending message tests ──

    #[test]
    fn add_pending_message_appends_and_selects_it() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());
        state.set_ready(vec![message(1, "A"), message(2, "B")]);

        state.add_pending_message(
            "Hello".to_owned(),
            crate::domain::message::MessageMedia::None,
        );

        assert_eq!(state.messages().len(), 3);
        let pending = &state.messages()[2];
        assert_eq!(pending.text, "Hello");
        assert!(pending.is_outgoing);
        assert_eq!(
            pending.status,
            crate::domain::message::MessageStatus::Sending
        );
        assert_eq!(pending.media, crate::domain::message::MessageMedia::None);
        assert_eq!(state.selected_index(), Some(2));
        assert_eq!(state.scroll_offset(), ScrollOffset::BOTTOM);
    }

    #[test]
    fn remove_pending_messages_keeps_delivered() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());
        state.set_ready(vec![message(1, "A"), message(2, "B")]);

        state.add_pending_message(
            "Pending".to_owned(),
            crate::domain::message::MessageMedia::None,
        );
        assert_eq!(state.messages().len(), 3);

        state.remove_pending_messages();

        assert_eq!(state.messages().len(), 2);
        assert_eq!(state.messages()[0].text, "A");
        assert_eq!(state.messages()[1].text, "B");
    }

    #[test]
    fn remove_pending_messages_fixes_selection() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());
        state.set_ready(vec![message(1, "A")]);

        state.add_pending_message(
            "Pending".to_owned(),
            crate::domain::message::MessageMedia::None,
        );
        assert_eq!(state.selected_index(), Some(1)); // pending message selected

        state.remove_pending_messages();

        // Selection should clamp to last remaining message
        assert_eq!(state.selected_index(), Some(0));
    }

    #[test]
    fn set_ready_replaces_pending_messages() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());
        state.set_ready(vec![message(1, "A")]);

        state.add_pending_message(
            "Pending".to_owned(),
            crate::domain::message::MessageMedia::None,
        );
        assert_eq!(state.messages().len(), 2);

        // Server refresh replaces everything including pending
        state.set_ready(vec![message(1, "A"), message(3, "Pending delivered")]);

        assert_eq!(state.messages().len(), 2);
        assert_eq!(state.messages()[1].text, "Pending delivered");
        assert_eq!(
            state.messages()[1].status,
            crate::domain::message::MessageStatus::Delivered
        );
    }

    #[test]
    fn add_pending_message_with_voice_media() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());
        state.set_ready(vec![message(1, "A")]);

        state.add_pending_message(String::new(), crate::domain::message::MessageMedia::Voice);

        assert_eq!(state.messages().len(), 2);
        let pending = &state.messages()[1];
        assert_eq!(pending.text, "");
        assert_eq!(pending.media, crate::domain::message::MessageMedia::Voice);
        assert_eq!(
            pending.status,
            crate::domain::message::MessageStatus::Sending
        );
        assert!(pending.is_outgoing);
        assert_eq!(pending.id, 0);
        assert_eq!(state.selected_index(), Some(1));
        assert_eq!(state.scroll_offset(), ScrollOffset::BOTTOM);
    }

    // ── selected_message tests ──

    #[test]
    fn selected_message_returns_message_at_selected_index() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());
        state.set_ready(vec![
            message(1, "First"),
            message(2, "Second"),
            message(3, "Third"),
        ]);

        // Initially selected = last (index 2)
        let msg = state.selected_message().unwrap();
        assert_eq!(msg.id, 3);
        assert_eq!(msg.text, "Third");
    }

    #[test]
    fn selected_message_returns_none_when_empty() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());
        state.set_ready(vec![]);

        assert!(state.selected_message().is_none());
    }

    #[test]
    fn selected_message_follows_navigation() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());
        state.set_ready(vec![message(1, "A"), message(2, "B")]);

        state.select_previous();
        let msg = state.selected_message().unwrap();
        assert_eq!(msg.id, 1);
        assert_eq!(msg.text, "A");
    }

    // ── remove_message tests ──

    #[test]
    fn remove_message_removes_by_id() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());
        state.set_ready(vec![message(1, "A"), message(2, "B"), message(3, "C")]);

        state.remove_message(2);

        assert_eq!(state.messages().len(), 2);
        assert_eq!(state.messages()[0].id, 1);
        assert_eq!(state.messages()[1].id, 3);
    }

    #[test]
    fn remove_message_adjusts_selection_when_last_removed() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());
        state.set_ready(vec![message(1, "A"), message(2, "B")]);
        // Selection defaults to last (index 1, id=2)

        state.remove_message(2);

        // Selection should clamp to new last (index 0)
        assert_eq!(state.selected_message().unwrap().id, 1);
    }

    #[test]
    fn remove_message_clears_selection_when_empty() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());
        state.set_ready(vec![message(1, "A")]);

        state.remove_message(1);

        assert!(state.messages().is_empty());
        assert!(state.selected_message().is_none());
    }

    #[test]
    fn remove_message_ignores_unknown_id() {
        let mut state = OpenChatState::default();
        state.set_loading(1, "Chat".to_owned());
        state.set_ready(vec![message(1, "A"), message(2, "B")]);

        state.remove_message(999);

        assert_eq!(state.messages().len(), 2);
    }
}
