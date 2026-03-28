use super::*;
use crate::ui::message_rendering::{
    build_message_list_elements, element_to_text, MessageListElement,
};

#[test]
fn reply_info_propagated_to_element() {
    let messages = vec![msg_with_reply(
        1,
        "Bob",
        "Yes, done",
        FEB_14_2026_10AM,
        "Alice",
        "Is the PR ready?",
    )];

    let elements = build_message_list_elements(&messages);

    if let MessageListElement::Message { reply_info, .. } = &elements[1] {
        let reply = reply_info.as_ref().expect("should have reply_info");
        assert_eq!(reply.sender_name, "Alice");
        assert_eq!(reply.text, "Is the PR ready?");
    } else {
        panic!("Expected Message element");
    }
}

#[test]
fn reply_renders_between_header_and_content() {
    let messages = vec![msg_with_reply(
        1,
        "Bob",
        "Yes, done",
        FEB_14_2026_10AM,
        "Alice",
        "Is the PR ready?",
    )];

    let elements = build_message_list_elements(&messages);
    let text = element_to_text(&elements[1], 80);

    // Header + reply + content = 3 lines
    assert_eq!(
        text.lines.len(),
        3,
        "Expected 3 lines (header + reply + content), got {}",
        text.lines.len()
    );

    let reply_line: String = text.lines[1]
        .spans
        .iter()
        .map(|s| s.content.as_ref())
        .collect();
    assert!(
        reply_line.contains('│'),
        "Reply line should contain bar: '{}'",
        reply_line
    );
    assert!(
        reply_line.contains("Alice"),
        "Reply line should contain sender name: '{}'",
        reply_line
    );
    assert!(
        reply_line.contains("Is the PR ready?"),
        "Reply line should contain reply text: '{}'",
        reply_line
    );
}

#[test]
fn reply_not_rendered_when_none() {
    let messages = vec![msg(1, "Alice", "Hello", FEB_14_2026_10AM, false)];

    let elements = build_message_list_elements(&messages);
    let text = element_to_text(&elements[1], 80);

    assert_eq!(text.lines.len(), 2);

    let all_text: String = text
        .lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
        .collect();
    assert!(
        !all_text.contains('│'),
        "Should not contain reply bar when no reply"
    );
}

#[test]
fn reply_with_empty_sender_omits_name() {
    let messages = vec![msg_with_reply(
        1,
        "Bob",
        "OK",
        FEB_14_2026_10AM,
        "",
        "Some message",
    )];

    let elements = build_message_list_elements(&messages);
    let text = element_to_text(&elements[1], 80);

    let reply_line: String = text.lines[1]
        .spans
        .iter()
        .map(|s| s.content.as_ref())
        .collect();
    assert!(reply_line.contains("Some message"));
    assert!(!reply_line.contains(": Some"));
}

#[test]
fn grouped_message_with_reply_shows_reply_line() {
    let messages = vec![
        msg(1, "Alice", "First", FEB_14_2026_10AM, false),
        Message {
            id: 2,
            sender_name: "Alice".to_owned(),
            text: "Reply msg".to_owned(),
            timestamp_ms: FEB_14_2026_10AM + 5000,
            is_outgoing: false,
            media: MessageMedia::None,
            status: MessageStatus::Delivered,
            file_info: None,
            call_info: None,
            reply_to: Some(ReplyInfo {
                sender_name: "Bob".to_owned(),
                text: "Original text".to_owned(),
                is_outgoing: false,
            }),
            reaction_count: 0,
            links: Vec::new(),
            is_edited: false,
        },
    ];

    let elements = build_message_list_elements(&messages);

    if let MessageListElement::Message {
        sender, reply_info, ..
    } = &elements[2]
    {
        assert!(sender.is_none(), "Should be grouped (no sender)");
        assert!(reply_info.is_some(), "Should have reply_info");
    } else {
        panic!("Expected Message element");
    }

    let text = element_to_text(&elements[2], 80);
    let all_text: String = text
        .lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
        .collect();
    assert!(
        all_text.contains("Bob") && all_text.contains("Original text"),
        "Grouped message should render reply: '{}'",
        all_text
    );
}
