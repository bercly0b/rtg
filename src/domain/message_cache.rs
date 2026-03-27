use std::collections::{HashMap, VecDeque};

use super::message::Message;

/// Default maximum number of chats to keep in cache.
pub const DEFAULT_MAX_CACHED_CHATS: usize = 50;

/// Default maximum number of messages to retain per chat.
pub const DEFAULT_MAX_MESSAGES_PER_CHAT: usize = 200;

/// Default minimum number of cached messages required to show them immediately.
/// If the cache holds fewer messages than this threshold, the UI shows a
/// "Loading" state instead of a sparse preview (eliminates the "1 message flash").
pub const DEFAULT_MIN_DISPLAY_MESSAGES: usize = 5;

/// Per-chat cached message storage.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ChatMessages {
    messages: Vec<Message>,
    /// Whether a full network fetch (not just local cache) has been completed.
    fully_loaded: bool,
}

/// In-memory cache of fetched messages across all visited chats.
///
/// Stores messages independently from `OpenChatState` (which holds only the
/// currently displayed chat). When the user revisits a previously opened chat,
/// the cache provides instant access without any TDLib calls.
///
/// Implements LRU eviction: when the number of cached chats exceeds
/// `max_cached_chats`, the least recently accessed chat is evicted.
///
/// Lives in the domain layer — pure data, no I/O.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageCache {
    chats: HashMap<i64, ChatMessages>,
    /// LRU tracking: front = least recently used, back = most recently used.
    access_order: VecDeque<i64>,
    max_cached_chats: usize,
    max_messages_per_chat: usize,
}

impl Default for MessageCache {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_CACHED_CHATS, DEFAULT_MAX_MESSAGES_PER_CHAT)
    }
}

impl MessageCache {
    pub fn new(max_cached_chats: usize, max_messages_per_chat: usize) -> Self {
        Self {
            chats: HashMap::new(),
            access_order: VecDeque::new(),
            max_cached_chats: max_cached_chats.max(1),
            max_messages_per_chat: max_messages_per_chat.max(1),
        }
    }

    /// Returns cached messages for a chat, or `None` if not cached.
    ///
    /// Counts as an access for LRU tracking (requires `&mut self`).
    pub fn get(&mut self, chat_id: i64) -> Option<&[Message]> {
        if self.chats.contains_key(&chat_id) {
            self.touch(chat_id);
            self.chats.get(&chat_id).map(|cm| cm.messages.as_slice())
        } else {
            None
        }
    }

    /// Stores (or replaces) messages for a chat.
    ///
    /// Truncates to `max_messages_per_chat` (keeping the newest messages).
    /// Evicts the least recently used chat if the cache is full.
    pub fn put(&mut self, chat_id: i64, messages: Vec<Message>, fully_loaded: bool) {
        let mut truncated = messages;
        self.truncate_messages(&mut truncated);

        self.chats.insert(
            chat_id,
            ChatMessages {
                messages: truncated,
                fully_loaded,
            },
        );
        self.touch(chat_id);
        self.evict_if_needed();
    }

    /// Returns `true` if the cache has a non-empty entry for this chat.
    ///
    /// Does NOT count as an access for LRU tracking.
    pub fn has_messages(&self, chat_id: i64) -> bool {
        self.chats
            .get(&chat_id)
            .is_some_and(|cm| !cm.messages.is_empty())
    }

    /// Returns whether a full network fetch has been completed for this chat.
    #[allow(dead_code)]
    pub fn is_fully_loaded(&self, chat_id: i64) -> bool {
        self.chats.get(&chat_id).is_some_and(|cm| cm.fully_loaded)
    }

    /// Appends a message to a chat's cached messages.
    ///
    /// Inserts in timestamp order (oldest first). If the chat has no cache
    /// entry yet, creates one with just this message. Skips duplicates by ID.
    /// Evicts the least recently used chat if the cache is full.
    pub fn add_message(&mut self, chat_id: i64, message: Message) {
        let is_new_entry = !self.chats.contains_key(&chat_id);

        let entry = self.chats.entry(chat_id).or_insert_with(|| ChatMessages {
            messages: Vec::new(),
            fully_loaded: false,
        });

        // Skip if already present (dedup by message ID)
        if entry.messages.iter().any(|m| m.id == message.id) {
            return;
        }

        // Insert in timestamp order (messages are sorted oldest-first)
        let pos = entry
            .messages
            .partition_point(|m| m.timestamp_ms <= message.timestamp_ms);
        entry.messages.insert(pos, message);

        self.truncate_messages_for_chat(chat_id);
        self.touch(chat_id);

        if is_new_entry {
            self.evict_if_needed();
        }
    }

