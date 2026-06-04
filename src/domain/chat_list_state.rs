use super::chat::ChatSummary;
use super::selectable_list::SelectableList;

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatListUiState {
    Loading,
    Ready,
    Empty,
    Error,
}

use crate::usecases::list_chats::DEFAULT_CHAT_PAGE_SIZE;

const LOAD_MORE_OFFSET: usize = 10;

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatListState {
    ui_state: ChatListUiState,
    list: SelectableList<ChatSummary>,
    all_chats_loaded: bool,
    total_limit: usize,
}

impl Default for ChatListState {
    fn default() -> Self {
        Self {
            ui_state: ChatListUiState::Loading,
            list: SelectableList::default(),
            all_chats_loaded: false,
            total_limit: DEFAULT_CHAT_PAGE_SIZE,
        }
    }
}

impl ChatListState {
    /// Creates a pre-populated state from cached data.
    ///
    /// If `chats` is non-empty, the state starts as `Ready` with the first
    /// chat selected. If `chats` is empty, falls back to `Loading` (same as
    /// [`Default`]).
    pub fn with_initial_chats(chats: Vec<ChatSummary>) -> Self {
        if chats.is_empty() {
            return Self::default();
        }

        let total_limit = chats.len().max(DEFAULT_CHAT_PAGE_SIZE);
        let mut list = SelectableList::default();
        list.replace(chats, None);
        Self {
            ui_state: ChatListUiState::Ready,
            all_chats_loaded: false,
            total_limit,
            list,
        }
    }
}

#[cfg_attr(not(test), allow(dead_code))]
impl ChatListState {
    pub fn ui_state(&self) -> ChatListUiState {
        self.ui_state.clone()
    }

    pub fn chats(&self) -> &[ChatSummary] {
        self.list.items()
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.list.selected_index()
    }

    pub fn selected_chat(&self) -> Option<&ChatSummary> {
        self.list.selected()
    }

    pub fn all_chats_loaded(&self) -> bool {
        self.all_chats_loaded
    }

    pub fn total_limit(&self) -> usize {
        self.total_limit
    }

    pub fn needs_more_chats(&self) -> bool {
        if self.all_chats_loaded {
            return false;
        }
        let Some(index) = self.list.selected_index() else {
            return false;
        };
        let last = self.list.items().len().saturating_sub(1);
        last.saturating_sub(index) < LOAD_MORE_OFFSET
    }

    pub fn request_more_chats(&mut self) {
        self.total_limit += DEFAULT_CHAT_PAGE_SIZE;
    }

    pub fn set_all_chats_loaded(&mut self, all_loaded: bool) {
        self.all_chats_loaded = all_loaded;
    }

    pub fn set_loading(&mut self) {
        self.ui_state = ChatListUiState::Loading;
        self.list.clear();
        self.all_chats_loaded = false;
        self.total_limit = DEFAULT_CHAT_PAGE_SIZE;
    }

    pub fn set_ready(&mut self, chats: Vec<ChatSummary>) {
        let previous_selected_chat_id = self.selected_chat().map(|chat| chat.chat_id);
        self.set_ready_with_selection_hint(chats, previous_selected_chat_id);
    }

    fn set_ready_with_selection_hint(
        &mut self,
        chats: Vec<ChatSummary>,
        preferred_chat_id: Option<i64>,
    ) {
        if chats.is_empty() {
            self.set_empty();
            return;
        }

        let preferred_index = preferred_chat_id
            .and_then(|chat_id| chats.iter().position(|chat| chat.chat_id == chat_id));
        self.list.replace(chats, preferred_index);
        self.ui_state = ChatListUiState::Ready;
    }

    pub fn set_empty(&mut self) {
        self.ui_state = ChatListUiState::Empty;
        self.list.clear();
        self.all_chats_loaded = true;
    }

    pub fn set_error(&mut self) {
        self.ui_state = ChatListUiState::Error;
        self.list.clear();
        self.all_chats_loaded = false;
    }

