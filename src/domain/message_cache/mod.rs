use std::collections::{HashMap, VecDeque};

use super::message::Message;

#[cfg(test)]
mod tests;

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
