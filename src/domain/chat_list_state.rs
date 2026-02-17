use super::chat::ChatSummary;

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatListUiState {
    Loading,
    Ready,
    Empty,
    Error,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatListState {
    ui_state: ChatListUiState,
    chats: Vec<ChatSummary>,
    selected_index: Option<usize>,
}

impl Default for ChatListState {
    fn default() -> Self {
        Self {
            ui_state: ChatListUiState::Loading,
            chats: Vec::new(),
            selected_index: None,
        }
    }
}

#[cfg_attr(not(test), allow(dead_code))]
impl ChatListState {
    pub fn ui_state(&self) -> ChatListUiState {
        self.ui_state.clone()
    }

    pub fn chats(&self) -> &[ChatSummary] {
        &self.chats
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    pub fn selected_chat(&self) -> Option<&ChatSummary> {
        self.selected_index.and_then(|index| self.chats.get(index))
    }

    pub fn set_loading(&mut self) {
        self.ui_state = ChatListUiState::Loading;
        self.chats.clear();
        self.selected_index = None;
    }

    pub fn set_ready(&mut self, chats: Vec<ChatSummary>) {
        if chats.is_empty() {
            self.set_empty();
            return;
        }

        let previous_selected_chat_id = self.selected_chat().map(|chat| chat.chat_id);
        self.ui_state = ChatListUiState::Ready;
        self.chats = chats;
        self.selected_index = resolve_selection_index(&self.chats, previous_selected_chat_id);
    }

    pub fn set_empty(&mut self) {
        self.ui_state = ChatListUiState::Empty;
        self.chats.clear();
        self.selected_index = None;
    }

    pub fn set_error(&mut self) {
        self.ui_state = ChatListUiState::Error;
        self.chats.clear();
        self.selected_index = None;
    }

    pub fn select_next(&mut self) {
        let Some(index) = self.selected_index else {
            return;
        };

        let last_index = self.chats.len().saturating_sub(1);
        self.selected_index = Some(std::cmp::min(index.saturating_add(1), last_index));
    }

    pub fn select_previous(&mut self) {
        let Some(index) = self.selected_index else {
            return;
        };

        self.selected_index = Some(index.saturating_sub(1));
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn resolve_selection_index(
    chats: &[ChatSummary],
    previous_selected_chat_id: Option<i64>,
) -> Option<usize> {
    if chats.is_empty() {
        return None;
    }

    previous_selected_chat_id
        .and_then(|chat_id| chats.iter().position(|chat| chat.chat_id == chat_id))
        .or(Some(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chat(chat_id: i64, title: &str) -> ChatSummary {
        ChatSummary {
            chat_id,
            title: title.to_owned(),
            unread_count: 0,
            last_message_preview: None,
            last_message_unix_ms: None,
        }
    }

    #[test]
    fn default_state_is_loading_without_selection() {
        let state = ChatListState::default();

        assert_eq!(state.ui_state(), ChatListUiState::Loading);
        assert!(state.chats().is_empty());
        assert_eq!(state.selected_index(), None);
    }

    #[test]
    fn set_ready_with_data_sets_ready_and_selects_first_item() {
        let mut state = ChatListState::default();

        state.set_ready(vec![chat(1, "General"), chat(2, "Backend")]);

        assert_eq!(state.ui_state(), ChatListUiState::Ready);
        assert_eq!(state.selected_index(), Some(0));
        assert_eq!(state.selected_chat().map(|item| item.chat_id), Some(1));
    }

    #[test]
    fn set_ready_with_empty_list_transitions_to_empty_state() {
        let mut state = ChatListState::default();

        state.set_ready(vec![]);

        assert_eq!(state.ui_state(), ChatListUiState::Empty);
        assert!(state.chats().is_empty());
        assert_eq!(state.selected_index(), None);
    }

    #[test]
    fn set_error_clears_items_and_selection() {
        let mut state = ChatListState::default();
        state.set_ready(vec![chat(1, "General")]);

        state.set_error();

        assert_eq!(state.ui_state(), ChatListUiState::Error);
        assert!(state.chats().is_empty());
        assert_eq!(state.selected_index(), None);
    }

    #[test]
    fn set_loading_clears_items_and_selection() {
        let mut state = ChatListState::default();
        state.set_ready(vec![chat(1, "General")]);

        state.set_loading();

        assert_eq!(state.ui_state(), ChatListUiState::Loading);
        assert!(state.chats().is_empty());
        assert_eq!(state.selected_index(), None);
    }

    #[test]
    fn selection_moves_within_bounds() {
        let mut state = ChatListState::default();
        state.set_ready(vec![chat(1, "General"), chat(2, "Backend")]);

        state.select_next();
        state.select_next();
        state.select_previous();

        assert_eq!(state.selected_index(), Some(0));
        assert_eq!(state.selected_chat().map(|item| item.chat_id), Some(1));
    }

    #[test]
    fn set_ready_preserves_selection_by_chat_id() {
        let mut state = ChatListState::default();
        state.set_ready(vec![chat(1, "General"), chat(2, "Backend"), chat(3, "Ops")]);
        state.select_next();

        state.set_ready(vec![
            chat(8, "Infra"),
            chat(2, "Backend"),
            chat(9, "Design"),
        ]);

        assert_eq!(state.selected_chat().map(|item| item.chat_id), Some(2));
        assert_eq!(state.selected_index(), Some(1));
    }

    #[test]
    fn set_ready_falls_back_to_first_when_previous_selection_disappears() {
        let mut state = ChatListState::default();
        state.set_ready(vec![chat(1, "General"), chat(2, "Backend")]);
        state.select_next();

        state.set_ready(vec![chat(10, "Infra"), chat(11, "Design")]);

        assert_eq!(state.selected_index(), Some(0));
        assert_eq!(state.selected_chat().map(|item| item.chat_id), Some(10));
    }
}
