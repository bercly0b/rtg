use super::forum_topic::ForumTopicSummary;
use super::selectable_list::SelectableList;

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForumTopicListUiState {
    Loading,
    Ready,
    Empty,
    Error,
}

/// Left-panel state when the user is browsing the topic list of a forum chat.
///
/// Sits alongside (and is independent of) the root `ChatListState`. Entering a
/// forum chat installs an instance of this state; leaving it drops the state
/// without touching the root chat list — so the user returns to the same
/// selection and scroll position they had before.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForumTopicListState {
    ui_state: ForumTopicListUiState,
    parent_chat_id: i64,
    parent_chat_title: String,
    list: SelectableList<ForumTopicSummary>,
}

#[cfg_attr(not(test), allow(dead_code))]
impl ForumTopicListState {
    /// Creates a fresh state in `Loading` for the given forum chat.
    pub fn loading(parent_chat_id: i64, parent_chat_title: String) -> Self {
        Self {
            ui_state: ForumTopicListUiState::Loading,
            parent_chat_id,
            parent_chat_title,
            list: SelectableList::default(),
        }
    }

    pub fn ui_state(&self) -> ForumTopicListUiState {
        self.ui_state.clone()
    }

    pub fn parent_chat_id(&self) -> i64 {
        self.parent_chat_id
    }

    pub fn parent_chat_title(&self) -> &str {
        &self.parent_chat_title
    }

    pub fn topics(&self) -> &[ForumTopicSummary] {
        self.list.items()
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.list.selected_index()
    }

    pub fn selected_topic(&self) -> Option<&ForumTopicSummary> {
        self.list.selected()
    }

    pub fn find_topic(&self, topic_id: i32) -> Option<&ForumTopicSummary> {
        self.list.items().iter().find(|t| t.topic_id == topic_id)
    }

    /// Replaces topics; sorts by `order` desc, like the regular chat list.
    pub fn set_ready(&mut self, topics: Vec<ForumTopicSummary>) {
        if topics.is_empty() {
            self.set_empty();
            return;
        }
        let previous_topic_id = self.selected_topic().map(|t| t.topic_id);

        let mut sorted = topics;
        sorted.sort_by_key(|t| std::cmp::Reverse(t.order));

        let preferred_index =
            previous_topic_id.and_then(|tid| sorted.iter().position(|t| t.topic_id == tid));
        self.list.replace(sorted, preferred_index);
        self.ui_state = ForumTopicListUiState::Ready;
    }

    pub fn set_empty(&mut self) {
        self.ui_state = ForumTopicListUiState::Empty;
        self.list.clear();
    }

    pub fn set_error(&mut self) {
        self.ui_state = ForumTopicListUiState::Error;
        self.list.clear();
    }

    pub fn set_loading(&mut self) {
        self.ui_state = ForumTopicListUiState::Loading;
        self.list.clear();
    }

    pub fn select_next(&mut self) {
        self.list.select_next();
    }

    pub fn select_previous(&mut self) {
        self.list.select_previous();
    }

