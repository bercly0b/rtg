use tdlib_rs::enums::ChatType as TdChatType;
use tdlib_rs::types::Chat as TdChat;

use crate::domain::chat::{ChatSummary, ChatType, OutgoingReadStatus};

use super::extract_message_preview;

/// Maps a TDLib Chat to a domain ChatSummary.
///
/// This requires the full `Chat` object from TDLib. For sender name resolution
/// in group chats, an optional user lookup function can be provided.
pub fn map_chat_to_summary(
    chat: &TdChat,
    sender_name: Option<String>,
    is_sender_online: Option<bool>,
    is_bot: bool,
) -> ChatSummary {
    let chat_type = map_chat_type(&chat.r#type);
    let is_pinned = chat
        .positions
        .iter()
        .any(|pos| matches!(&pos.list, tdlib_rs::enums::ChatList::Main) && pos.is_pinned);

    let (last_message_preview, last_message_unix_ms, outgoing_status, last_message_id) =
        extract_last_message_info(chat, sender_name.is_some());

    // For private chats, is_online comes from the user's status
    // For groups/channels, is_online is None
    let is_online = match chat_type {
        ChatType::Private => is_sender_online,
        _ => None,
    };

    let title = if chat.title.trim().is_empty() {
        "Deleted".to_owned()
    } else {
        chat.title.clone()
    };

    ChatSummary {
        chat_id: chat.id,
        title,
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
        is_bot,
        outgoing_status,
        last_message_id,
        unread_reaction_count: chat.unread_reaction_count.max(0) as u32,
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
/// Returns (preview_text, timestamp_ms, outgoing_status, last_message_id).
fn extract_last_message_info(
    chat: &TdChat,
    _is_group_chat: bool,
) -> (Option<String>, Option<i64>, OutgoingReadStatus, Option<i64>) {
    let Some(ref msg) = chat.last_message else {
        return (None, None, OutgoingReadStatus::default(), None);
    };

    let preview = extract_message_preview(&msg.content);
    let timestamp_ms = i64::from(msg.date) * 1000;

    let outgoing_status = if msg.is_outgoing {
        // Message is read if its ID is <= last_read_outbox_message_id
        OutgoingReadStatus::Outgoing {
            is_read: msg.id <= chat.last_read_outbox_message_id,
        }
    } else {
        OutgoingReadStatus::NotOutgoing
    };

    (preview, Some(timestamp_ms), outgoing_status, Some(msg.id))
}
