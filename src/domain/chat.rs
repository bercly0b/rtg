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
pub struct OutgoingReadStatus {
    /// Whether the last message was sent by the current user.
    pub is_outgoing: bool,
    /// Whether the last outgoing message was read by the recipient.
    /// Only meaningful when `is_outgoing` is true.
    pub is_read: bool,
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
    /// Read status of the last outgoing message.
    pub outgoing_status: OutgoingReadStatus,
}
