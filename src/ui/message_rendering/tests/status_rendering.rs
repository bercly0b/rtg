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
        is_service: false,
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
        is_service: false,
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

// ── hidden time ──

#[test]
fn hidden_time_renders_blank_indent_span() {
    let messages = vec![
        msg(1, "Alice", "First", FEB_14_2026_10AM, false),
        msg(2, "Alice", "Second", FEB_14_2026_10AM + 5000, false),
    ];

    let elements = build_message_list_elements(&messages);
    let MessageListElement::Message {
        time, show_time, ..
    } = &elements[2]
    else {
        panic!("Expected Message element");
    };
    assert!(!show_time, "precondition: grouped message hides its time");

    let text = element_to_text(&elements[2], 80);

    assert_eq!(text.lines.len(), 1);
    let first_span = &text.lines[0].spans[0];
    assert_eq!(first_span.content.as_ref(), "      ");
    assert_eq!(first_span.style, ratatui::style::Style::default());

    let line_text: String = text.lines[0]
        .spans
        .iter()
        .map(|s| s.content.as_ref())
        .collect();
    assert!(
        !line_text.contains(time.as_str()),
        "hidden time must not be rendered: '{}'",
        line_text
    );
    assert!(line_text.contains("Second"));
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
        is_service: false,
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
        is_service: false,
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
        is_service: false,
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
