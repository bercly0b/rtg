//! Type mappers from TDLib types to RTG domain types.
//!
//! This module provides conversion functions that map TDLib's rich type system
//! to RTG's simplified domain types for UI rendering.

use tdlib_rs::enums::{ChatType as TdChatType, MessageContent, MessageSender, UserStatus};
use tdlib_rs::types::{Chat as TdChat, Message as TdMessage, User as TdUser};

use crate::domain::chat::{ChatSummary, ChatType, OutgoingReadStatus};
use crate::domain::message::{Message, MessageMedia};

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

/// Extracts the sender name from a TDLib message using a pre-resolved user list.
#[allow(dead_code)]
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

/// Maps a TDLib Message to a domain Message.
///
/// Requires the sender name to be resolved externally (via get_user or chat title).
pub fn map_tdlib_message_to_domain(msg: &TdMessage, sender_name: String) -> Message {
    let text = extract_message_text(&msg.content);
    let media = extract_message_media(&msg.content);
    let timestamp_ms = i64::from(msg.date) * 1000;

    Message {
        id: msg.id,
        sender_name,
        text,
        timestamp_ms,
        is_outgoing: msg.is_outgoing,
        media,
    }
}

/// Extracts the media type from a TDLib MessageContent.
pub fn extract_message_media(content: &MessageContent) -> MessageMedia {
    match content {
        MessageContent::MessageText(_) => MessageMedia::None,
        MessageContent::MessagePhoto(_) => MessageMedia::Photo,
        MessageContent::MessageVoiceNote(_) => MessageMedia::Voice,
        MessageContent::MessageVideo(_) => MessageMedia::Video,
        MessageContent::MessageVideoNote(_) => MessageMedia::VideoNote,
        MessageContent::MessageSticker(_) => MessageMedia::Sticker,
        MessageContent::MessageDocument(_) => MessageMedia::Document,
        MessageContent::MessageAudio(_) => MessageMedia::Audio,
        MessageContent::MessageAnimation(_) => MessageMedia::Animation,
        MessageContent::MessageContact(_) => MessageMedia::Contact,
        MessageContent::MessageLocation(_) | MessageContent::MessageVenue(_) => {
            MessageMedia::Location
        }
        MessageContent::MessagePoll(_) => MessageMedia::Poll,
        // Service messages and other types
        _ => MessageMedia::Other,
    }
}

