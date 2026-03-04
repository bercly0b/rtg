//! Typed TDLib update events.
//!
//! These events are dispatched from the TDLib update loop and can be consumed
//! by the chat updates monitor to trigger UI refreshes.

/// Typed TDLib update events dispatched from the update loop.
///
/// These represent the subset of TDLib updates that are relevant for
/// the RTG UI (chat list, message view, user status).
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields are used for pattern matching and future granular handling
pub enum TdLibUpdate {
    /// New message received in a chat.
    NewMessage { chat_id: i64 },

    /// Message content changed.
    MessageContent { chat_id: i64, message_id: i64 },

    /// Messages were deleted.
    DeleteMessages { chat_id: i64, message_ids: Vec<i64> },

    /// Chat's last message changed (affects chat list preview).
    ChatLastMessage { chat_id: i64 },

    /// Chat position changed (affects chat list ordering).
    ChatPosition { chat_id: i64 },

    /// Incoming messages were read (affects unread count).
    ChatReadInbox { chat_id: i64 },

    /// Outgoing messages were read (affects read receipts).
    ChatReadOutbox { chat_id: i64 },

    /// User status changed (online/offline).
    UserStatus { user_id: i64 },

    /// Message send succeeded (for sent message confirmation).
    MessageSendSucceeded { chat_id: i64, old_message_id: i64 },
}

impl TdLibUpdate {
    /// Returns the chat_id affected by this update, if any.
    #[allow(dead_code)] // Will be used for filtering updates
    pub fn chat_id(&self) -> Option<i64> {
        match self {
            TdLibUpdate::NewMessage { chat_id }
            | TdLibUpdate::MessageContent { chat_id, .. }
            | TdLibUpdate::DeleteMessages { chat_id, .. }
            | TdLibUpdate::ChatLastMessage { chat_id }
            | TdLibUpdate::ChatPosition { chat_id }
            | TdLibUpdate::ChatReadInbox { chat_id }
            | TdLibUpdate::ChatReadOutbox { chat_id }
            | TdLibUpdate::MessageSendSucceeded { chat_id, .. } => Some(*chat_id),
            TdLibUpdate::UserStatus { .. } => None,
        }
    }

    /// Returns the kind of update for logging.
    #[allow(dead_code)] // Will be used in Phase 6.3
    pub fn kind(&self) -> &'static str {
        match self {
            TdLibUpdate::NewMessage { .. } => "new_message",
            TdLibUpdate::MessageContent { .. } => "message_content",
            TdLibUpdate::DeleteMessages { .. } => "delete_messages",
            TdLibUpdate::ChatLastMessage { .. } => "chat_last_message",
            TdLibUpdate::ChatPosition { .. } => "chat_position",
            TdLibUpdate::ChatReadInbox { .. } => "chat_read_inbox",
            TdLibUpdate::ChatReadOutbox { .. } => "chat_read_outbox",
            TdLibUpdate::UserStatus { .. } => "user_status",
            TdLibUpdate::MessageSendSucceeded { .. } => "message_send_succeeded",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_message_has_chat_id() {
        let update = TdLibUpdate::NewMessage { chat_id: 123 };
        assert_eq!(update.chat_id(), Some(123));
        assert_eq!(update.kind(), "new_message");
    }

    #[test]
    fn user_status_has_no_chat_id() {
        let update = TdLibUpdate::UserStatus { user_id: 456 };
        assert_eq!(update.chat_id(), None);
        assert_eq!(update.kind(), "user_status");
    }

    #[test]
    fn message_send_succeeded_has_chat_id() {
        let update = TdLibUpdate::MessageSendSucceeded {
            chat_id: 789,
            old_message_id: 100,
        };
        assert_eq!(update.chat_id(), Some(789));
        assert_eq!(update.kind(), "message_send_succeeded");
    }
}
