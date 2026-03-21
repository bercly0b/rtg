use std::collections::HashMap;

use super::message::Message;

/// Per-chat cached message storage.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ChatMessages {
    messages: Vec<Message>,
}

/// In-memory cache of fetched messages across all visited chats.
///
/// Stores messages independently from `OpenChatState` (which holds only the
/// currently displayed chat). When the user revisits a previously opened chat,
/// the cache provides instant access without any TDLib calls.
///
/// Lives in the domain layer — pure data, no I/O.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageCache {
    chats: HashMap<i64, ChatMessages>,
}

impl Default for MessageCache {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageCache {
    pub fn new() -> Self {
        Self {
            chats: HashMap::new(),
        }
    }

    /// Returns cached messages for a chat, or `None` if not cached.
    pub fn get(&self, chat_id: i64) -> Option<&[Message]> {
        self.chats.get(&chat_id).map(|cm| cm.messages.as_slice())
    }

    /// Stores (or replaces) messages for a chat.
    pub fn put(&mut self, chat_id: i64, messages: Vec<Message>) {
        self.chats.insert(chat_id, ChatMessages { messages });
    }

    /// Returns `true` if the cache has a non-empty entry for this chat.
    #[allow(dead_code)]
    pub fn has_messages(&self, chat_id: i64) -> bool {
        self.chats
            .get(&chat_id)
            .is_some_and(|cm| !cm.messages.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::message::{MessageMedia, MessageStatus};

    fn msg(id: i64, text: &str) -> Message {
        Message {
            id,
            sender_name: "User".to_owned(),
            text: text.to_owned(),
            timestamp_ms: 1000,
            is_outgoing: false,
            media: MessageMedia::None,
            status: MessageStatus::Delivered,
        }
    }

    #[test]
    fn empty_cache_returns_none() {
        let cache = MessageCache::new();
        assert!(cache.get(42).is_none());
    }

    #[test]
    fn empty_cache_has_no_messages() {
        let cache = MessageCache::new();
        assert!(!cache.has_messages(42));
    }

    #[test]
    fn put_and_get_roundtrip() {
        let mut cache = MessageCache::new();
        let messages = vec![msg(1, "hello"), msg(2, "world")];

        cache.put(100, messages.clone());

        let cached = cache.get(100).unwrap();
        assert_eq!(cached.len(), 2);
        assert_eq!(cached[0].text, "hello");
        assert_eq!(cached[1].text, "world");
    }

    #[test]
    fn has_messages_returns_true_after_put() {
        let mut cache = MessageCache::new();
        cache.put(100, vec![msg(1, "hello")]);
        assert!(cache.has_messages(100));
    }

    #[test]
    fn has_messages_returns_false_for_empty_vec() {
        let mut cache = MessageCache::new();
        cache.put(100, vec![]);
        assert!(!cache.has_messages(100));
    }

    #[test]
    fn put_replaces_existing_entry() {
        let mut cache = MessageCache::new();
        cache.put(100, vec![msg(1, "old")]);
        cache.put(100, vec![msg(2, "new"), msg(3, "data")]);

        let cached = cache.get(100).unwrap();
        assert_eq!(cached.len(), 2);
        assert_eq!(cached[0].text, "new");
    }

    #[test]
    fn multiple_chats_are_independent() {
        let mut cache = MessageCache::new();
        cache.put(1, vec![msg(10, "chat one")]);
        cache.put(2, vec![msg(20, "chat two")]);

        assert_eq!(cache.get(1).unwrap()[0].text, "chat one");
        assert_eq!(cache.get(2).unwrap()[0].text, "chat two");
    }

    #[test]
    fn get_unknown_chat_returns_none() {
        let mut cache = MessageCache::new();
        cache.put(1, vec![msg(10, "hello")]);
        assert!(cache.get(999).is_none());
    }
}