    /// Removes messages by ID from a chat's cache.
    ///
    /// Does NOT count as an access for LRU tracking.
    pub fn remove_messages(&mut self, chat_id: i64, message_ids: &[i64]) {
        if let Some(entry) = self.chats.get_mut(&chat_id) {
            entry.messages.retain(|m| !message_ids.contains(&m.id));
        }
    }

    /// Updates the `FileInfo` of a specific message in the cache.
    ///
    /// If the message is found and has `file_info`, the closure is called
    /// to mutate it (e.g., to update download status/progress).
    pub fn update_file_info(
        &mut self,
        chat_id: i64,
        message_id: i64,
        updater: impl FnOnce(&mut super::message::FileInfo),
    ) {
        if let Some(entry) = self.chats.get_mut(&chat_id) {
            if let Some(msg) = entry.messages.iter_mut().find(|m| m.id == message_id) {
                if let Some(ref mut fi) = msg.file_info {
                    updater(fi);
                }
            }
        }
    }

    /// Updates the `reaction_count` of a specific message in the cache.
    pub fn update_reaction_count(&mut self, chat_id: i64, message_id: i64, reaction_count: u32) {
        if let Some(entry) = self.chats.get_mut(&chat_id) {
            if let Some(msg) = entry.messages.iter_mut().find(|m| m.id == message_id) {
                msg.reaction_count = reaction_count;
            }
        }
    }

    /// Moves `chat_id` to the back (most recently used) of `access_order`.
    ///
    /// O(n) in the number of cached chats via `VecDeque::retain`. Acceptable
    /// for `max_cached_chats` up to ~500; consider the `lru` crate if bounds
    /// grow significantly.
    fn touch(&mut self, chat_id: i64) {
        self.access_order.retain(|&id| id != chat_id);
        self.access_order.push_back(chat_id);
    }

    /// Evicts the least recently used chats until within capacity.
    fn evict_if_needed(&mut self) {
        while self.chats.len() > self.max_cached_chats {
            if let Some(evicted_id) = self.access_order.pop_front() {
                tracing::debug!(chat_id = evicted_id, "evicting LRU chat from message cache");
                self.chats.remove(&evicted_id);
            } else {
                break;
            }
        }
    }

    /// Truncates a message vec to keep only the newest `max_messages_per_chat`.
    fn truncate_messages(&self, messages: &mut Vec<Message>) {
        if messages.len() > self.max_messages_per_chat {
            let start = messages.len() - self.max_messages_per_chat;
            *messages = messages.split_off(start);
        }
    }

