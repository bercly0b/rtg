/// Summary of a single forum topic inside a supergroup forum chat.
///
/// Mirrors `ChatSummary` for rendering in the topic list panel — fields are
/// intentionally narrow: only what the list item view actually needs. Heavy
/// data (icon stickers, draft messages, notification settings) is omitted at
/// this layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForumTopicSummary {
    /// Parent forum chat identifier.
    pub chat_id: i64,
    /// Forum topic identifier (TDLib `forum_topic_id`, also the id of the
    /// topic-creation message). 32-bit, distinct from `chat_id`.
    pub topic_id: i32,
    /// Topic name as shown to the user. Falls back to "General" for the
    /// implicit General topic if TDLib returns an empty string.
    pub name: String,
    /// True for the special General topic that exists on every forum.
    pub is_general: bool,
    /// True when the topic is closed (no new messages, except by admins).
    pub is_closed: bool,
    /// True when the topic is hidden above the list (only valid for General).
    pub is_hidden: bool,
    /// True when the topic is pinned at the top of the topic list.
    pub is_pinned: bool,
    /// Number of unread messages in the topic.
    pub unread_count: u32,
    /// Preview text of the topic's last message, if any.
    pub last_message_preview: Option<String>,
    /// Unix timestamp (ms) of the topic's last message.
    pub last_message_unix_ms: Option<i64>,
    /// Id of the topic's last message (used for mark-as-read).
    pub last_message_id: Option<i64>,
    /// TDLib-supplied sort key — higher means closer to the top.
    /// Mirrors `ForumTopic.order`.
    pub order: i64,
}