    pub fn select_next(&mut self) {
        self.list.select_next();
    }

    pub fn select_first(&mut self) {
        self.list.select_first();
    }

    pub fn select_previous(&mut self) {
        self.list.select_previous();
    }

    pub fn select_by_query(&mut self, query: &str) -> bool {
        let chats = self.list.items();
        if chats.is_empty() || query.is_empty() {
            return false;
        }
        let query_lower = query.to_lowercase();
        let start = self.list.selected_index().unwrap_or(0);
        let len = chats.len();
        for offset in 0..len {
            let idx = (start + offset) % len;
            if chats[idx].title.to_lowercase().contains(&query_lower) {
                self.list.set_selected_index(Some(idx));
                return true;
            }
        }
        false
    }

    pub fn clear_selected_chat_unread(&mut self) {
        if let Some(chat) = self.list.selected_mut() {
            chat.unread_count = 0;
            if chat.is_forum {
                chat.unread_topic_count = Some(0);
            }
        }
    }

    /// Sets a forum chat's unread-topic count directly.
    ///
    /// Used to reconcile the root-list badge from a topic list already held in
    /// memory (e.g. when leaving a forum) without a `getForumTopics` round-trip.
    /// No-op for non-forum chats and unknown ids.
    pub fn set_forum_unread_topic_count(&mut self, chat_id: i64, count: u32) {
        if let Some(chat) = self
            .list
            .items_mut()
            .iter_mut()
            .find(|chat| chat.chat_id == chat_id)
        {
            if chat.is_forum {
                chat.unread_topic_count = Some(count);
            }
        }
    }

