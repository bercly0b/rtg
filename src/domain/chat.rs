/// Type of chat for UI rendering purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChatType {
    /// Private 1-to-1 conversation with a user.
    #[default]
    Private,
    /// Group chat (small group or megagroup).
    Group,
    /// Broadcast channel.
    Channel,
}

/// Information about the last outgoing message's read status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutgoingReadStatus {
    /// The last message was not sent by the current user.
    #[default]
    NotOutgoing,
    /// The last message was sent by the current user.
    Outgoing {
        /// Whether the message was read by the recipient.
        is_read: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatSummary {
    pub chat_id: i64,
    pub title: String,
    pub unread_count: u32,
    pub last_message_preview: Option<String>,
    pub last_message_unix_ms: Option<i64>,
    pub is_pinned: bool,
    /// Type of chat (Private, Group, Channel).
    pub chat_type: ChatType,
    /// Name of the sender of the last message (for group chats).
    pub last_message_sender: Option<String>,
    /// Whether the chat partner is online (only for private chats).
    pub is_online: Option<bool>,
    /// Whether the chat partner is a bot (only for private chats).
    pub is_bot: bool,
    /// Read status of the last outgoing message.
    pub outgoing_status: OutgoingReadStatus,
    /// ID of the last message in the chat (used for mark-as-read).
    pub last_message_id: Option<i64>,
    /// Number of unread reactions on own messages in this chat.
    pub unread_reaction_count: u32,
}
