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
            MessageMedia::Sticker => Some("[Sticker]"),
            MessageMedia::Document => Some("[Document]"),
            MessageMedia::Audio => Some("[Audio]"),
            MessageMedia::Animation => Some("[GIF]"),
            MessageMedia::Contact => Some("[Contact]"),
            MessageMedia::Location => Some("[Location]"),
            MessageMedia::Poll => Some("[Poll]"),
            MessageMedia::Other => Some("[Media]"),
        }
    }
}

/// File metadata for messages with downloadable media.
///
/// Provides the information needed to open/play a file: its local path
/// (if already downloaded) and MIME type (for handler lookup).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileInfo {
    /// TDLib file identifier, used for download requests.
    pub file_id: i32,
    /// Local filesystem path; `None` if the file has not been downloaded yet.
    pub local_path: Option<String>,
    /// MIME type reported by TDLib (e.g. `"audio/ogg"`, `"video/mp4"`).
    pub mime_type: String,
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
    pub sender_name: String,
    pub text: String,
    pub timestamp_ms: i64,
    pub is_outgoing: bool,
    pub media: MessageMedia,
    pub status: MessageStatus,
    /// File metadata for messages that carry downloadable media.
    /// `None` for text-only, poll, contact, location, and other non-file types.
    pub file_info: Option<FileInfo>,
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
}

/// Extracts the first URL (`http://` or `https://`) from text.
///
/// Uses simple whitespace-delimited scanning — no regex dependency required.
pub fn extract_first_url(text: &str) -> Option<&str> {
    text.split_whitespace()
        .find(|word| word.starts_with("https://") || word.starts_with("http://"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(text: &str, media: MessageMedia) -> Message {
        Message {
            id: 1,
            sender_name: "User".to_owned(),
            text: text.to_owned(),
            timestamp_ms: 1000,
            is_outgoing: false,
            media,
            status: MessageStatus::Delivered,
            file_info: None,
        }
    }

    #[test]
    fn display_label_returns_none_for_no_media() {
        assert_eq!(MessageMedia::None.display_label(), None);
    }

    #[test]
    fn display_label_returns_photo_indicator() {
        assert_eq!(MessageMedia::Photo.display_label(), Some("[Photo]"));
    }

    #[test]
    fn display_label_returns_voice_indicator() {
        assert_eq!(MessageMedia::Voice.display_label(), Some("[Voice]"));
    }

    #[test]
    fn display_label_returns_sticker_indicator() {
        assert_eq!(MessageMedia::Sticker.display_label(), Some("[Sticker]"));
    }

    #[test]
    fn display_label_returns_video_note_indicator() {
        assert_eq!(
            MessageMedia::VideoNote.display_label(),
            Some("[Video message]")
        );
    }

    #[test]
    fn display_content_returns_text_only_when_no_media() {
        let message = msg("Hello world", MessageMedia::None);

        assert_eq!(message.display_content(), "Hello world");
    }

    #[test]
    fn display_content_returns_media_label_only_when_text_empty() {
        let message = msg("", MessageMedia::Photo);

        assert_eq!(message.display_content(), "[Photo]");
    }

    #[test]
    fn display_content_combines_media_label_and_text() {
        let message = msg("Check this out", MessageMedia::Photo);

        assert_eq!(message.display_content(), "[Photo]\nCheck this out");
    }

    #[test]
    fn display_content_media_with_text_uses_newline_separator() {
        let message = msg("Caption text", MessageMedia::Video);
        let content = message.display_content();

        assert!(
            content.contains('\n'),
            "Media + text should be separated by newline"
        );
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "[Video]");
        assert_eq!(lines[1], "Caption text");
    }

    #[test]
    fn display_content_media_only_has_no_newline() {
        let message = msg("", MessageMedia::Photo);
        let content = message.display_content();

        assert!(!content.contains('\n'));
        assert_eq!(content, "[Photo]");
    }

    #[test]
    fn display_content_text_only_has_no_media_prefix() {
        let message = msg("Just text", MessageMedia::None);
        let content = message.display_content();

        assert_eq!(content, "Just text");
        assert!(!content.starts_with('['));
    }

    #[test]
    fn display_content_handles_all_media_types() {
        let types = [
            (MessageMedia::Photo, "[Photo]"),
            (MessageMedia::Voice, "[Voice]"),
            (MessageMedia::Video, "[Video]"),
            (MessageMedia::VideoNote, "[Video message]"),
            (MessageMedia::Sticker, "[Sticker]"),
            (MessageMedia::Document, "[Document]"),
            (MessageMedia::Audio, "[Audio]"),
            (MessageMedia::Animation, "[GIF]"),
            (MessageMedia::Contact, "[Contact]"),
            (MessageMedia::Location, "[Location]"),
            (MessageMedia::Poll, "[Poll]"),
            (MessageMedia::Other, "[Media]"),
        ];

        for (media, expected_label) in types {
            let message = msg("", media);
            assert_eq!(
                message.display_content(),
                expected_label,
                "Failed for {:?}",
                media
            );
        }
    }

    // ── extract_first_url tests ──

    #[test]
    fn extract_first_url_returns_none_when_no_url() {
        assert_eq!(extract_first_url("hello world"), None);
    }

    #[test]
    fn extract_first_url_finds_https() {
        assert_eq!(
            extract_first_url("visit https://example.com please"),
            Some("https://example.com")
        );
    }

    #[test]
    fn extract_first_url_finds_http() {
        assert_eq!(
            extract_first_url("go to http://example.com"),
            Some("http://example.com")
        );
    }

    #[test]
    fn extract_first_url_returns_first_when_multiple() {
        assert_eq!(
            extract_first_url("see https://first.com and https://second.com"),
            Some("https://first.com")
        );
    }

    #[test]
    fn extract_first_url_handles_url_at_start() {
        assert_eq!(
            extract_first_url("https://start.com is the link"),
            Some("https://start.com")
        );
    }

    #[test]
    fn extract_first_url_handles_url_at_end() {
        assert_eq!(
            extract_first_url("link: https://end.com"),
            Some("https://end.com")
        );
    }

    #[test]
    fn extract_first_url_returns_none_for_empty_string() {
        assert_eq!(extract_first_url(""), None);
    }

    #[test]
    fn extract_first_url_ignores_non_http_schemes() {
        assert_eq!(extract_first_url("check ftp://files.com out"), None);
    }
}