    /// Optimistically decrements a forum chat's unread-topic count by one.
    ///
    /// Called when the user opens a forum topic that had unread messages: that
    /// topic is now read, so one fewer topic is unread. The forum badge shows
    /// the unread-topic count (not the message count), and TDLib does not push a
    /// chat-level read for forums, so reflecting it here keeps the badge in step
    /// until the next chat-list refresh recomputes the authoritative value from
    /// `getForumTopics`. No-op when the count is unknown (`None`).
    pub fn mark_forum_topic_read(&mut self, chat_id: i64) {
        if let Some(chat) = self
            .list
            .items_mut()
            .iter_mut()
            .find(|chat| chat.chat_id == chat_id)
        {
            if let Some(count) = chat.unread_topic_count.as_mut() {
                *count = count.saturating_sub(1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::chat::{ChatType, OutgoingReadStatus};

    fn chat(chat_id: i64, title: &str) -> ChatSummary {
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
            is_bot: false,
            outgoing_status: OutgoingReadStatus::default(),
            last_message_id: None,
            unread_reaction_count: 0,
            is_forum: false,
            unread_topic_count: None,
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

    #[test]
    fn with_initial_chats_starts_ready_when_data_present() {
        let state = ChatListState::with_initial_chats(vec![chat(1, "Alpha"), chat(2, "Beta")]);

        assert_eq!(state.ui_state(), ChatListUiState::Ready);
        assert_eq!(state.chats().len(), 2);
        assert_eq!(state.selected_index(), Some(0));
        assert_eq!(state.selected_chat().map(|c| c.chat_id), Some(1));
    }

    #[test]
    fn with_initial_chats_falls_back_to_loading_when_empty() {
        let state = ChatListState::with_initial_chats(vec![]);

        assert_eq!(state.ui_state(), ChatListUiState::Loading);
        assert!(state.chats().is_empty());
        assert_eq!(state.selected_index(), None);
    }

    #[test]
    fn set_ready_with_selection_hint_preserves_selection_across_reload() {
        let mut state = ChatListState::default();
        state.set_ready(vec![chat(1, "General"), chat(2, "Backend")]);
        state.select_next();

        state.set_loading();
        state.set_ready_with_selection_hint(
            vec![chat(10, "Infra"), chat(2, "Backend"), chat(11, "Design")],
            Some(2),
        );

        assert_eq!(state.selected_index(), Some(1));
        assert_eq!(state.selected_chat().map(|item| item.chat_id), Some(2));
    }

    fn chat_with_unread(chat_id: i64, title: &str, unread_count: u32) -> ChatSummary {
        let mut c = chat(chat_id, title);
        c.unread_count = unread_count;
        c
    }

    #[test]
    fn select_first_moves_to_index_zero() {
        let mut state = ChatListState::default();
        state.set_ready(vec![chat(1, "Alpha"), chat(2, "Beta"), chat(3, "Gamma")]);
        state.select_next();
        state.select_next();
        assert_eq!(state.selected_index(), Some(2));

        state.select_first();
        assert_eq!(state.selected_index(), Some(0));
        assert_eq!(state.selected_chat().map(|c| c.chat_id), Some(1));
    }

    #[test]
    fn select_first_noop_on_empty_list() {
        let mut state = ChatListState::default();
        state.select_first();
        assert_eq!(state.selected_index(), None);
    }

    #[test]
    fn clear_selected_chat_unread_zeroes_counter() {
        let mut state = ChatListState::default();
        state.set_ready(vec![
            chat_with_unread(1, "General", 3),
            chat_with_unread(2, "Backend", 7),
        ]);
        state.select_next(); // select chat 2

        state.clear_selected_chat_unread();

        assert_eq!(state.selected_chat().unwrap().unread_count, 0);
        // chat 1 remains unchanged
        assert_eq!(state.chats()[0].unread_count, 3);
    }

    #[test]
    fn clear_selected_chat_unread_noop_without_selection() {
        let mut state = ChatListState::default();
        state.clear_selected_chat_unread(); // should not panic
    }

    fn forum_chat_with_topics(chat_id: i64, title: &str, unread_topic_count: u32) -> ChatSummary {
        let mut c = chat(chat_id, title);
        c.is_forum = true;
        c.unread_topic_count = Some(unread_topic_count);
        c
    }

    #[test]
    fn mark_forum_topic_read_decrements_by_one_and_saturates() {
        let mut state = ChatListState::default();
        state.set_ready(vec![
            forum_chat_with_topics(1, "Forum", 2),
            forum_chat_with_topics(2, "Other", 3),
        ]);

        state.mark_forum_topic_read(1);
        assert_eq!(state.chats()[0].unread_topic_count, Some(1));
        // unrelated forum is untouched
        assert_eq!(state.chats()[1].unread_topic_count, Some(3));

        // decrementing past zero saturates rather than wrapping
        state.mark_forum_topic_read(1);
        state.mark_forum_topic_read(1);
        assert_eq!(state.chats()[0].unread_topic_count, Some(0));
    }

    #[test]
    fn mark_forum_topic_read_noop_when_count_unknown() {
        let mut state = ChatListState::default();
        state.set_ready(vec![chat_with_unread(1, "Plain", 5)]);
        state.mark_forum_topic_read(1); // no topic count to decrement
        assert_eq!(state.chats()[0].unread_topic_count, None);
        assert_eq!(state.chats()[0].unread_count, 5);
    }

    #[test]
    fn mark_forum_topic_read_noop_for_unknown_chat() {
        let mut state = ChatListState::default();
        state.set_ready(vec![forum_chat_with_topics(1, "Forum", 2)]);
        state.mark_forum_topic_read(999); // should not panic or change anything
        assert_eq!(state.chats()[0].unread_topic_count, Some(2));
    }

    #[test]
    fn clear_selected_chat_unread_zeroes_forum_topic_count() {
        let mut state = ChatListState::default();
        state.set_ready(vec![forum_chat_with_topics(1, "Forum", 4)]);
        state.clear_selected_chat_unread();
        assert_eq!(state.chats()[0].unread_topic_count, Some(0));
    }

    #[test]
    fn select_by_query_finds_match_forward() {
        let mut state = ChatListState::default();
        state.set_ready(vec![chat(1, "Alice"), chat(2, "Bob"), chat(3, "Charlie")]);
        assert!(state.select_by_query("bob"));
        assert_eq!(state.selected_index(), Some(1));
    }

    #[test]
    fn select_by_query_case_insensitive() {
        let mut state = ChatListState::default();
        state.set_ready(vec![chat(1, "Alice"), chat(2, "Bob")]);
        assert!(state.select_by_query("BOB"));
        assert_eq!(state.selected_index(), Some(1));
    }

    #[test]
    fn select_by_query_wraps_around() {
        let mut state = ChatListState::default();
        state.set_ready(vec![chat(1, "Alice"), chat(2, "Bob"), chat(3, "Charlie")]);
        state.select_next();
        state.select_next(); // now at Charlie (index 2)
        assert!(state.select_by_query("alice"));
        assert_eq!(state.selected_index(), Some(0));
    }

    #[test]
    fn select_by_query_no_match_returns_false() {
        let mut state = ChatListState::default();
        state.set_ready(vec![chat(1, "Alice"), chat(2, "Bob")]);
        assert!(!state.select_by_query("xyz"));
        assert_eq!(state.selected_index(), Some(0));
    }

    #[test]
    fn select_by_query_empty_query_returns_false() {
        let mut state = ChatListState::default();
        state.set_ready(vec![chat(1, "Alice")]);
        assert!(!state.select_by_query(""));
    }

    #[test]
    fn select_by_query_empty_list_returns_false() {
        let mut state = ChatListState::default();
        assert!(!state.select_by_query("alice"));
    }

    #[test]
    fn select_by_query_substring_match() {
        let mut state = ChatListState::default();
        state.set_ready(vec![chat(1, "Alice Johnson"), chat(2, "Bob Smith")]);
        assert!(state.select_by_query("john"));
        assert_eq!(state.selected_index(), Some(0));
    }

    #[test]
    fn needs_more_chats_true_when_near_end() {
        let chats: Vec<ChatSummary> = (1..=20).map(|i| chat(i, &format!("Chat {i}"))).collect();
        let mut state = ChatListState::default();
        state.set_ready(chats);

        for _ in 0..15 {
            state.select_next();
        }

        assert!(state.needs_more_chats());
    }

    #[test]
    fn needs_more_chats_false_when_far_from_end() {
        let chats: Vec<ChatSummary> = (1..=50).map(|i| chat(i, &format!("Chat {i}"))).collect();
        let mut state = ChatListState::default();
        state.set_ready(chats);

        state.select_next();

        assert!(!state.needs_more_chats());
    }

    #[test]
    fn needs_more_chats_false_when_all_loaded() {
        let chats: Vec<ChatSummary> = (1..=5).map(|i| chat(i, &format!("Chat {i}"))).collect();
        let mut state = ChatListState::default();
        state.set_all_chats_loaded(true);
        state.set_ready(chats);

        for _ in 0..4 {
            state.select_next();
        }

        assert!(!state.needs_more_chats());
    }

    #[test]
    fn request_more_chats_increases_total_limit() {
        let mut state = ChatListState::default();
        assert_eq!(state.total_limit(), 50);

        state.request_more_chats();
        assert_eq!(state.total_limit(), 100);

        state.request_more_chats();
        assert_eq!(state.total_limit(), 150);
    }

    #[test]
    fn set_loading_resets_pagination_state() {
        let mut state = ChatListState::default();
        state.set_all_chats_loaded(true);
        state.request_more_chats();

        state.set_loading();

        assert!(!state.all_chats_loaded());
        assert_eq!(state.total_limit(), 50);
    }

    #[test]
    fn set_empty_marks_all_loaded() {
        let mut state = ChatListState::default();
        state.set_empty();
        assert!(state.all_chats_loaded());
    }
}