/// Extracts the text content from a TDLib MessageContent.
///
/// For text messages, returns the message text.
/// For media messages with captions, returns the caption.
/// For service messages, returns an empty string.
pub fn extract_message_text(content: &MessageContent) -> String {
    match content {
        MessageContent::MessageText(t) => t.text.text.clone(),
        MessageContent::MessagePhoto(p) => p.caption.text.clone(),
        MessageContent::MessageVideo(v) => v.caption.text.clone(),
        MessageContent::MessageVoiceNote(v) => v.caption.text.clone(),
        MessageContent::MessageDocument(d) => d.caption.text.clone(),
        MessageContent::MessageAudio(a) => a.caption.text.clone(),
        MessageContent::MessageAnimation(a) => a.caption.text.clone(),
        // These types don't have captions or text
        MessageContent::MessageVideoNote(_)
        | MessageContent::MessageSticker(_)
        | MessageContent::MessageContact(_)
        | MessageContent::MessageLocation(_)
        | MessageContent::MessageVenue(_)
        | MessageContent::MessagePoll(_) => String::new(),
        // Service messages and other types
        _ => String::new(),
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

    #[test]
    fn extract_message_media_identifies_text_as_none() {
        let content = MessageContent::MessageText(tdlib_rs::types::MessageText {
            text: tdlib_rs::types::FormattedText {
                text: "Hello".to_owned(),
                entities: vec![],
            },
            link_preview: None,
            link_preview_options: None,
        });
        assert_eq!(extract_message_media(&content), MessageMedia::None);
    }

    #[test]
    fn extract_message_media_identifies_photo() {
        let content = MessageContent::MessagePhoto(tdlib_rs::types::MessagePhoto {
            photo: tdlib_rs::types::Photo {
                minithumbnail: None,
                sizes: vec![],
                has_stickers: false,
            },
            caption: tdlib_rs::types::FormattedText {
                text: String::new(),
                entities: vec![],
            },
            show_caption_above_media: false,
            has_spoiler: false,
            is_secret: false,
        });
        assert_eq!(extract_message_media(&content), MessageMedia::Photo);
    }

    #[test]
    fn extract_message_media_identifies_voice() {
        let content = MessageContent::MessageVoiceNote(tdlib_rs::types::MessageVoiceNote {
            voice_note: tdlib_rs::types::VoiceNote {
                duration: 10,
                waveform: String::new(),
                mime_type: "audio/ogg".to_owned(),
                speech_recognition_result: None,
                voice: tdlib_rs::types::File {
                    id: 1,
                    size: 1000,
                    expected_size: 1000,
                    local: tdlib_rs::types::LocalFile {
                        path: String::new(),
                        can_be_downloaded: false,
                        can_be_deleted: false,
                        is_downloading_active: false,
                        is_downloading_completed: false,
                        download_offset: 0,
                        downloaded_prefix_size: 0,
                        downloaded_size: 0,
                    },
                    remote: tdlib_rs::types::RemoteFile {
                        id: String::new(),
                        unique_id: String::new(),
                        is_uploading_active: false,
                        is_uploading_completed: false,
                        uploaded_size: 0,
                    },
                },
            },
            caption: tdlib_rs::types::FormattedText {
                text: String::new(),
                entities: vec![],
            },
            is_listened: false,
        });
        assert_eq!(extract_message_media(&content), MessageMedia::Voice);
    }

    #[test]
    fn extract_message_text_returns_text_from_message_text() {
        let content = MessageContent::MessageText(tdlib_rs::types::MessageText {
            text: tdlib_rs::types::FormattedText {
                text: "Hello, world!".to_owned(),
                entities: vec![],
            },
            link_preview: None,
            link_preview_options: None,
        });
        assert_eq!(extract_message_text(&content), "Hello, world!");
    }

    #[test]
    fn extract_message_text_returns_caption_from_photo() {
        let content = MessageContent::MessagePhoto(tdlib_rs::types::MessagePhoto {
            photo: tdlib_rs::types::Photo {
                minithumbnail: None,
                sizes: vec![],
                has_stickers: false,
            },
            caption: tdlib_rs::types::FormattedText {
                text: "Photo caption".to_owned(),
                entities: vec![],
            },
            show_caption_above_media: false,
            has_spoiler: false,
            is_secret: false,
        });
        assert_eq!(extract_message_text(&content), "Photo caption");
    }

    #[test]
    fn map_tdlib_message_to_domain_creates_correct_message() {
        let td_message = make_test_message(123, "Hello from TDLib", false);
        let message = map_tdlib_message_to_domain(&td_message, "John Doe".to_owned());

        assert_eq!(message.id, 123);
        assert_eq!(message.sender_name, "John Doe");
        assert_eq!(message.text, "Hello from TDLib");
        assert!(!message.is_outgoing);
        assert_eq!(message.media, MessageMedia::None);
    }

    #[test]
    fn map_tdlib_message_to_domain_handles_outgoing() {
        let td_message = make_test_message(456, "My message", true);
        let message = map_tdlib_message_to_domain(&td_message, "Me".to_owned());

        assert!(message.is_outgoing);
    }

    /// Creates a minimal TdMessage for testing.
    fn make_test_message(id: i64, text: &str, is_outgoing: bool) -> TdMessage {
        TdMessage {
            id,
            sender_id: MessageSender::User(tdlib_rs::types::MessageSenderUser { user_id: 1 }),
            chat_id: 100,
            sending_state: None,
            scheduling_state: None,
            is_outgoing,
            is_pinned: false,
            is_from_offline: false,
            can_be_saved: true,
            has_timestamped_media: false,
            is_channel_post: false,
            is_paid_star_suggested_post: false,
            is_paid_ton_suggested_post: false,
            contains_unread_mention: false,
            date: 1609459200, // 2021-01-01 00:00:00 UTC
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
                    text: text.to_owned(),
                    entities: vec![],
                },
                link_preview: None,
                link_preview_options: None,
            }),
            reply_markup: None,
        }
    }
}
