//! Type mappers from TDLib types to RTG domain types.
//!
//! This module provides conversion functions that map TDLib's rich type system
//! to RTG's simplified domain types for UI rendering.

use tdlib_rs::enums::{ChatType as TdChatType, MessageContent, MessageSender, UserStatus};
use tdlib_rs::types::{Chat as TdChat, Message as TdMessage, User as TdUser};

use crate::domain::chat::{ChatSummary, ChatType, OutgoingReadStatus};

/// Maps a TDLib Chat to a domain ChatSummary.
///
/// This requires the full `Chat` object from TDLib. For sender name resolution
/// in group chats, an optional user lookup function can be provided.
pub fn map_chat_to_summary(
    chat: &TdChat,
    sender_name: Option<String>,
    is_sender_online: Option<bool>,
) -> ChatSummary {
    let chat_type = map_chat_type(&chat.r#type);
    let is_pinned = chat
        .positions
        .iter()
        .any(|pos| matches!(&pos.list, tdlib_rs::enums::ChatList::Main) && pos.is_pinned);

    let (last_message_preview, last_message_unix_ms, outgoing_status) =
        extract_last_message_info(chat, sender_name.is_some());

    // For private chats, is_online comes from the user's status
    // For groups/channels, is_online is None
    let is_online = match chat_type {
        ChatType::Private => is_sender_online,
        _ => None,
    };

    ChatSummary {
        chat_id: chat.id,
        title: chat.title.clone(),
        unread_count: chat.unread_count.max(0) as u32,
        last_message_preview,
        last_message_unix_ms,
        is_pinned,
        chat_type,
        last_message_sender: match chat_type {
            ChatType::Group | ChatType::Channel => sender_name,
            ChatType::Private => None, // Don't show sender name in private chats
        },
        is_online,
        outgoing_status,
    }
}

/// Maps TDLib ChatType to domain ChatType.
pub fn map_chat_type(td_type: &TdChatType) -> ChatType {
    match td_type {
        TdChatType::Private(_) | TdChatType::Secret(_) => ChatType::Private,
        TdChatType::BasicGroup(_) => ChatType::Group,
        TdChatType::Supergroup(sg) => {
            if sg.is_channel {
                ChatType::Channel
            } else {
                ChatType::Group
            }
        }
    }
}

/// Extracts last message info from a TDLib Chat.
///
/// Returns (preview_text, timestamp_ms, outgoing_status).
fn extract_last_message_info(
    chat: &TdChat,
    _is_group_chat: bool,
) -> (Option<String>, Option<i64>, OutgoingReadStatus) {
    let Some(ref msg) = chat.last_message else {
        return (None, None, OutgoingReadStatus::default());
    };

    let preview = extract_message_preview(&msg.content);
    let timestamp_ms = i64::from(msg.date) * 1000;

    // Determine if the last outgoing message was read
    let is_outgoing = msg.is_outgoing;
    let is_read = if is_outgoing {
        // Message is read if its ID is <= last_read_outbox_message_id
        msg.id <= chat.last_read_outbox_message_id
    } else {
        false
    };

    (
        preview,
        Some(timestamp_ms),
        OutgoingReadStatus {
            is_outgoing,
            is_read,
        },
    )
}

/// Extracts a text preview from message content.
pub fn extract_message_preview(content: &MessageContent) -> Option<String> {
    let text = match content {
        MessageContent::MessageText(t) => Some(t.text.text.clone()),
        MessageContent::MessagePhoto(p) => {
            let caption = &p.caption.text;
            if caption.is_empty() {
                Some("[Photo]".to_owned())
            } else {
                Some(format!("[Photo] {}", caption))
            }
        }
        MessageContent::MessageVideo(v) => {
            let caption = &v.caption.text;
            if caption.is_empty() {
                Some("[Video]".to_owned())
            } else {
                Some(format!("[Video] {}", caption))
            }
        }
        MessageContent::MessageVoiceNote(v) => {
            let caption = &v.caption.text;
            if caption.is_empty() {
                Some("[Voice]".to_owned())
            } else {
                Some(format!("[Voice] {}", caption))
            }
        }
        MessageContent::MessageVideoNote(_) => Some("[Video message]".to_owned()),
        MessageContent::MessageSticker(s) => Some(format!("{} Sticker", s.sticker.emoji)),
        MessageContent::MessageDocument(d) => {
            let name = &d.document.file_name;
            if name.is_empty() {
                Some("[Document]".to_owned())
            } else {
                Some(format!("[Document] {}", name))
            }
        }
        MessageContent::MessageAudio(a) => {
            let title = &a.audio.title;
            if title.is_empty() {
                Some("[Audio]".to_owned())
            } else {
                Some(format!("[Audio] {}", title))
            }
        }
        MessageContent::MessageAnimation(a) => {
            let caption = &a.caption.text;
            if caption.is_empty() {
                Some("[GIF]".to_owned())
            } else {
                Some(format!("[GIF] {}", caption))
            }
        }
        MessageContent::MessageContact(c) => Some(format!("[Contact] {}", c.contact.first_name)),
        MessageContent::MessageLocation(_) => Some("[Location]".to_owned()),
        MessageContent::MessagePoll(p) => Some(format!("[Poll] {}", p.poll.question.text)),
        MessageContent::MessageCall(_) => Some("[Call]".to_owned()),
        // Service messages
        MessageContent::MessageChatAddMembers(_) => Some("[Members added]".to_owned()),
        MessageContent::MessageChatJoinByLink => Some("[Joined via link]".to_owned()),
        MessageContent::MessageChatJoinByRequest => Some("[Joined by request]".to_owned()),
        MessageContent::MessageChatDeleteMember(_) => Some("[Member removed]".to_owned()),
        MessageContent::MessageChatChangeTitle(t) => {
            Some(format!("[Title changed to \"{}\"]", t.title))
        }
        MessageContent::MessageChatChangePhoto(_) => Some("[Photo changed]".to_owned()),
        MessageContent::MessageChatDeletePhoto => Some("[Photo removed]".to_owned()),
        MessageContent::MessagePinMessage(_) => Some("[Message pinned]".to_owned()),
        _ => Some("[Message]".to_owned()),
    };

    // Normalize whitespace
    text.and_then(|t| normalize_preview_text(&t))
}

/// Normalizes message preview text by collapsing whitespace.
fn normalize_preview_text(text: &str) -> Option<String> {
    let normalized: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

/// Extracts the sender name from a TDLib message.
#[allow(dead_code)] // Will be used in message loading (Phase 5.4)
pub fn extract_sender_name_from_message(
    msg: &TdMessage,
    users: &[(i64, TdUser)],
) -> Option<String> {
    match &msg.sender_id {
        MessageSender::User(u) => users
            .iter()
            .find(|(id, _)| *id == u.user_id)
            .map(|(_, user)| format_user_name(user)),
        MessageSender::Chat(_) => None, // For channel posts, we use chat title
    }
}

/// Formats a user's display name from TDLib User.
pub fn format_user_name(user: &TdUser) -> String {
    let first = user.first_name.trim();
    let last = user.last_name.trim();

    if last.is_empty() {
        first.to_owned()
    } else {
        format!("{} {}", first, last)
    }
}

/// Checks if a user is currently online based on their status.
pub fn is_user_online(status: &UserStatus) -> bool {
    matches!(status, UserStatus::Online(_))
}

/// Gets the user ID from a MessageSender, if it's a user.
pub fn get_sender_user_id(sender: &MessageSender) -> Option<i64> {
    match sender {
        MessageSender::User(u) => Some(u.user_id),
        MessageSender::Chat(_) => None,
    }
}

/// Gets the user ID for a private chat.
pub fn get_private_chat_user_id(chat_type: &TdChatType) -> Option<i64> {
    match chat_type {
        TdChatType::Private(p) => Some(p.user_id),
        TdChatType::Secret(s) => Some(s.user_id),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_preview_text_collapses_whitespace() {
        assert_eq!(
            normalize_preview_text("hello  world"),
            Some("hello world".to_owned())
        );
        assert_eq!(
            normalize_preview_text("  multiple   spaces  "),
            Some("multiple spaces".to_owned())
        );
    }

    #[test]
    fn normalize_preview_text_returns_none_for_empty() {
        assert_eq!(normalize_preview_text(""), None);
        assert_eq!(normalize_preview_text("   "), None);
    }

    #[test]
    fn format_user_name_handles_first_name_only() {
        let user = make_test_user("John", "");
        assert_eq!(format_user_name(&user), "John");
    }

    #[test]
    fn format_user_name_combines_first_and_last() {
        let user = make_test_user("John", "Doe");
        assert_eq!(format_user_name(&user), "John Doe");
    }

    /// Creates a minimal TdUser for testing.
    fn make_test_user(first_name: &str, last_name: &str) -> TdUser {
        TdUser {
            id: 1,
            first_name: first_name.to_owned(),
            last_name: last_name.to_owned(),
            usernames: None,
            phone_number: String::new(),
            status: UserStatus::Empty,
            profile_photo: None,
            accent_color_id: 0,
            background_custom_emoji_id: 0,
            upgraded_gift_colors: None,
            profile_accent_color_id: -1,
            profile_background_custom_emoji_id: 0,
            emoji_status: None,
            is_contact: false,
            is_mutual_contact: false,
            is_close_friend: false,
            verification_status: None,
            is_premium: false,
            is_support: false,
            restriction_info: None,
            active_story_state: None,
            restricts_new_chats: false,
            paid_message_star_count: 0,
            have_access: true,
            r#type: tdlib_rs::enums::UserType::Regular,
            language_code: String::new(),
            added_to_attachment_menu: false,
        }
    }

    #[test]
    fn is_user_online_detects_online_status() {
        assert!(is_user_online(&UserStatus::Online(Default::default())));
        assert!(!is_user_online(&UserStatus::Offline(Default::default())));
        assert!(!is_user_online(&UserStatus::Recently(Default::default())));
        assert!(!is_user_online(&UserStatus::Empty));
    }
}
