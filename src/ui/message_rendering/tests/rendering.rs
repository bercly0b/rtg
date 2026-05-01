use super::*;
use crate::ui::message_rendering::{
    build_message_list_elements, element_to_text, MessageListElement,
};

// ── sending status inline ──

#[test]
fn sending_status_on_same_line_as_content() {
    let messages = vec![Message {
        id: 1,
        sender_name: "User".to_owned(),
        text: "Hello".to_owned(),
        timestamp_ms: FEB_14_2026_10AM,
        is_outgoing: true,
        media: MessageMedia::None,
        status: MessageStatus::Sending,
        file_info: None,
        call_info: None,
        reply_to: None,
        forward_info: None,
        reaction_count: 0,
        links: Vec::new(),
        is_edited: false,
    }];

    let elements = build_message_list_elements(&messages);

    let msg_text = element_to_text(&elements[1], 80);
    let line_count = msg_text.lines.len();

    // Header line (time + sender) + content line with "sending..." appended = 2 lines
    assert_eq!(
        line_count, 2,
        "Expected header + content/status on same line, got {} lines",
        line_count
    );

    let last_line = &msg_text.lines[1];
    let last_line_text: String = last_line.spans.iter().map(|s| s.content.as_ref()).collect();
    assert!(
        last_line_text.contains("sending..."),
        "Last line should contain 'sending...', got: '{}'",
        last_line_text
    );
    assert!(
        last_line_text.contains("Hello"),
        "Last line should contain message text, got: '{}'",
        last_line_text
    );
}

#[test]
fn delivered_message_has_no_sending_indicator() {
    let messages = vec![Message {
        id: 1,
        sender_name: "User".to_owned(),
        text: "Hello".to_owned(),
        timestamp_ms: FEB_14_2026_10AM,
        is_outgoing: true,
        media: MessageMedia::None,
        status: MessageStatus::Delivered,
        file_info: None,
        call_info: None,
        reply_to: None,
        forward_info: None,
        reaction_count: 0,
        links: Vec::new(),
        is_edited: false,
    }];

    let elements = build_message_list_elements(&messages);
    let msg_text = element_to_text(&elements[1], 80);

    assert_eq!(msg_text.lines.len(), 2);
    let all_text: String = msg_text
        .lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
        .collect();
    assert!(!all_text.contains("sending..."));
}

// ── edited indicator ──

#[test]
fn edited_message_shows_edited_indicator() {
    let messages = vec![Message {
        id: 1,
        sender_name: "User".to_owned(),
        text: "Hello".to_owned(),
        timestamp_ms: FEB_14_2026_10AM,
        is_outgoing: false,
        media: MessageMedia::None,
        status: MessageStatus::Delivered,
        file_info: None,
        call_info: None,
        reply_to: None,
        forward_info: None,
        reaction_count: 0,
        links: Vec::new(),
        is_edited: true,
    }];

    let elements = build_message_list_elements(&messages);
    let msg_text = element_to_text(&elements[1], 80);

    let all_text: String = msg_text
        .lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
        .collect();
    assert!(
        all_text.contains("edited"),
        "Edited message should contain 'edited' indicator, got: '{}'",
        all_text
    );
}

#[test]
fn non_edited_message_has_no_edited_indicator() {
    let messages = vec![Message {
        id: 1,
        sender_name: "User".to_owned(),
        text: "Hello".to_owned(),
        timestamp_ms: FEB_14_2026_10AM,
        is_outgoing: false,
        media: MessageMedia::None,
        status: MessageStatus::Delivered,
        file_info: None,
        call_info: None,
        reply_to: None,
        forward_info: None,
        reaction_count: 0,
        links: Vec::new(),
        is_edited: false,
    }];

    let elements = build_message_list_elements(&messages);
    let msg_text = element_to_text(&elements[1], 80);

    let all_text: String = msg_text
        .lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
        .collect();
    assert!(
        !all_text.contains("edited"),
        "Non-edited message should not contain 'edited' indicator"
    );
}

#[test]
fn edited_indicator_on_same_line_as_content() {
    let messages = vec![Message {
        id: 1,
        sender_name: "User".to_owned(),
        text: "Hello".to_owned(),
        timestamp_ms: FEB_14_2026_10AM,
        is_outgoing: true,
        media: MessageMedia::None,
        status: MessageStatus::Delivered,
        file_info: None,
        call_info: None,
        reply_to: None,
        forward_info: None,
        reaction_count: 0,
        links: Vec::new(),
        is_edited: true,
    }];

    let elements = build_message_list_elements(&messages);
    let msg_text = element_to_text(&elements[1], 80);

    assert_eq!(msg_text.lines.len(), 2);

    let last_line = &msg_text.lines[1];
    let last_line_text: String = last_line.spans.iter().map(|s| s.content.as_ref()).collect();
    assert!(
        last_line_text.contains("Hello") && last_line_text.contains("edited"),
        "Content and 'edited' should be on the same line, got: '{}'",
        last_line_text
    );
}

