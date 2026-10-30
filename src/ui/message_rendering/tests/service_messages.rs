use super::*;
use crate::ui::message_rendering::{build_message_list_elements, element_to_text};

#[test]
fn service_message_renders_centered_with_sender_inline() {
    let messages = vec![Message {
        id: 1,
        sender_name: "Alice".to_owned(),
        text: "added Bob".to_owned(),
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
        is_service: true,
    }];

    let elements = build_message_list_elements(&messages);
    let msg_text = element_to_text(&elements[1], 80);

    assert_eq!(
        msg_text.lines.len(),
        1,
        "service message should be a single line"
    );

    let line = &msg_text.lines[0];
    assert_eq!(line.alignment, Some(ratatui::layout::Alignment::Center));

    let text_span = &line.spans[0];
    assert_eq!(text_span.content.as_ref(), "Alice added Bob");
    assert_eq!(text_span.style, crate::ui::styles::service_message_style());
}

#[test]
fn service_message_breaks_sender_grouping() {
    let messages = vec![
        msg(1, "Alice", "Hello", FEB_14_2026_10AM, false),
        Message {
            id: 2,
            sender_name: "Bob".to_owned(),
            text: "added Charlie".to_owned(),
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
            is_service: true,
        },
        msg(3, "Alice", "World", FEB_14_2026_10AM, false),
    ];

    let elements = build_message_list_elements(&messages);

    // elements[0] = date separator
    // elements[1] = Alice "Hello" (with sender header)
    // elements[2] = service message (centered)
    // elements[3] = Alice "World" (should show sender header again)

    let last_msg = element_to_text(&elements[3], 80);
    let all_text: String = last_msg
        .lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
        .collect();
    assert!(
        all_text.contains("Alice:"),
        "Sender header should reappear after service message"
    );
}

#[test]
fn service_message_shows_reaction_badge() {
    let messages = vec![Message {
        id: 1,
        sender_name: "Alice".to_owned(),
        text: "changed group photo".to_owned(),
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
        is_service: true,
    }];

    let elements = build_message_list_elements(&messages);
    let msg_text = element_to_text(&elements[1], 80);

    let all_text: String = msg_text
        .lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
        .collect();
    assert!(
        all_text.contains("[♡×3]"),
        "Service message should show reaction badge, got: '{}'",
        all_text
    );
}

#[test]
fn service_message_without_reactions_has_no_badge() {
    let messages = vec![Message {
        id: 1,
        sender_name: "Alice".to_owned(),
        text: "changed group photo".to_owned(),
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
        is_service: true,
    }];

    let elements = build_message_list_elements(&messages);
    let msg_text = element_to_text(&elements[1], 80);

    let all_text: String = msg_text
        .lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
        .collect();
    assert!(
        !all_text.contains("♡"),
        "Service message without reactions should have no badge"
    );
}
