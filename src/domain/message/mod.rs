mod call_info;
mod file_info;
mod url;

#[cfg(test)]
mod tests;

pub use call_info::{build_call_metadata_display, CallDiscardReason, CallInfo};
#[allow(unused_imports, reason = "some re-exports are used only by tests")]
pub use file_info::{
    build_file_metadata_display, file_extension, format_duration, format_file_size, DownloadStatus,
    FileInfo,
};
pub use url::{open_targets, LinkTarget, TextLink};

/// Type of media attached to a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MessageMedia {
    #[default]
    None,
    Photo,
    Voice,
    Video,
    VideoNote,
    Sticker,
    Document,
    Audio,
    Animation,
    Contact,
    Location,
    Poll,
    Call,
    VideoCall,
    Other,
}

impl MessageMedia {
    /// Returns a display label for the media type, or None if no media.
    pub fn display_label(&self) -> Option<&'static str> {
        match self {
            MessageMedia::None => None,
            MessageMedia::Photo => Some("[Photo]"),
            MessageMedia::Voice => Some("[Voice]"),
            MessageMedia::Video => Some("[Video]"),
            MessageMedia::VideoNote => Some("[Video message]"),
            MessageMedia::Sticker => None,
            MessageMedia::Document => Some("[Document]"),
            MessageMedia::Audio => Some("[Audio]"),
            MessageMedia::Animation => Some("[GIF]"),
            MessageMedia::Contact => Some("[Contact]"),
            MessageMedia::Location => Some("[Location]"),
            MessageMedia::Poll => Some("[Poll]"),
            MessageMedia::Call => Some("[Call]"),
            MessageMedia::VideoCall => Some("[Video call]"),
            MessageMedia::Other => Some("[Unsupported message]"),
        }
    }
}

/// Information about the message being replied to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplyInfo {
    /// Display name of the original message sender.
    pub sender_name: String,
    /// Text preview of the original message.
    pub text: String,
    /// Whether the replied-to message was sent by the current user.
    pub is_outgoing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForwardInfo {
    pub sender_name: String,
}

/// Who sent a message into a chat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageSender {
    User(i64),
    /// A chat itself: a channel for its posts, a group for an anonymous admin.
    Chat(i64),
}

/// Delivery status of a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MessageStatus {
    /// Message has been delivered (normal state).
    #[default]
    Delivered,
    /// Message is being sent (optimistic display).
    Sending,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub id: i64,
    /// `None` only for the optimistically displayed message.
    pub sender: Option<MessageSender>,
    pub sender_name: String,
    pub text: String,
    pub timestamp_ms: i64,
    pub is_outgoing: bool,
    pub media: MessageMedia,
    pub status: MessageStatus,
    /// File metadata for messages that carry downloadable media.
    /// `None` for text-only, poll, contact, location, and other non-file types.
    pub file_info: Option<FileInfo>,
    /// Call metadata for `MessageMedia::Call` messages.
    pub call_info: Option<CallInfo>,
    /// Information about the message this message replies to.
    /// `None` if the message is not a reply.
    pub reply_to: Option<ReplyInfo>,
    pub forward_info: Option<ForwardInfo>,
    /// Total number of reactions on this message (summed across all reaction types).
    pub reaction_count: u32,
    /// Links and mentions embedded in the message text via text entities.
    pub links: Vec<TextLink>,
    /// Whether the message has been edited after sending.
    pub is_edited: bool,
    /// Whether this is a service/system message (member added, photo changed, etc.).
    pub is_service: bool,
}

impl Message {
    /// Returns the display content: media label + text, or just text if no media.
    pub fn display_content(&self) -> String {
        match (self.media.display_label(), self.text.is_empty()) {
            (Some(label), true) => label.to_owned(),
            (Some(label), false) => format!("{}\n{}", label, self.text),
            (None, _) => self.text.clone(),
        }
    }

    /// Whether Telegram has not accepted this message yet: neither the
    /// optimistically displayed one nor TDLib's own yet unsent one can be
    /// replied to, edited or reacted to.
    pub fn is_pending(&self) -> bool {
        self.status == MessageStatus::Sending
    }

    /// Whether this is the optimistically displayed message: its id is local
    /// and must not reach TDLib, unlike the id of TDLib's yet unsent message.
    pub fn is_optimistic(&self) -> bool {
        self.id < 0
    }
}

/// Sets the upload progress of the messages being sent with `file_id`.
pub(super) fn set_upload_progress(
    messages: &mut [Message],
    file_id: i32,
    progress_percent: Option<u8>,
) {
    for file_info in messages
        .iter_mut()
        .filter(|message| message.is_pending())
        .filter_map(|message| message.file_info.as_mut())
        .filter(|file_info| file_info.file_id == file_id)
    {
        file_info.upload_progress_percent = progress_percent;
    }
}
