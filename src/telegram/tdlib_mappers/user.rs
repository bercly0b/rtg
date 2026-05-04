use tdlib_rs::enums::{ChatType as TdChatType, MessageSender, UserStatus};
use tdlib_rs::types::User as TdUser;

use crate::domain::chat_subtitle::ChatSubtitle;

/// Formats a user's display name from TDLib User.
pub fn format_user_name(user: &TdUser) -> String {
    let first = user.first_name.trim();
    let last = user.last_name.trim();

    match (first.is_empty(), last.is_empty()) {
        (true, true) => "Deleted".to_owned(),
        (_, true) => first.to_owned(),
        _ => format!("{} {}", first, last),
    }
}

/// Checks if a user is currently online based on their status.
pub fn is_user_online(status: &UserStatus) -> bool {
    matches!(status, UserStatus::Online(_))
}

/// Maps a TDLib `UserStatus` to a domain `ChatSubtitle`.
pub fn map_user_status_to_subtitle(status: &UserStatus) -> ChatSubtitle {
    match status {
        UserStatus::Online(_) => ChatSubtitle::Online,
        UserStatus::Offline(o) => ChatSubtitle::LastSeen(o.was_online),
        UserStatus::Recently(_) => ChatSubtitle::Recently,
        UserStatus::LastWeek(_) => ChatSubtitle::WithinWeek,
        UserStatus::LastMonth(_) => ChatSubtitle::WithinMonth,
        UserStatus::Empty => ChatSubtitle::LongTimeAgo,
    }
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
