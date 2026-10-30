use super::*;
use crate::domain::message::{DownloadStatus, FileInfo};
use crate::ui::message_rendering::{build_message_list_elements, element_to_text};

fn voice_file() -> FileInfo {
    FileInfo {
        file_id: 1,
        local_path: Some("/tmp/v.ogg".to_owned()),
        mime_type: "audio/ogg".to_owned(),
        size: Some(15_500),
        duration: Some(3),
        file_name: None,
        is_listened: true,
        download_status: DownloadStatus::Completed,
    }
}

fn voice_reply(sender: &str, reply_text: &str) -> Message {
    let mut message = msg_with_reply(1, sender, "", FEB_14_2026_10AM, "Alice", reply_text);
    message.media = MessageMedia::Voice;
    message.file_info = Some(voice_file());
    message
}

fn rendered_lines(message: Message) -> Vec<String> {
    let elements = build_message_list_elements(&[message]);
    element_to_text(&elements[1], 120)
        .lines
        .iter()
        .map(|line| line.to_string())
        .collect()
}

#[test]
fn file_meta_skips_bracketed_reply_text() {
    let lines = rendered_lines(voice_reply("Bob", "[1] foo"));

    assert_eq!(lines[1], "      │ Alice: [1] foo");
    assert!(
        lines[2].starts_with("      [Voice] download="),
        "meta must follow the media label: {:?}",
        lines
    );
}

#[test]
fn file_meta_skips_reply_quoting_the_same_media_label() {
    let lines = rendered_lines(voice_reply("Bob", "[Voice]"));

    assert_eq!(lines[1], "      │ Alice: [Voice]");
    assert!(
        lines[2].starts_with("      [Voice] download="),
        "meta must follow the media label: {:?}",
        lines
    );
}

#[test]
fn file_meta_skips_bracketed_sender_name() {
    let mut message = msg_with_media(1, "[Bot] Bob", "", FEB_14_2026_10AM, MessageMedia::Voice);
    message.file_info = Some(voice_file());

    let lines = rendered_lines(message);

    assert!(lines[0].ends_with("[Bot] Bob:"), "header: {:?}", lines);
    assert!(
        lines[1].starts_with("      [Voice] download="),
        "meta must follow the media label: {:?}",
        lines
    );
}

#[test]
fn unlabeled_media_meta_follows_content_not_bracketed_reply() {
    let mut message = msg_with_reply(1, "Bob", "😀", FEB_14_2026_10AM, "Alice", "[1] foo");
    message.media = MessageMedia::Sticker;

    let lines = rendered_lines(message);

    assert_eq!(lines[1], "      │ Alice: [1] foo");
    assert_eq!(lines[2], "      😀 Sticker");
}