// ── media rendering ──

#[test]
fn media_with_text_renders_on_separate_lines() {
    let messages = vec![msg_with_media(
        1,
        "Alice",
        "Check this out",
        FEB_14_2026_10AM,
        MessageMedia::Photo,
    )];

    let elements = build_message_list_elements(&messages);
    let msg_text = element_to_text(&elements[1], 80);

    assert_eq!(
        msg_text.lines.len(),
        3,
        "Expected 3 lines (header + media + text), got {}",
        msg_text.lines.len()
    );

    let media_line: String = msg_text.lines[1]
        .spans
        .iter()
        .map(|s| s.content.as_ref())
        .collect();
    assert!(
        media_line.contains("[Photo]"),
        "Second line should be media indicator"
    );

    let text_line: String = msg_text.lines[2]
        .spans
        .iter()
        .map(|s| s.content.as_ref())
        .collect();
    assert!(
        text_line.contains("Check this out"),
        "Third line should be message text"
    );
}

#[test]
fn media_only_renders_single_content_line() {
    let messages = vec![msg_with_media(
        1,
        "Alice",
        "",
        FEB_14_2026_10AM,
        MessageMedia::Photo,
    )];

    let elements = build_message_list_elements(&messages);
    let msg_text = element_to_text(&elements[1], 80);

    assert_eq!(msg_text.lines.len(), 2);
}

// ── text wrapping ──

#[test]
fn long_message_wraps_within_width() {
    let long_text = "a".repeat(50);
    let messages = vec![msg(1, "Alice", &long_text, FEB_14_2026_10AM, false)];

    let elements = build_message_list_elements(&messages);
    let msg_text = element_to_text(&elements[1], 30);

    assert!(
        msg_text.lines.len() >= 3,
        "Long text should wrap into multiple lines, got {} lines",
        msg_text.lines.len()
    );
}

// ── file metadata ──

#[test]
fn voice_message_shows_file_metadata() {
    use crate::domain::message::{DownloadStatus, FileInfo};

    let messages = vec![Message {
        id: 1,
        sender_name: "Alice".to_owned(),
        text: String::new(),
        timestamp_ms: FEB_14_2026_10AM,
        is_outgoing: false,
        media: MessageMedia::Voice,
        status: MessageStatus::Delivered,
        file_info: Some(FileInfo {
            file_id: 1,
            local_path: Some("/tmp/v.ogg".to_owned()),
            mime_type: "audio/ogg".to_owned(),
            size: Some(15_500),
            duration: Some(3),
            file_name: None,
            is_listened: true,
            download_status: DownloadStatus::Completed,
        }),
        call_info: None,
        reply_to: None,
        forward_info: None,
        reaction_count: 0,
        links: Vec::new(),
        is_edited: false,
    }];

    let elements = build_message_list_elements(&messages);

    if let MessageListElement::Message { file_meta, .. } = &elements[1] {
        let meta = file_meta
            .as_ref()
            .expect("voice message should have file_meta");
        assert!(meta.contains("download=yes"), "should contain download=yes");
        assert!(meta.contains("size=15.5KB"), "should contain size");
        assert!(meta.contains("duration=0:03"), "should contain duration");
        assert!(meta.contains("listened=yes"), "should contain listened=yes");
    } else {
        panic!("Expected Message element");
    }

    let msg_text = element_to_text(&elements[1], 120);
    let all_text: String = msg_text
        .lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
        .collect();
    assert!(
        all_text.contains("[Voice]") && all_text.contains("download=yes"),
        "Rendered text should contain both media label and metadata"
    );
}

