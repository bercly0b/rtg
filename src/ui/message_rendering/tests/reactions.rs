use super::*;
use crate::ui::message_rendering::{build_message_list_elements, element_to_text};

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
        is_service: false,
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
        is_service: false,
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
