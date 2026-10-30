use super::*;

// ── Recording task dispatcher for tests ──

type SendRecord = (i64, Option<i32>, String, Option<i64>);
type MarkAsReadRecord = (i64, Option<i32>, Vec<i64>);

/// Records what the orchestrator dispatched and allows inspection.
pub(super) struct RecordingDispatcher {
    dispatched_chat_list_count: RefCell<usize>,
    dispatched_chat_list_force: RefCell<Vec<bool>>,
    dispatched_messages: RefCell<Vec<(i64, Option<i32>)>>,
    dispatched_older_messages: RefCell<Vec<(i64, Option<i32>, i64)>>,
    dispatched_sends: RefCell<Vec<SendRecord>>,
    dispatched_open_chats: RefCell<Vec<i64>>,
    dispatched_close_chats: RefCell<Vec<i64>>,
    dispatched_mark_as_read: RefCell<Vec<MarkAsReadRecord>>,
    dispatched_mark_chat_as_read: RefCell<Vec<(i64, i64)>>,
    dispatched_prefetches: RefCell<Vec<(i64, Option<i32>)>>,
    dispatched_forum_topics: RefCell<Vec<i64>>,
    dispatched_forum_unread_counts: RefCell<Vec<Vec<i64>>>,
    dispatched_deletes: RefCell<Vec<(i64, i64)>>,
    dispatched_voice_sends: RefCell<Vec<(i64, Option<i32>, String)>>,
    dispatched_subtitles: RefCell<Vec<ChatSubtitleQuery>>,
    dispatched_add_reactions: RefCell<Vec<(i64, i64, String)>>,
    dispatched_remove_reactions: RefCell<Vec<(i64, i64, String)>>,
}

impl RecordingDispatcher {
    pub(super) fn new() -> Self {
        Self {
            dispatched_chat_list_count: RefCell::new(0),
            dispatched_chat_list_force: RefCell::new(Vec::new()),
            dispatched_messages: RefCell::new(Vec::new()),
            dispatched_older_messages: RefCell::new(Vec::new()),
            dispatched_sends: RefCell::new(Vec::new()),
            dispatched_open_chats: RefCell::new(Vec::new()),
            dispatched_close_chats: RefCell::new(Vec::new()),
            dispatched_mark_as_read: RefCell::new(Vec::new()),
            dispatched_mark_chat_as_read: RefCell::new(Vec::new()),
            dispatched_prefetches: RefCell::new(Vec::new()),
            dispatched_forum_topics: RefCell::new(Vec::new()),
            dispatched_forum_unread_counts: RefCell::new(Vec::new()),
            dispatched_deletes: RefCell::new(Vec::new()),
            dispatched_voice_sends: RefCell::new(Vec::new()),
            dispatched_subtitles: RefCell::new(Vec::new()),
            dispatched_add_reactions: RefCell::new(Vec::new()),
            dispatched_remove_reactions: RefCell::new(Vec::new()),
        }
    }

    pub(super) fn chat_list_dispatch_count(&self) -> usize {
        *self.dispatched_chat_list_count.borrow()
    }

    pub(super) fn last_chat_list_force(&self) -> Option<bool> {
        self.dispatched_chat_list_force.borrow().last().copied()
    }

    pub(super) fn messages_dispatch_count(&self) -> usize {
        self.dispatched_messages.borrow().len()
    }

    pub(super) fn older_messages_dispatch_count(&self) -> usize {
        self.dispatched_older_messages.borrow().len()
    }

    pub(super) fn last_older_messages(&self) -> Option<(i64, i64)> {
        self.dispatched_older_messages
            .borrow()
            .last()
            .map(|(chat_id, _, from)| (*chat_id, *from))
    }

    #[allow(dead_code)]
    pub(super) fn last_older_messages_full(&self) -> Option<(i64, Option<i32>, i64)> {
        self.dispatched_older_messages.borrow().last().copied()
    }

    #[allow(dead_code)]
    pub(super) fn last_load_messages(&self) -> Option<(i64, Option<i32>)> {
        self.dispatched_messages.borrow().last().copied()
    }

    #[allow(dead_code)]
    pub(super) fn last_forum_topics_chat_id(&self) -> Option<i64> {
        self.dispatched_forum_topics.borrow().last().copied()
    }

