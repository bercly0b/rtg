use tdlib_rs::enums::MessageContent;
use tdlib_rs::types::Message as TdMessage;

use crate::domain::message::{Message, MessageMedia, ReplyInfo};

use super::file_info::{extract_call_info, extract_file_info};
use super::text_links::extract_content_links;

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
    let call_info = extract_call_info(&msg.content);
    let links = extract_content_links(&msg.content);
    let timestamp_ms = i64::from(msg.date) * 1000;
    let reaction_count = extract_total_reaction_count(msg);

    Message {
        id: msg.id,
        sender_name,
        text,
        timestamp_ms,
        is_outgoing: msg.is_outgoing,
        media,
        status: crate::domain::message::MessageStatus::Delivered,
        file_info,
        call_info,
        reply_to,
        reaction_count,
        links,
        is_edited: msg.edit_date > 0,
    }
}

/// Sums `total_count` across all reaction types from interaction info.
pub fn sum_reaction_counts(
    interaction_info: Option<&tdlib_rs::types::MessageInteractionInfo>,
) -> u32 {
    interaction_info
        .and_then(|info| info.reactions.as_ref())
        .map(|reactions| {
            reactions
                .reactions
                .iter()
                .map(|r| r.total_count.max(0) as u32)
                .sum()
        })
        .unwrap_or(0)
}

fn extract_total_reaction_count(msg: &TdMessage) -> u32 {
    sum_reaction_counts(msg.interaction_info.as_ref())
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
    my_user_id: Option<i64>,
) -> Option<ReplyInfo> {
    use tdlib_rs::enums::{MessageOrigin, MessageReplyTo};

    let reply_to = msg.reply_to.as_ref()?;

    let MessageReplyTo::Message(info) = reply_to else {
        return None;
    };

    let (sender_name, is_outgoing) = match info.origin.as_ref() {
        Some(MessageOrigin::User(u)) => {
            let outgoing = my_user_id.is_some_and(|me| me == u.sender_user_id);
            let name = if outgoing {
                "You".to_owned()
            } else {
                resolve_user_name(u.sender_user_id)
                    .unwrap_or_else(|| format!("User#{}", u.sender_user_id))
            };
            (name, outgoing)
        }
        Some(MessageOrigin::Chat(c)) => {
            let name =
                resolve_chat_title(c.sender_chat_id).unwrap_or_else(|| c.author_signature.clone());
            (name, false)
        }
        Some(MessageOrigin::HiddenUser(h)) => (h.sender_name.clone(), false),
        Some(MessageOrigin::Channel(ch)) => {
            let name =
                resolve_chat_title(ch.chat_id).unwrap_or_else(|| ch.author_signature.clone());
            (name, false)
        }
        None => (String::new(), false),
    };

    let text = if let Some(content) = info.content.as_ref() {
        extract_message_text(content)
    } else if let Some(quote) = info.quote.as_ref() {
        quote.text.text.clone()
    } else {
        String::new()
    };

    Some(ReplyInfo {
        sender_name,
        text,
        is_outgoing,
    })
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
        MessageContent::MessageAnimatedEmoji(_) => MessageMedia::AnimatedEmoji,
        MessageContent::MessageCall(c) => {
            if c.is_video {
                MessageMedia::VideoCall
            } else {
                MessageMedia::Call
            }
        }
        // Service messages and other types
        _ => MessageMedia::Other,
    }
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
        MessageContent::MessageAnimatedEmoji(e) => Some(format!("{} Animated Emoji", e.emoji)),
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
        MessageContent::MessageCall(c) => {
            use tdlib_rs::enums::CallDiscardReason as TdReason;
            let kind = if c.is_video { "Video call" } else { "Call" };
            let detail = match &c.discard_reason {
                TdReason::Missed => "Missed".to_owned(),
                TdReason::Declined => "Declined".to_owned(),
                _ if c.duration > 0 => crate::domain::message::format_duration(c.duration),
                _ => "Cancelled".to_owned(),
            };
            Some(format!("[{kind}] {detail}"))
        }
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
pub(super) fn normalize_preview_text(text: &str) -> Option<String> {
    let normalized: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
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
        MessageContent::MessageAnimatedEmoji(e) => e.emoji.clone(),
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