    /// Truncates in-place for an existing chat entry.
    fn truncate_messages_for_chat(&mut self, chat_id: i64) {
        if let Some(entry) = self.chats.get_mut(&chat_id) {
            let max = self.max_messages_per_chat;
            if entry.messages.len() > max {
                let start = entry.messages.len() - max;
                entry.messages = entry.messages.split_off(start);
            }
        }
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
            file_info: None,
            reply_to: None,
            reaction_count: 0,
            links: Vec::new(),
            is_edited: false,
        }
    }

    fn msg_with_ts(id: i64, text: &str, timestamp_ms: i64) -> Message {
        Message {
            id,
            sender_name: "User".to_owned(),
            text: text.to_owned(),
            timestamp_ms,
            is_outgoing: false,
            media: MessageMedia::None,
            status: MessageStatus::Delivered,
            file_info: None,
            reply_to: None,
            reaction_count: 0,
            links: Vec::new(),
            is_edited: false,
        }
    }

    // ── Basic operations ──

    #[test]
    fn empty_cache_returns_none() {
        let mut cache = MessageCache::default();
        assert!(cache.get(42).is_none());
    }

    #[test]
    fn empty_cache_has_no_messages() {
        let cache = MessageCache::default();
        assert!(!cache.has_messages(42));
    }

    #[test]
    fn put_and_get_roundtrip() {
        let mut cache = MessageCache::default();
        let messages = vec![msg(1, "hello"), msg(2, "world")];

        cache.put(100, messages.clone(), true);

        let cached = cache.get(100).unwrap();
        assert_eq!(cached.len(), 2);
        assert_eq!(cached[0].text, "hello");
        assert_eq!(cached[1].text, "world");
    }

    #[test]
    fn has_messages_returns_true_after_put() {
        let mut cache = MessageCache::default();
        cache.put(100, vec![msg(1, "hello")], true);
        assert!(cache.has_messages(100));
    }

    #[test]
    fn has_messages_returns_false_for_empty_vec() {
        let mut cache = MessageCache::default();
        cache.put(100, vec![], true);
        assert!(!cache.has_messages(100));
    }

    #[test]
    fn put_replaces_existing_entry() {
        let mut cache = MessageCache::default();
        cache.put(100, vec![msg(1, "old")], false);
        cache.put(100, vec![msg(2, "new"), msg(3, "data")], true);

        let cached = cache.get(100).unwrap();
        assert_eq!(cached.len(), 2);
        assert_eq!(cached[0].text, "new");
    }

    #[test]
    fn multiple_chats_are_independent() {
        let mut cache = MessageCache::default();
        cache.put(1, vec![msg(10, "chat one")], true);
        cache.put(2, vec![msg(20, "chat two")], true);

        assert_eq!(cache.get(1).unwrap()[0].text, "chat one");
        assert_eq!(cache.get(2).unwrap()[0].text, "chat two");
    }

    #[test]
    fn get_unknown_chat_returns_none() {
        let mut cache = MessageCache::default();
        cache.put(1, vec![msg(10, "hello")], true);
        assert!(cache.get(999).is_none());
    }

    // ── add_message tests ──

    #[test]
    fn add_message_to_empty_cache_creates_entry() {
        let mut cache = MessageCache::default();
        cache.add_message(100, msg(1, "hello"));

        assert!(cache.has_messages(100));
        assert_eq!(cache.get(100).unwrap().len(), 1);
        assert_eq!(cache.get(100).unwrap()[0].text, "hello");
    }

    #[test]
    fn add_message_appends_to_existing_entry() {
        let mut cache = MessageCache::default();
        cache.put(100, vec![msg_with_ts(1, "first", 1000)], true);
        cache.add_message(100, msg_with_ts(2, "second", 2000));

        let cached = cache.get(100).unwrap();
        assert_eq!(cached.len(), 2);
        assert_eq!(cached[0].text, "first");
        assert_eq!(cached[1].text, "second");
    }

    #[test]
    fn add_message_inserts_in_timestamp_order() {
        let mut cache = MessageCache::default();
        cache.put(
            100,
            vec![msg_with_ts(1, "early", 1000), msg_with_ts(3, "late", 3000)],
            true,
        );
        cache.add_message(100, msg_with_ts(2, "middle", 2000));

        let cached = cache.get(100).unwrap();
        assert_eq!(cached.len(), 3);
        assert_eq!(cached[0].text, "early");
        assert_eq!(cached[1].text, "middle");
        assert_eq!(cached[2].text, "late");
    }

    #[test]
    fn add_message_skips_duplicate_by_id() {
        let mut cache = MessageCache::default();
        cache.put(100, vec![msg(1, "original")], true);
        cache.add_message(100, msg(1, "duplicate"));

        let cached = cache.get(100).unwrap();
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].text, "original");
    }

    // ── remove_messages tests ──

    #[test]
    fn remove_messages_removes_by_id() {
        let mut cache = MessageCache::default();
        cache.put(100, vec![msg(1, "a"), msg(2, "b"), msg(3, "c")], true);
        cache.remove_messages(100, &[2]);

        let cached = cache.get(100).unwrap();
        assert_eq!(cached.len(), 2);
        assert_eq!(cached[0].text, "a");
        assert_eq!(cached[1].text, "c");
    }

    #[test]
    fn remove_messages_handles_multiple_ids() {
        let mut cache = MessageCache::default();
        cache.put(100, vec![msg(1, "a"), msg(2, "b"), msg(3, "c")], true);
        cache.remove_messages(100, &[1, 3]);

        let cached = cache.get(100).unwrap();
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].text, "b");
    }

    #[test]
    fn remove_messages_ignores_unknown_chat() {
        let mut cache = MessageCache::default();
        cache.remove_messages(999, &[1, 2]); // should not panic
        assert!(cache.get(999).is_none());
    }

    #[test]
    fn remove_messages_ignores_unknown_ids() {
        let mut cache = MessageCache::default();
        cache.put(100, vec![msg(1, "a")], true);
        cache.remove_messages(100, &[999]);

        assert_eq!(cache.get(100).unwrap().len(), 1);
    }

    // ── LRU eviction tests ──

    #[test]
    fn evicts_lru_chat_when_over_capacity() {
        let mut cache = MessageCache::new(3, 200);

        cache.put(1, vec![msg(10, "chat 1")], true);
        cache.put(2, vec![msg(20, "chat 2")], true);
        cache.put(3, vec![msg(30, "chat 3")], true);
        // All 3 fit
        assert!(cache.has_messages(1));

        // Adding a 4th should evict chat 1 (LRU)
        cache.put(4, vec![msg(40, "chat 4")], true);

        assert!(!cache.has_messages(1), "chat 1 should have been evicted");
        assert!(cache.has_messages(2));
        assert!(cache.has_messages(3));
        assert!(cache.has_messages(4));
    }

    #[test]
    fn get_touch_prevents_eviction() {
        let mut cache = MessageCache::new(3, 200);

        cache.put(1, vec![msg(10, "chat 1")], true);
        cache.put(2, vec![msg(20, "chat 2")], true);
        cache.put(3, vec![msg(30, "chat 3")], true);

        // Touch chat 1 via get (makes it MRU)
        let _ = cache.get(1);

        // Adding chat 4 should evict chat 2 (now LRU), not chat 1
        cache.put(4, vec![msg(40, "chat 4")], true);

        assert!(cache.has_messages(1), "chat 1 should survive (was touched)");
        assert!(!cache.has_messages(2), "chat 2 should be evicted (now LRU)");
        assert!(cache.has_messages(3));
        assert!(cache.has_messages(4));
    }

    #[test]
    fn has_messages_does_not_touch_lru() {
        let mut cache = MessageCache::new(3, 200);

        cache.put(1, vec![msg(10, "chat 1")], true);
        cache.put(2, vec![msg(20, "chat 2")], true);
        cache.put(3, vec![msg(30, "chat 3")], true);

        // has_messages should NOT touch LRU order
        assert!(cache.has_messages(1));

        // Adding chat 4 should still evict chat 1 (not touched by has_messages)
        cache.put(4, vec![msg(40, "chat 4")], true);

        assert!(
            !cache.has_messages(1),
            "chat 1 should be evicted (has_messages doesn't touch)"
        );
        assert!(cache.has_messages(2));
    }

    #[test]
    fn add_message_new_entry_can_trigger_eviction() {
        let mut cache = MessageCache::new(2, 200);

        cache.put(1, vec![msg(10, "a")], true);
        cache.put(2, vec![msg(20, "b")], true);

        // add_message for a new chat should evict the LRU
        cache.add_message(3, msg(30, "c"));

        assert!(!cache.has_messages(1), "chat 1 should be evicted");
        assert!(cache.has_messages(2));
        assert!(cache.has_messages(3));
    }

    #[test]
    fn add_message_existing_entry_does_not_evict() {
        let mut cache = MessageCache::new(2, 200);

        cache.put(1, vec![msg(10, "a")], true);
        cache.put(2, vec![msg(20, "b")], true);

        // add_message to existing chat should not evict anything
        cache.add_message(2, msg(21, "b2"));

        assert!(cache.has_messages(1));
        assert!(cache.has_messages(2));
        assert_eq!(cache.get(2).unwrap().len(), 2);
    }

    // ── Per-chat message limit tests ──

    #[test]
    fn put_truncates_to_max_messages_per_chat() {
        let mut cache = MessageCache::new(50, 3);

        cache.put(
            1,
            vec![
                msg_with_ts(1, "oldest", 1000),
                msg_with_ts(2, "old", 2000),
                msg_with_ts(3, "mid", 3000),
                msg_with_ts(4, "new", 4000),
                msg_with_ts(5, "newest", 5000),
            ],
            true,
        );

        let cached = cache.get(1).unwrap();
        assert_eq!(cached.len(), 3, "should keep only 3 newest messages");
        assert_eq!(cached[0].text, "mid");
        assert_eq!(cached[1].text, "new");
        assert_eq!(cached[2].text, "newest");
    }

    #[test]
    fn add_message_respects_max_messages_per_chat() {
        let mut cache = MessageCache::new(50, 3);

        cache.put(
            1,
            vec![
                msg_with_ts(1, "a", 1000),
                msg_with_ts(2, "b", 2000),
                msg_with_ts(3, "c", 3000),
            ],
            true,
        );

        // Adding a 4th should truncate to 3
        cache.add_message(1, msg_with_ts(4, "d", 4000));

        let cached = cache.get(1).unwrap();
        assert_eq!(cached.len(), 3);
        assert_eq!(cached[0].text, "b");
        assert_eq!(cached[1].text, "c");
        assert_eq!(cached[2].text, "d");
    }

    // ── fully_loaded tracking tests ──

    #[test]
    fn fully_loaded_false_by_default() {
        let cache = MessageCache::default();
        assert!(!cache.is_fully_loaded(42));
    }

    #[test]
    fn fully_loaded_true_after_put_with_flag() {
        let mut cache = MessageCache::default();
        cache.put(1, vec![msg(1, "a")], true);
        assert!(cache.is_fully_loaded(1));
    }

    #[test]
    fn fully_loaded_false_after_put_with_false_flag() {
        let mut cache = MessageCache::default();
        cache.put(1, vec![msg(1, "a")], false);
        assert!(!cache.is_fully_loaded(1));
    }

    #[test]
    fn fully_loaded_replaced_on_subsequent_put() {
        let mut cache = MessageCache::default();
        cache.put(1, vec![msg(1, "a")], true);
        assert!(cache.is_fully_loaded(1));

        cache.put(1, vec![msg(1, "a")], false);
        assert!(!cache.is_fully_loaded(1));
    }

    #[test]
    fn add_message_creates_not_fully_loaded_entry() {
        let mut cache = MessageCache::default();
        cache.add_message(1, msg(1, "a"));
        assert!(!cache.is_fully_loaded(1));
    }

    #[test]
    fn add_message_preserves_fully_loaded_flag() {
        let mut cache = MessageCache::default();
        cache.put(1, vec![msg(1, "a")], true);
        cache.add_message(1, msg(2, "b"));
        assert!(cache.is_fully_loaded(1));
    }

    // ── Default config tests ──

    #[test]
    fn default_cache_has_expected_limits() {
        let cache = MessageCache::default();
        assert_eq!(cache.max_cached_chats, DEFAULT_MAX_CACHED_CHATS);
        assert_eq!(cache.max_messages_per_chat, DEFAULT_MAX_MESSAGES_PER_CHAT);
    }

    // ── Reaction count update tests ──

    #[test]
    fn update_reaction_count_modifies_existing_message() {
        let mut cache = MessageCache::default();
        cache.put(1, vec![msg(10, "hello"), msg(20, "world")], false);

        cache.update_reaction_count(1, 20, 5);

        let messages = cache.get(1).unwrap();
        assert_eq!(messages[0].reaction_count, 0);
        assert_eq!(messages[1].reaction_count, 5);
    }

    #[test]
    fn update_reaction_count_noop_for_unknown_chat() {
        let mut cache = MessageCache::default();
        cache.update_reaction_count(999, 1, 3);
        assert!(cache.get(999).is_none());
    }

    #[test]
    fn update_reaction_count_noop_for_unknown_message() {
        let mut cache = MessageCache::default();
        cache.put(1, vec![msg(10, "hello")], false);

        cache.update_reaction_count(1, 999, 3);

        let messages = cache.get(1).unwrap();
        assert_eq!(messages[0].reaction_count, 0);
    }
}