#[test]
fn document_message_shows_file_name_and_extension() {
    use crate::domain::message::{DownloadStatus, FileInfo};

    let messages = vec![Message {
        id: 1,
        sender_name: "Alice".to_owned(),
        text: String::new(),
        timestamp_ms: FEB_14_2026_10AM,
        is_outgoing: false,
        media: MessageMedia::Document,
        status: MessageStatus::Delivered,
        file_info: Some(FileInfo {
            file_id: 1,
            local_path: Some("/tmp/report.pdf".to_owned()),
            mime_type: "application/pdf".to_owned(),
            size: Some(2_500_000),
            duration: None,
            file_name: Some("report.pdf".to_owned()),
            is_listened: false,
            download_status: DownloadStatus::Completed,
        }),
        call_info: None,
        reply_to: None,
        forward_info: None,
        reaction_count: 0,
        links: Vec::new(),
        is_edited: false,
    }];

    let elements = build_message_list_elements(&messages);

    if let MessageListElement::Message { file_meta, .. } = &elements[1] {
        let meta = file_meta
            .as_ref()
            .expect("document message should have file_meta");
        assert!(
            meta.contains("name=report.pdf"),
            "should contain file name, got: '{}'",
            meta
        );
        assert!(
            meta.contains("type=pdf"),
            "should contain file extension, got: '{}'",
            meta
        );
    } else {
        panic!("Expected Message element");
    }

    let msg_text = element_to_text(&elements[1], 120);
    let all_text: String = msg_text
        .lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
        .collect();
    assert!(
        all_text.contains("[Document]")
            && all_text.contains("report.pdf")
            && all_text.contains("type=pdf"),
        "Rendered text should contain media label, file name and type, got: '{}'",
        all_text
    );
}

#[test]
fn document_without_extension_does_not_break_layout() {
    use crate::domain::message::{DownloadStatus, FileInfo};

    fn doc_message(file_name: Option<String>) -> Message {
        Message {
            id: 1,
            sender_name: "Alice".to_owned(),
            text: String::new(),
            timestamp_ms: FEB_14_2026_10AM,
            is_outgoing: false,
            media: MessageMedia::Document,
            status: MessageStatus::Delivered,
            file_info: Some(FileInfo {
                file_id: 1,
                local_path: None,
                mime_type: "application/octet-stream".to_owned(),
                size: Some(500),
                duration: None,
                file_name,
                is_listened: false,
                download_status: DownloadStatus::NotStarted,
            }),
            call_info: None,
            reply_to: None,
            forward_info: None,
            reaction_count: 0,
            links: Vec::new(),
            is_edited: false,
        }
    }

    let with_name = vec![doc_message(Some("notes".to_owned()))];
    let without_name = vec![doc_message(None)];

    let elems_with = build_message_list_elements(&with_name);
    let elems_without = build_message_list_elements(&without_name);

    if let MessageListElement::Message { file_meta, .. } = &elems_with[1] {
        let meta = file_meta.as_ref().expect("file_meta present");
        assert!(meta.contains("name=notes"), "got: '{}'", meta);
        assert!(
            !meta.contains("type="),
            "extension-less name must not produce type=, got: '{}'",
            meta
        );
    } else {
        panic!("Expected Message element");
    }

    let text_with = element_to_text(&elems_with[1], 120);
    let text_without = element_to_text(&elems_without[1], 120);
    assert_eq!(
        text_with.lines.len(),
        text_without.lines.len(),
        "extension-less name must not change line count (layout stays consistent)"
    );
}

#[test]
fn text_message_has_no_file_metadata() {
    let messages = vec![msg(1, "Alice", "Hello", FEB_14_2026_10AM, false)];

    let elements = build_message_list_elements(&messages);

    if let MessageListElement::Message { file_meta, .. } = &elements[1] {
        assert!(file_meta.is_none(), "text message should have no file_meta");
    } else {
        panic!("Expected Message element");
    }
}

// ── reactions ──

#[test]
fn message_with_multiple_reactions_shows_count() {
    let messages = vec![Message {
        id: 1,
        sender_name: "Alice".to_owned(),
        text: "Hello".to_owned(),
        timestamp_ms: FEB_14_2026_10AM,
        is_outgoing: false,
        media: MessageMedia::None,
        status: MessageStatus::Delivered,
        file_info: None,
        call_info: None,
        reply_to: None,
        forward_info: None,
        reaction_count: 3,
        links: Vec::new(),
        is_edited: false,
    }];

    let elements = build_message_list_elements(&messages);
    let msg_text = element_to_text(&elements[1], 80);
    let all_text: String = msg_text
        .lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
        .collect();

    assert!(all_text.contains("[♡×3]"));
}

#[test]
fn message_with_single_reaction_shows_heart_without_count() {
    let messages = vec![Message {
        id: 1,
        sender_name: "Alice".to_owned(),
        text: "Hello".to_owned(),
        timestamp_ms: FEB_14_2026_10AM,
        is_outgoing: false,
        media: MessageMedia::None,
        status: MessageStatus::Delivered,
        file_info: None,
        call_info: None,
        reply_to: None,
        forward_info: None,
        reaction_count: 1,
        links: Vec::new(),
        is_edited: false,
    }];

    let elements = build_message_list_elements(&messages);
    let msg_text = element_to_text(&elements[1], 80);
    let all_text: String = msg_text
        .lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
        .collect();

    assert!(all_text.contains("[♡]"));
    assert!(!all_text.contains("[♡×1]"));
}
