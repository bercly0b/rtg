//! Typed TDLib update events.
//!
//! These events are dispatched from the TDLib update loop and can be consumed
//! by the chat updates monitor to trigger UI refreshes and cache warming.

use tdlib_rs::enums::MessageContent;
use tdlib_rs::types::Message as TdMessage;

/// Typed TDLib update events dispatched from the update loop.
///
/// These represent the subset of TDLib updates that are relevant for
/// the RTG UI (chat list, message view, user status).
///
/// Message-carrying variants use `Box` to keep the enum size reasonable
/// since raw TDLib types are large structs.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields reserved for future granular update handling
pub enum TdLibUpdate {
    /// A new chat was discovered by TDLib. Carries the full Chat object
    /// for cache population. Guaranteed to arrive before the chat ID
    /// appears in any TDLib response.
    NewChat { chat: Box<tdlib_rs::types::Chat> },

    /// New message received in a chat. Carries the full raw TDLib message
    /// for mapping to domain types downstream.
    NewMessage {
        chat_id: i64,
        message: Box<TdMessage>,
    },

    /// Message content changed. Carries the new content for cache updates.
    MessageContentChanged {
        chat_id: i64,
        message_id: i64,
        new_content: Box<MessageContent>,
    },

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

    /// A user started or stopped a chat action (typing, recording, etc.).
    ChatAction {
        chat_id: i64,
        sender_user_id: i64,
        sender_name: String,
        action_label: String,
        is_cancel: bool,
    },

    /// Message send succeeded (for sent message confirmation).
    MessageSendSucceeded { chat_id: i64, old_message_id: i64 },

    /// Unread reaction count changed for a chat (affects chat list badge).
    ChatUnreadReactionCount { chat_id: i64 },

    /// Message interaction info changed (reaction counts on a message).
    MessageInteractionInfoChanged {
        chat_id: i64,
        message_id: i64,
        reaction_count: u32,
    },

    /// File download progress or completion update.
    FileUpdated {
        file_id: i32,
        size: i64,
        expected_size: i64,
        local_path: String,
        is_downloading_active: bool,
        is_downloading_completed: bool,
        downloaded_size: i64,
    },
}

impl TdLibUpdate {
    /// Returns the chat_id affected by this update, if any.
    #[allow(dead_code)]
    pub fn chat_id(&self) -> Option<i64> {
        match self {
            TdLibUpdate::NewChat { chat } => Some(chat.id),
            TdLibUpdate::NewMessage { chat_id, .. }
            | TdLibUpdate::MessageContentChanged { chat_id, .. }
            | TdLibUpdate::DeleteMessages { chat_id, .. }
            | TdLibUpdate::ChatLastMessage { chat_id }
            | TdLibUpdate::ChatPosition { chat_id }
            | TdLibUpdate::ChatReadInbox { chat_id }
            | TdLibUpdate::ChatReadOutbox { chat_id }
            | TdLibUpdate::MessageSendSucceeded { chat_id, .. }
            | TdLibUpdate::ChatUnreadReactionCount { chat_id }
            | TdLibUpdate::MessageInteractionInfoChanged { chat_id, .. } => Some(*chat_id),
            TdLibUpdate::ChatAction { chat_id, .. } => Some(*chat_id),
            TdLibUpdate::UserStatus { .. } | TdLibUpdate::FileUpdated { .. } => None,
        }
    }

    /// Returns the kind of update for logging.
    pub fn kind(&self) -> &'static str {
        match self {
            TdLibUpdate::NewChat { .. } => "new_chat",
            TdLibUpdate::NewMessage { .. } => "new_message",
            TdLibUpdate::MessageContentChanged { .. } => "message_content",
            TdLibUpdate::DeleteMessages { .. } => "delete_messages",
            TdLibUpdate::ChatLastMessage { .. } => "chat_last_message",
            TdLibUpdate::ChatPosition { .. } => "chat_position",
            TdLibUpdate::ChatReadInbox { .. } => "chat_read_inbox",
            TdLibUpdate::ChatReadOutbox { .. } => "chat_read_outbox",
            TdLibUpdate::UserStatus { .. } => "user_status",
            TdLibUpdate::MessageSendSucceeded { .. } => "message_send_succeeded",
            TdLibUpdate::ChatUnreadReactionCount { .. } => "chat_unread_reaction_count",
            TdLibUpdate::MessageInteractionInfoChanged { .. } => "message_interaction_info_changed",
            TdLibUpdate::ChatAction { .. } => "chat_action",
            TdLibUpdate::FileUpdated { .. } => "file_updated",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_td_message(chat_id: i64) -> TdMessage {
        use tdlib_rs::enums::MessageSender;
        TdMessage {
            id: 1,
            sender_id: MessageSender::User(tdlib_rs::types::MessageSenderUser { user_id: 1 }),
            chat_id,
            sending_state: None,
            scheduling_state: None,
            is_outgoing: false,
            is_pinned: false,
            is_from_offline: false,
            can_be_saved: true,
            has_timestamped_media: false,
            is_channel_post: false,
            is_paid_star_suggested_post: false,
            is_paid_ton_suggested_post: false,
            contains_unread_mention: false,
            date: 1609459200,
            edit_date: 0,
            forward_info: None,
            import_info: None,
            interaction_info: None,
            unread_reactions: vec![],
            fact_check: None,
            suggested_post_info: None,
            reply_to: None,
            topic_id: None,
            self_destruct_type: None,
            self_destruct_in: 0.0,
            auto_delete_in: 0.0,
            via_bot_user_id: 0,
            sender_business_bot_user_id: 0,
            sender_boost_count: 0,
            paid_message_star_count: 0,
            author_signature: String::new(),
            media_album_id: 0,
            effect_id: 0,
            restriction_info: None,
            summary_language_code: String::new(),
            content: MessageContent::MessageText(tdlib_rs::types::MessageText {
                text: tdlib_rs::types::FormattedText {
                    text: "test".to_owned(),
                    entities: vec![],
                },
                link_preview: None,
                link_preview_options: None,
            }),
            reply_markup: None,
        }
    }

    #[test]
    fn new_message_has_chat_id() {
        let update = TdLibUpdate::NewMessage {
            chat_id: 123,
            message: Box::new(make_test_td_message(123)),
        };
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

    #[test]
    fn delete_messages_has_chat_id() {
        let update = TdLibUpdate::DeleteMessages {
            chat_id: 42,
            message_ids: vec![1, 2, 3],
        };
        assert_eq!(update.chat_id(), Some(42));
        assert_eq!(update.kind(), "delete_messages");
    }

    #[test]
    fn new_chat_has_chat_id() {
        let chat = super::super::tdlib_cache::tests::make_test_chat(77, "Test");
        let update = TdLibUpdate::NewChat {
            chat: Box::new(chat),
        };
        assert_eq!(update.chat_id(), Some(77));
        assert_eq!(update.kind(), "new_chat");
    }
}
