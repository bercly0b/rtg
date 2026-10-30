use super::*;

// ── Recording opener for tests ──

#[derive(Debug, Default)]
pub(super) struct RecordingOpener {
    opened: RefCell<Vec<String>>,
}

impl ExternalOpener for RecordingOpener {
    fn open(&self, target: &str) -> anyhow::Result<()> {
        self.opened.borrow_mut().push(target.to_owned());
        Ok(())
    }
}

impl RecordingOpener {
    pub(super) fn opened_urls(&self) -> Vec<String> {
        self.opened.borrow().clone()
    }
}

// ── Domain helpers ──

pub(super) fn chat(chat_id: i64, title: &str) -> ChatSummary {
    use crate::domain::chat::{ChatType, OutgoingReadStatus};
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

pub(super) fn message(id: i64, text: &str) -> Message {
    Message {
        id,
        sender_name: "User".to_owned(),
        text: text.to_owned(),
        timestamp_ms: 1000,
        is_outgoing: false,
        media: crate::domain::message::MessageMedia::None,
        status: crate::domain::message::MessageStatus::Delivered,
        file_info: None,
        call_info: None,
        reply_to: None,
        forward_info: None,
        reaction_count: 0,
        links: Vec::new(),
        is_edited: false,
        is_service: false,
    }
}

/// Helper: build a ChatSummary that's a forum supergroup.
#[allow(dead_code)]
pub(super) fn forum_chat(chat_id: i64, title: &str) -> ChatSummary {
    let mut c = chat(chat_id, title);
    c.is_forum = true;
    c.chat_type = crate::domain::chat::ChatType::Group;
    c
}

/// Helper: build a ChatSummary that's a broadcast channel.
#[allow(dead_code)]
pub(super) fn channel_chat(chat_id: i64, title: &str) -> ChatSummary {
    let mut c = chat(chat_id, title);
    c.chat_type = crate::domain::chat::ChatType::Channel;
    c
}

/// Helper: build a ForumTopicSummary for tests.
#[allow(dead_code)]
pub(super) fn topic(
    chat_id: i64,
    topic_id: i32,
    name: &str,
    order: i64,
) -> crate::domain::forum_topic::ForumTopicSummary {
    crate::domain::forum_topic::ForumTopicSummary {
        chat_id,
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

// ── Stub cache source for tests ──

pub(super) struct StubCacheSource {
    messages: std::sync::Mutex<std::collections::HashMap<i64, Vec<Message>>>,
}

impl StubCacheSource {
    pub(super) fn empty() -> Self {
        Self {
            messages: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    pub(super) fn with_messages(entries: Vec<(i64, Vec<Message>)>) -> Self {
        let map = entries.into_iter().collect();
        Self {
            messages: std::sync::Mutex::new(map),
        }
    }
}

impl CachedMessagesSource for StubCacheSource {
    fn list_cached_messages(
        &self,
        chat_id: i64,
        _limit: usize,
    ) -> Result<Vec<Message>, crate::usecases::load_messages::MessagesSourceError> {
        let map = self.messages.lock().unwrap();
        Ok(map.get(&chat_id).cloned().unwrap_or_default())
    }
}
