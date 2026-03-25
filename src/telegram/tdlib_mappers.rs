//! Type mappers from TDLib types to RTG domain types.
//!
//! This module provides conversion functions that map TDLib's rich type system
//! to RTG's simplified domain types for UI rendering.

use tdlib_rs::enums::{ChatType as TdChatType, MessageContent, MessageSender, UserStatus};
use tdlib_rs::types::{Chat as TdChat, Message as TdMessage, User as TdUser};

use crate::domain::chat::{ChatSummary, ChatType, OutgoingReadStatus};
use crate::domain::chat_subtitle::ChatSubtitle;
use crate::domain::message::{DownloadStatus, FileInfo, Message, MessageMedia, ReplyInfo};

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
        is_bot,
        outgoing_status,
        last_message_id,
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
) -> (Option<String>, Option<i64>, OutgoingReadStatus, Option<i64>) {
    let Some(ref msg) = chat.last_message else {
        return (None, None, OutgoingReadStatus::default(), None);
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
        Some(msg.id),
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

/// Maps a TDLib Message to a domain Message.
///
/// Requires the sender name to be resolved externally (via get_user or chat title).
/// Reply info is also resolved externally via resolver closures.
pub fn map_tdlib_message_to_domain(
    msg: &TdMessage,
    sender_name: String,
    reply_to: Option<ReplyInfo>,
) -> Message {
    let text = extract_message_text(&msg.content);
    let media = extract_message_media(&msg.content);
    let file_info = extract_file_info(&msg.content);
    let timestamp_ms = i64::from(msg.date) * 1000;

    Message {
        id: msg.id,
        sender_name,
        text,
        timestamp_ms,
        is_outgoing: msg.is_outgoing,
        media,
        status: crate::domain::message::MessageStatus::Delivered,
        file_info,
        reply_to,
    }
}

/// Extracts reply information from a TDLib Message.
///
/// Handles `MessageReplyTo::Message` variant, extracting sender name from
/// `origin` and text from `content`. Story replies are ignored.
///
/// `resolve_user_name` resolves a user ID to a display name via cache lookup.
/// `resolve_chat_title` resolves a chat ID to a chat title via cache lookup.
pub fn extract_reply_info(
    msg: &TdMessage,
    resolve_user_name: impl Fn(i64) -> Option<String>,
    resolve_chat_title: impl Fn(i64) -> Option<String>,
) -> Option<ReplyInfo> {
    use tdlib_rs::enums::{MessageOrigin, MessageReplyTo};

    let reply_to = msg.reply_to.as_ref()?;

    let MessageReplyTo::Message(info) = reply_to else {
        return None;
    };

    let sender_name = match info.origin.as_ref() {
        Some(MessageOrigin::User(u)) => resolve_user_name(u.sender_user_id)
            .unwrap_or_else(|| format!("User#{}", u.sender_user_id)),
        Some(MessageOrigin::Chat(c)) => {
            resolve_chat_title(c.sender_chat_id).unwrap_or_else(|| c.author_signature.clone())
        }
        Some(MessageOrigin::HiddenUser(h)) => h.sender_name.clone(),
        Some(MessageOrigin::Channel(ch)) => {
            resolve_chat_title(ch.chat_id).unwrap_or_else(|| ch.author_signature.clone())
        }
        None => String::new(),
    };

    let text = if let Some(content) = info.content.as_ref() {
        extract_message_text(content)
    } else if let Some(quote) = info.quote.as_ref() {
        quote.text.text.clone()
    } else {
        String::new()
    };

    Some(ReplyInfo { sender_name, text })
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

/// Extra metadata extracted alongside the TDLib `File` reference.
struct FileMetadata {
    mime: String,
    duration: Option<i32>,
    file_name: Option<String>,
    is_listened: bool,
}

/// Extracts file metadata from a TDLib MessageContent, if it carries a downloadable file.
///
/// Returns `Some(FileInfo)` for media types that have a file (voice, audio, video, document,
/// photo, etc.) and `None` for text, polls, contacts, locations, and service messages.
pub fn extract_file_info(content: &MessageContent) -> Option<FileInfo> {
    match content {
        MessageContent::MessagePhoto(p) => extract_photo_file_info(p),
        _ => extract_single_file_info(content),
    }
}

/// Extracts file info for media types that carry a single `File`.
fn extract_single_file_info(content: &MessageContent) -> Option<FileInfo> {
    let (file, meta) = match content {
        MessageContent::MessageVoiceNote(v) => (
            &v.voice_note.voice,
            FileMetadata {
                mime: v.voice_note.mime_type.clone(),
                duration: Some(v.voice_note.duration),
                file_name: None,
                is_listened: v.is_listened,
            },
        ),
        MessageContent::MessageAudio(a) => (
            &a.audio.audio,
            FileMetadata {
                mime: a.audio.mime_type.clone(),
                duration: Some(a.audio.duration),
                file_name: Some(a.audio.file_name.clone()).filter(|s| !s.is_empty()),
                is_listened: false,
            },
        ),
        MessageContent::MessageDocument(d) => (
            &d.document.document,
            FileMetadata {
                mime: d.document.mime_type.clone(),
                duration: None,
                file_name: Some(d.document.file_name.clone()).filter(|s| !s.is_empty()),
                is_listened: false,
            },
        ),
        MessageContent::MessageVideo(v) => (
            &v.video.video,
            FileMetadata {
                mime: v.video.mime_type.clone(),
                duration: Some(v.video.duration),
                file_name: None,
                is_listened: false,
            },
        ),
        MessageContent::MessageVideoNote(v) => (
            &v.video_note.video,
            FileMetadata {
                mime: "video/mp4".to_owned(),
                duration: Some(v.video_note.duration),
                file_name: None,
                is_listened: v.is_viewed,
            },
        ),
        MessageContent::MessageAnimation(a) => (
            &a.animation.animation,
            FileMetadata {
                mime: a.animation.mime_type.clone(),
                duration: Some(a.animation.duration),
                file_name: None,
                is_listened: false,
            },
        ),
        _ => return None,
    };

    Some(build_file_info(file, meta))
}

/// Extracts file info from a photo message by selecting the largest PhotoSize.
fn extract_photo_file_info(p: &tdlib_rs::types::MessagePhoto) -> Option<FileInfo> {
    let largest = p.photo.sizes.iter().max_by_key(|s| s.width * s.height)?;
    let file = &largest.photo;
    let meta = FileMetadata {
        // TDLib PhotoSize doesn't expose MIME type; JPEG is the most common format.
        mime: "image/jpeg".to_owned(),
        duration: None,
        file_name: None,
        is_listened: false,
    };
    Some(build_file_info(file, meta))
}

/// Builds a `FileInfo` from a TDLib `File` and extracted metadata.
fn build_file_info(file: &tdlib_rs::types::File, meta: FileMetadata) -> FileInfo {
    let is_completed = file.local.is_downloading_completed && !file.local.path.is_empty();
    let local_path = if is_completed {
        Some(file.local.path.clone())
    } else {
        None
    };

    let download_status = if is_completed {
        DownloadStatus::Completed
    } else if file.local.is_downloading_active {
        let total = effective_file_size(file);
        let percent = if total > 0 {
            ((file.local.downloaded_size as u64) * 100 / total).min(99) as u8
        } else {
            0
        };
        DownloadStatus::Downloading {
            progress_percent: percent,
        }
    } else {
        DownloadStatus::NotStarted
    };

    let size = {
        let s = effective_file_size(file);
        if s > 0 {
            Some(s)
        } else {
            None
        }
    };

    FileInfo {
        file_id: file.id,
        local_path,
        mime_type: meta.mime,
        size,
        duration: meta.duration,
        file_name: meta.file_name,
        is_listened: meta.is_listened,
        download_status,
    }
}

/// Returns the best known file size from TDLib's `File` struct.
///
/// Guards against negative sentinel values from TDLib by clamping to 0.
fn effective_file_size(file: &tdlib_rs::types::File) -> u64 {
    let size = file.size.max(0) as u64;
    if size > 0 {
        size
    } else {
        file.expected_size.max(0) as u64
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
    fn map_user_status_to_subtitle_online() {
        assert_eq!(
            map_user_status_to_subtitle(&UserStatus::Online(Default::default())),
            ChatSubtitle::Online
        );
    }

    #[test]
    fn map_user_status_to_subtitle_offline() {
        let offline = tdlib_rs::types::UserStatusOffline {
            was_online: 1234567,
        };
        assert_eq!(
            map_user_status_to_subtitle(&UserStatus::Offline(offline)),
            ChatSubtitle::LastSeen(1234567)
        );
    }

    #[test]
    fn map_user_status_to_subtitle_recently() {
        assert_eq!(
            map_user_status_to_subtitle(&UserStatus::Recently(Default::default())),
            ChatSubtitle::Recently
        );
    }

    #[test]
    fn map_user_status_to_subtitle_last_week() {
        assert_eq!(
            map_user_status_to_subtitle(&UserStatus::LastWeek(Default::default())),
            ChatSubtitle::WithinWeek
        );
    }

    #[test]
    fn map_user_status_to_subtitle_last_month() {
        assert_eq!(
            map_user_status_to_subtitle(&UserStatus::LastMonth(Default::default())),
            ChatSubtitle::WithinMonth
        );
    }

    #[test]
    fn map_user_status_to_subtitle_empty() {
        assert_eq!(
            map_user_status_to_subtitle(&UserStatus::Empty),
            ChatSubtitle::LongTimeAgo
        );
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
        let message = map_tdlib_message_to_domain(&td_message, "John Doe".to_owned(), None);

        assert_eq!(message.id, 123);
        assert_eq!(message.sender_name, "John Doe");
        assert_eq!(message.text, "Hello from TDLib");
        assert!(!message.is_outgoing);
        assert_eq!(message.media, MessageMedia::None);
    }

    #[test]
    fn map_tdlib_message_to_domain_handles_outgoing() {
        let td_message = make_test_message(456, "My message", true);
        let message = map_tdlib_message_to_domain(&td_message, "Me".to_owned(), None);

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

    fn make_test_file(id: i32, path: &str, downloaded: bool) -> tdlib_rs::types::File {
        tdlib_rs::types::File {
            id,
            size: 1000,
            expected_size: 1000,
            local: tdlib_rs::types::LocalFile {
                path: path.to_owned(),
                can_be_downloaded: true,
                can_be_deleted: false,
                is_downloading_active: false,
                is_downloading_completed: downloaded,
                download_offset: 0,
                downloaded_prefix_size: 0,
                downloaded_size: if downloaded { 1000 } else { 0 },
            },
            remote: tdlib_rs::types::RemoteFile {
                id: String::new(),
                unique_id: String::new(),
                is_uploading_active: false,
                is_uploading_completed: false,
                uploaded_size: 0,
            },
        }
    }

    #[test]
    fn extract_file_info_returns_none_for_text_message() {
        let content = MessageContent::MessageText(tdlib_rs::types::MessageText {
            text: tdlib_rs::types::FormattedText {
                text: "hello".to_owned(),
                entities: vec![],
            },
            link_preview: None,
            link_preview_options: None,
        });
        assert!(extract_file_info(&content).is_none());
    }

    #[test]
    fn extract_file_info_for_downloaded_voice_note() {
        let content = MessageContent::MessageVoiceNote(tdlib_rs::types::MessageVoiceNote {
            voice_note: tdlib_rs::types::VoiceNote {
                duration: 5,
                waveform: String::new(),
                mime_type: "audio/ogg".to_owned(),
                speech_recognition_result: None,
                voice: make_test_file(42, "/tmp/voice.ogg", true),
            },
            caption: tdlib_rs::types::FormattedText {
                text: String::new(),
                entities: vec![],
            },
            is_listened: false,
        });

        let fi = extract_file_info(&content).expect("should have file info");
        assert_eq!(fi.file_id, 42);
        assert_eq!(fi.local_path, Some("/tmp/voice.ogg".to_owned()));
        assert_eq!(fi.mime_type, "audio/ogg");
    }

    #[test]
    fn extract_file_info_for_not_downloaded_voice_note() {
        let content = MessageContent::MessageVoiceNote(tdlib_rs::types::MessageVoiceNote {
            voice_note: tdlib_rs::types::VoiceNote {
                duration: 5,
                waveform: String::new(),
                mime_type: "audio/ogg".to_owned(),
                speech_recognition_result: None,
                voice: make_test_file(42, "", false),
            },
            caption: tdlib_rs::types::FormattedText {
                text: String::new(),
                entities: vec![],
            },
            is_listened: false,
        });

        let fi = extract_file_info(&content).expect("should have file info");
        assert_eq!(fi.file_id, 42);
        assert!(fi.local_path.is_none());
        assert_eq!(fi.mime_type, "audio/ogg");
    }

    #[test]
    fn extract_file_info_for_downloaded_audio() {
        let content = MessageContent::MessageAudio(tdlib_rs::types::MessageAudio {
            audio: tdlib_rs::types::Audio {
                duration: 180,
                title: "Song".to_owned(),
                performer: "Artist".to_owned(),
                file_name: "song.mp3".to_owned(),
                mime_type: "audio/mpeg".to_owned(),
                album_cover_minithumbnail: None,
                album_cover_thumbnail: None,
                external_album_covers: vec![],
                audio: make_test_file(99, "/tmp/song.mp3", true),
            },
            caption: tdlib_rs::types::FormattedText {
                text: String::new(),
                entities: vec![],
            },
        });

        let fi = extract_file_info(&content).expect("should have file info");
        assert_eq!(fi.file_id, 99);
        assert_eq!(fi.local_path, Some("/tmp/song.mp3".to_owned()));
        assert_eq!(fi.mime_type, "audio/mpeg");
    }

    #[test]
    fn extract_file_info_returns_none_for_photo() {
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
        assert!(extract_file_info(&content).is_none());
    }

    #[test]
    fn extract_file_info_for_photo_with_sizes() {
        let content = MessageContent::MessagePhoto(tdlib_rs::types::MessagePhoto {
            photo: tdlib_rs::types::Photo {
                minithumbnail: None,
                sizes: vec![
                    tdlib_rs::types::PhotoSize {
                        r#type: "s".to_owned(),
                        photo: make_test_file(10, "", false),
                        width: 100,
                        height: 100,
                        progressive_sizes: vec![],
                    },
                    tdlib_rs::types::PhotoSize {
                        r#type: "m".to_owned(),
                        photo: make_test_file(20, "/tmp/photo.jpg", true),
                        width: 800,
                        height: 600,
                        progressive_sizes: vec![],
                    },
                ],
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

        let fi = extract_file_info(&content).expect("photo with sizes should have file_info");
        assert_eq!(fi.file_id, 20, "should select the largest photo size");
        assert_eq!(fi.local_path, Some("/tmp/photo.jpg".to_owned()));
        assert_eq!(fi.mime_type, "image/jpeg");
        assert_eq!(fi.download_status, DownloadStatus::Completed);
    }

    #[test]
    fn extract_file_info_includes_duration_for_voice() {
        let content = MessageContent::MessageVoiceNote(tdlib_rs::types::MessageVoiceNote {
            voice_note: tdlib_rs::types::VoiceNote {
                duration: 42,
                waveform: String::new(),
                mime_type: "audio/ogg".to_owned(),
                speech_recognition_result: None,
                voice: make_test_file(1, "/tmp/v.ogg", true),
            },
            caption: tdlib_rs::types::FormattedText {
                text: String::new(),
                entities: vec![],
            },
            is_listened: true,
        });

        let fi = extract_file_info(&content).expect("should have file info");
        assert_eq!(fi.duration, Some(42));
        assert!(fi.is_listened);
        assert_eq!(fi.size, Some(1000));
        assert_eq!(fi.download_status, DownloadStatus::Completed);
    }

    #[test]
    fn extract_file_info_includes_file_name_for_document() {
        let content = MessageContent::MessageDocument(tdlib_rs::types::MessageDocument {
            document: tdlib_rs::types::Document {
                file_name: "report.pdf".to_owned(),
                mime_type: "application/pdf".to_owned(),
                minithumbnail: None,
                thumbnail: None,
                document: make_test_file(5, "", false),
            },
            caption: tdlib_rs::types::FormattedText {
                text: String::new(),
                entities: vec![],
            },
        });

        let fi = extract_file_info(&content).expect("should have file info");
        assert_eq!(fi.file_name, Some("report.pdf".to_owned()));
        assert_eq!(fi.duration, None);
        assert_eq!(fi.download_status, DownloadStatus::NotStarted);
    }

    #[test]
    fn map_tdlib_message_includes_file_info() {
        let mut td_msg = make_test_message(1, "", false);
        td_msg.content = MessageContent::MessageVoiceNote(tdlib_rs::types::MessageVoiceNote {
            voice_note: tdlib_rs::types::VoiceNote {
                duration: 3,
                waveform: String::new(),
                mime_type: "audio/ogg".to_owned(),
                speech_recognition_result: None,
                voice: make_test_file(7, "/tmp/v.ogg", true),
            },
            caption: tdlib_rs::types::FormattedText {
                text: String::new(),
                entities: vec![],
            },
            is_listened: false,
        });

        let msg = map_tdlib_message_to_domain(&td_msg, "User".to_owned(), None);
        assert_eq!(msg.media, MessageMedia::Voice);
        let fi = msg.file_info.expect("voice message should have file_info");
        assert_eq!(fi.file_id, 7);
        assert_eq!(fi.local_path, Some("/tmp/v.ogg".to_owned()));
    }
}