    pub fn select_first(&mut self) {
        self.list.select_first();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn topic(topic_id: i32, name: &str, order: i64) -> ForumTopicSummary {
        ForumTopicSummary {
            chat_id: 100,
            topic_id,
            name: name.to_owned(),
            is_general: false,
            is_closed: false,
            is_hidden: false,
            is_pinned: false,
            unread_count: 0,
            last_message_preview: None,
            last_message_unix_ms: None,
            last_message_id: None,
            order,
        }
    }

    fn general_topic(topic_id: i32) -> ForumTopicSummary {
        let mut t = topic(topic_id, "General", 0);
        t.is_general = true;
        t
    }

    #[test]
    fn loading_state_has_no_topics_and_no_selection() {
        let state = ForumTopicListState::loading(100, "Forum".to_owned());

        assert_eq!(state.ui_state(), ForumTopicListUiState::Loading);
        assert!(state.topics().is_empty());
        assert_eq!(state.selected_index(), None);
        assert_eq!(state.parent_chat_id(), 100);
        assert_eq!(state.parent_chat_title(), "Forum");
    }

    #[test]
    fn set_ready_with_data_transitions_to_ready_and_selects_first() {
        let mut state = ForumTopicListState::loading(100, "Forum".to_owned());

        state.set_ready(vec![topic(1, "Alpha", 100), topic(2, "Beta", 50)]);

        assert_eq!(state.ui_state(), ForumTopicListUiState::Ready);
        assert_eq!(state.selected_index(), Some(0));
    }

    #[test]
    fn set_ready_with_empty_collapses_to_empty_state() {
        let mut state = ForumTopicListState::loading(100, "Forum".to_owned());

        state.set_ready(vec![]);

        assert_eq!(state.ui_state(), ForumTopicListUiState::Empty);
        assert!(state.topics().is_empty());
        assert_eq!(state.selected_index(), None);
    }

    #[test]
    fn general_topic_is_sorted_by_order_like_any_other() {
        let mut state = ForumTopicListState::loading(100, "Forum".to_owned());

        let mut general = general_topic(2);
        general.order = 500;
        state.set_ready(vec![
            topic(1, "High order", 1000),
            general,
            topic(3, "Low order", 10),
        ]);

        // General is no longer pinned first — it sorts purely by `order`.
        assert_eq!(state.topics()[0].topic_id, 1);
        assert_eq!(state.topics()[1].topic_id, 2);
        assert!(state.topics()[1].is_general);
        assert_eq!(state.topics()[2].topic_id, 3);
    }

    #[test]
    fn non_general_topics_sorted_by_order_desc() {
        let mut state = ForumTopicListState::loading(100, "Forum".to_owned());

        state.set_ready(vec![
            topic(1, "Low", 1),
            topic(2, "Mid", 50),
            topic(3, "High", 1000),
        ]);

        assert_eq!(state.topics()[0].topic_id, 3);
        assert_eq!(state.topics()[1].topic_id, 2);
        assert_eq!(state.topics()[2].topic_id, 1);
    }

    #[test]
    fn set_ready_preserves_selection_by_topic_id() {
        let mut state = ForumTopicListState::loading(100, "Forum".to_owned());
        state.set_ready(vec![topic(1, "A", 100), topic(2, "B", 50)]);
        state.select_next();
        assert_eq!(state.selected_topic().map(|t| t.topic_id), Some(2));

        state.set_ready(vec![topic(2, "B", 50), topic(3, "C", 30)]);

        assert_eq!(state.selected_topic().map(|t| t.topic_id), Some(2));
    }

    #[test]
    fn set_ready_falls_back_to_first_when_previous_selection_disappears() {
        let mut state = ForumTopicListState::loading(100, "Forum".to_owned());
        state.set_ready(vec![topic(1, "A", 100)]);

        state.set_ready(vec![topic(2, "B", 50)]);

        assert_eq!(state.selected_index(), Some(0));
        assert_eq!(state.selected_topic().map(|t| t.topic_id), Some(2));
    }

    #[test]
    fn navigation_moves_within_bounds() {
        let mut state = ForumTopicListState::loading(100, "Forum".to_owned());
        state.set_ready(vec![
            topic(1, "A", 100),
            topic(2, "B", 50),
            topic(3, "C", 10),
        ]);

        state.select_next();
        state.select_next();
        state.select_next(); // clamps at last
        assert_eq!(state.selected_index(), Some(2));

        state.select_previous();
        state.select_previous();
        state.select_previous(); // clamps at first
        assert_eq!(state.selected_index(), Some(0));
    }

    #[test]
    fn select_first_resets_cursor_to_zero() {
        let mut state = ForumTopicListState::loading(100, "Forum".to_owned());
        state.set_ready(vec![topic(1, "A", 100), topic(2, "B", 50)]);
        state.select_next();

        state.select_first();

        assert_eq!(state.selected_index(), Some(0));
    }

    #[test]
    fn set_error_clears_topics_and_selection() {
        let mut state = ForumTopicListState::loading(100, "Forum".to_owned());
        state.set_ready(vec![topic(1, "A", 100)]);

        state.set_error();

        assert_eq!(state.ui_state(), ForumTopicListUiState::Error);
        assert!(state.topics().is_empty());
        assert_eq!(state.selected_index(), None);
    }

    #[test]
    fn select_first_noop_on_empty_state() {
        let mut state = ForumTopicListState::loading(100, "Forum".to_owned());
        state.select_first();
        assert_eq!(state.selected_index(), None);
    }

    #[test]
    fn find_topic_returns_match_by_id() {
        let mut state = ForumTopicListState::loading(100, "Forum".to_owned());
        state.set_ready(vec![topic(1, "A", 100), topic(2, "B", 50)]);

        assert_eq!(state.find_topic(2).map(|t| t.name.as_str()), Some("B"));
        assert!(state.find_topic(999).is_none());
    }
}
