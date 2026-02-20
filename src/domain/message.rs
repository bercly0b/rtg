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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub id: i32,
    pub sender_name: String,
    pub text: String,
    pub timestamp_ms: i64,
    pub is_outgoing: bool,
    pub media: MessageMedia,
}

impl Message {
    /// Returns the display content: media label + text, or just text if no media.
    pub fn display_content(&self) -> String {
        match (self.media.display_label(), self.text.is_empty()) {
            (Some(label), true) => label.to_owned(),
            (Some(label), false) => format!("{} {}", label, self.text),
            (None, _) => self.text.clone(),
        }
    }
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

        assert_eq!(message.display_content(), "[Photo] Check this out");
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
}