    #[allow(dead_code)]
    pub(super) fn forum_topics_dispatch_count(&self) -> usize {
        self.dispatched_forum_topics.borrow().len()
    }

    #[allow(dead_code)]
    pub(super) fn forum_unread_counts_dispatch_count(&self) -> usize {
        self.dispatched_forum_unread_counts.borrow().len()
    }

    #[allow(dead_code)]
    pub(super) fn last_forum_unread_counts_chat_ids(&self) -> Option<Vec<i64>> {
        self.dispatched_forum_unread_counts.borrow().last().cloned()
    }

    pub(super) fn send_dispatch_count(&self) -> usize {
        self.dispatched_sends.borrow().len()
    }

    pub(super) fn last_send(&self) -> Option<(i64, String, Option<i64>)> {
        self.dispatched_sends
            .borrow()
            .last()
            .map(|(c, _, t, r)| (*c, t.clone(), *r))
    }

    #[allow(dead_code)]
    pub(super) fn last_send_full(&self) -> Option<(i64, Option<i32>, String, Option<i64>)> {
        self.dispatched_sends.borrow().last().cloned()
    }

    pub(super) fn open_chat_dispatch_count(&self) -> usize {
        self.dispatched_open_chats.borrow().len()
    }

    pub(super) fn close_chat_dispatch_count(&self) -> usize {
        self.dispatched_close_chats.borrow().len()
    }

    pub(super) fn mark_as_read_dispatch_count(&self) -> usize {
        self.dispatched_mark_as_read.borrow().len()
    }

    pub(super) fn last_mark_as_read(&self) -> Option<(i64, Vec<i64>)> {
        self.dispatched_mark_as_read
            .borrow()
            .last()
            .map(|(c, _, ids)| (*c, ids.clone()))
    }

    #[allow(dead_code)]
    pub(super) fn last_mark_as_read_full(&self) -> Option<(i64, Option<i32>, Vec<i64>)> {
        self.dispatched_mark_as_read.borrow().last().cloned()
    }

    pub(super) fn mark_chat_as_read_dispatch_count(&self) -> usize {
        self.dispatched_mark_chat_as_read.borrow().len()
    }

    pub(super) fn last_mark_chat_as_read(&self) -> Option<(i64, i64)> {
        self.dispatched_mark_chat_as_read.borrow().last().cloned()
    }

    pub(super) fn prefetch_dispatch_count(&self) -> usize {
        self.dispatched_prefetches.borrow().len()
    }

    pub(super) fn last_prefetch_chat_id(&self) -> Option<i64> {
        self.dispatched_prefetches
            .borrow()
            .last()
            .map(|(chat_id, _)| *chat_id)
    }

    pub(super) fn delete_dispatch_count(&self) -> usize {
        self.dispatched_deletes.borrow().len()
    }

    pub(super) fn last_delete(&self) -> Option<(i64, i64)> {
        self.dispatched_deletes.borrow().last().copied()
    }

    pub(super) fn voice_send_dispatch_count(&self) -> usize {
        self.dispatched_voice_sends.borrow().len()
    }

    pub(super) fn last_voice_send(&self) -> Option<(i64, String)> {
        self.dispatched_voice_sends
            .borrow()
            .last()
            .map(|(c, _, f)| (*c, f.clone()))
    }

    #[allow(dead_code)]
    pub(super) fn last_voice_send_full(&self) -> Option<(i64, Option<i32>, String)> {
        self.dispatched_voice_sends.borrow().last().cloned()
    }

    pub(super) fn subtitle_dispatch_count(&self) -> usize {
        self.dispatched_subtitles.borrow().len()
    }

    pub(super) fn last_subtitle_query(&self) -> Option<ChatSubtitleQuery> {
        self.dispatched_subtitles.borrow().last().cloned()
    }

    pub(super) fn last_add_reaction(&self) -> Option<(i64, i64, String)> {
        self.dispatched_add_reactions.borrow().last().cloned()
    }

    pub(super) fn last_remove_reaction(&self) -> Option<(i64, i64, String)> {
        self.dispatched_remove_reactions.borrow().last().cloned()
    }
}

