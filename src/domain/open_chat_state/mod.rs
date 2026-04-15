use super::chat::ChatType;
use super::chat_subtitle::ChatSubtitle;
use super::message::{Message, MessageStatus, ReplyInfo};
use super::typing_state::TypingState;

#[cfg(test)]
mod tests;

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
    chat_type: ChatType,
    messages: Vec<Message>,
    ui_state: OpenChatUiState,
    selected_index: Option<usize>,
    scroll_offset: ScrollOffset,
    /// Whether a background refresh is in-flight while cached messages are shown.
    refreshing: bool,
    /// How the currently displayed messages were obtained.
    message_source: MessageSource,
    typing_state: TypingState,
}

impl Default for OpenChatState {
    fn default() -> Self {
        Self {
            chat_id: None,
            chat_title: String::new(),
            chat_subtitle: ChatSubtitle::None,
            chat_type: ChatType::Private,
            messages: Vec::new(),
            ui_state: OpenChatUiState::Empty,
            selected_index: None,
            scroll_offset: ScrollOffset::ZERO,
            refreshing: false,
            message_source: MessageSource::None,
            typing_state: TypingState::default(),
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

    pub fn typing_state(&self) -> &TypingState {
        &self.typing_state
    }

    pub fn typing_state_mut(&mut self) -> &mut TypingState {
        &mut self.typing_state
    }

    pub fn chat_type(&self) -> ChatType {
        self.chat_type
    }

    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Updates the `FileInfo` of a specific message by ID.
    ///
    /// If the message is found and has `file_info`, the closure is called
    /// to mutate it in-place (e.g., to update download status/progress).
    pub fn update_message_file_info(
        &mut self,
        message_id: i64,
        updater: impl FnOnce(&mut super::message::FileInfo),
    ) {
        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == message_id) {
            if let Some(ref mut fi) = msg.file_info {
                updater(fi);
            }
        }
    }

    pub fn update_message_text(&mut self, message_id: i64, new_text: String) {
        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == message_id) {
            msg.text = new_text;
            msg.is_edited = true;
        }
    }

    /// Updates the `reaction_count` of a specific message by ID.
    pub fn update_message_reaction_count(&mut self, message_id: i64, reaction_count: u32) {
        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == message_id) {
            msg.reaction_count = reaction_count;
        }
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

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn message_source(&self) -> MessageSource {
        self.message_source
    }

    pub fn set_message_source(&mut self, source: MessageSource) {
        self.message_source = source;
    }

    pub fn set_loading(&mut self, chat_id: i64, chat_title: String, chat_type: ChatType) {
        self.chat_id = Some(chat_id);
        self.chat_title = chat_title;
        self.chat_subtitle = ChatSubtitle::None;
        self.chat_type = chat_type;
        self.messages.clear();
        self.ui_state = OpenChatUiState::Loading;
        self.selected_index = None;
        self.scroll_offset = ScrollOffset::ZERO;
        self.refreshing = false;
        self.message_source = MessageSource::None;
        self.typing_state.clear();
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
    pub fn add_pending_message(
        &mut self,
        text: String,
        media: super::message::MessageMedia,
        reply_to: Option<ReplyInfo>,
    ) {
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
            call_info: None,
            reply_to,
            forward_info: None,
            reaction_count: 0,
            links: Vec::new(),
            is_edited: false,
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
        self.chat_type = ChatType::Private;
        self.messages.clear();
        self.ui_state = OpenChatUiState::Empty;
        self.selected_index = None;
        self.scroll_offset = ScrollOffset::ZERO;
        self.refreshing = false;
        self.message_source = MessageSource::None;
        self.typing_state.clear();
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
