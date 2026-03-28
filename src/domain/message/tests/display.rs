use super::*;

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
        (MessageMedia::Call, "[Call]"),
        (MessageMedia::VideoCall, "[Video call]"),
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