impl TaskDispatcher for RecordingDispatcher {
    fn dispatch_chat_list(&self, force: bool, _limit: usize) {
        *self.dispatched_chat_list_count.borrow_mut() += 1;
        self.dispatched_chat_list_force.borrow_mut().push(force);
    }

    fn dispatch_load_forum_topics(&self, chat_id: i64) {
        self.dispatched_forum_topics.borrow_mut().push(chat_id);
    }

    fn dispatch_forum_unread_counts(&self, chat_ids: Vec<i64>) {
        self.dispatched_forum_unread_counts
            .borrow_mut()
            .push(chat_ids);
    }

    fn dispatch_load_messages(&self, chat_id: i64, topic_id: Option<i32>) {
        self.dispatched_messages
            .borrow_mut()
            .push((chat_id, topic_id));
    }

    fn dispatch_load_older_messages(
        &self,
        chat_id: i64,
        topic_id: Option<i32>,
        from_message_id: i64,
    ) {
        self.dispatched_older_messages
            .borrow_mut()
            .push((chat_id, topic_id, from_message_id));
    }

    fn dispatch_send_message(
        &self,
        chat_id: i64,
        topic_id: Option<i32>,
        text: String,
        reply_to_message_id: Option<i64>,
    ) {
        self.dispatched_sends
            .borrow_mut()
            .push((chat_id, topic_id, text, reply_to_message_id));
    }

    fn dispatch_open_chat(&self, chat_id: i64) {
        self.dispatched_open_chats.borrow_mut().push(chat_id);
    }

    fn dispatch_close_chat(&self, chat_id: i64) {
        self.dispatched_close_chats.borrow_mut().push(chat_id);
    }

    fn dispatch_mark_as_read(&self, chat_id: i64, topic_id: Option<i32>, message_ids: Vec<i64>) {
        self.dispatched_mark_as_read
            .borrow_mut()
            .push((chat_id, topic_id, message_ids));
    }

    fn dispatch_mark_chat_as_read(&self, chat_id: i64, last_message_id: i64) {
        self.dispatched_mark_chat_as_read
            .borrow_mut()
            .push((chat_id, last_message_id));
    }

    fn dispatch_prefetch_messages(&self, chat_id: i64, topic_id: Option<i32>) {
        self.dispatched_prefetches
            .borrow_mut()
            .push((chat_id, topic_id));
    }

    fn dispatch_delete_message(&self, chat_id: i64, message_id: i64) {
        self.dispatched_deletes
            .borrow_mut()
            .push((chat_id, message_id));
    }

    fn dispatch_chat_subtitle(&self, query: ChatSubtitleQuery) {
        self.dispatched_subtitles.borrow_mut().push(query);
    }

    fn dispatch_send_voice(&self, chat_id: i64, topic_id: Option<i32>, file_path: String) {
        self.dispatched_voice_sends
            .borrow_mut()
            .push((chat_id, topic_id, file_path));
    }

    fn dispatch_download_file(&self, _file_id: i32) {
        // Recording: no-op for now
    }

    fn dispatch_chat_info(&self, _query: crate::usecases::chat_subtitle::ChatInfoQuery) {
        // Recording: no-op for now
    }

    fn dispatch_open_file(&self, _cmd_template: String, _file_path: String) {
        // Recording: no-op for now
    }

    fn dispatch_save_file(&self, _file_id: i32, _local_path: String, _file_name: Option<String>) {
        // Recording: no-op for now
    }

    fn dispatch_message_info(&self, _query: crate::usecases::message_info::MessageInfoQuery) {
        // Recording: no-op for now
    }

    fn dispatch_edit_message(&self, _chat_id: i64, _message_id: i64, _text: String) {}

    fn dispatch_available_reactions(
        &self,
        _query: crate::usecases::message_reactions::AvailableReactionsQuery,
    ) {
    }

    fn dispatch_add_reaction(&self, chat_id: i64, message_id: i64, emoji: String) {
        self.dispatched_add_reactions
            .borrow_mut()
            .push((chat_id, message_id, emoji));
    }

    fn dispatch_remove_reaction(&self, chat_id: i64, message_id: i64, emoji: String) {
        self.dispatched_remove_reactions
            .borrow_mut()
            .push((chat_id, message_id, emoji));
    }
}
