use super::*;
use crate::ui::message_rendering::{
    build_message_list_elements, element_to_text, MessageListElement,
};

#[test]
fn forward_info_propagated_to_element() {
    let messages = vec![msg_with_forward(
        1,
        "Bob",
        "Check this out",
        FEB_14_2026_10AM,
        "Alice",
    )];

    let elements = build_message_list_elements(&messages);

    if let MessageListElement::Message { forward_info, .. } = &elements[1] {
        let fwd = forward_info.as_ref().expect("should have forward_info");
        assert_eq!(fwd.sender_name, "Alice");
    } else {
        panic!("Expected Message element");
    }
}

#[test]
fn forward_renders_between_header_and_content() {
    let messages = vec![msg_with_forward(
        1,
        "Bob",
        "Check this out",
        FEB_14_2026_10AM,
        "Alice",
    )];

    let elements = build_message_list_elements(&messages);
    let text = element_to_text(&elements[1], 80);

    assert_eq!(
        text.lines.len(),
        3,
        "Expected 3 lines (header + forward + content), got {}",
        text.lines.len()
    );

    let forward_line: String = text.lines[1]
        .spans
        .iter()
        .map(|s| s.content.as_ref())
        .collect();
    assert!(
        forward_line.contains('│'),
        "Forward line should contain bar: '{}'",
        forward_line
    );
    assert!(
        forward_line.contains("Forwarded from"),
        "Forward line should contain label: '{}'",
        forward_line
    );
    assert!(
        forward_line.contains("Alice"),
        "Forward line should contain sender name: '{}'",
        forward_line
    );
}

#[test]
fn forward_not_rendered_when_none() {
    let messages = vec![msg(1, "Alice", "Hello", FEB_14_2026_10AM, false)];

    let elements = build_message_list_elements(&messages);
    let text = element_to_text(&elements[1], 80);

    let all_text: String = text
        .lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
        .collect();
    assert!(
        !all_text.contains("Forwarded from"),
        "Should not contain forward label when no forward"
    );
}

#[test]
fn grouped_message_with_forward_shows_forward_line() {
    let messages = vec![
        msg(1, "Alice", "First", FEB_14_2026_10AM, false),
        msg_with_forward(
            2,
            "Alice",
            "Forwarded msg",
            FEB_14_2026_10AM + 5000,
            "Charlie",
        ),
    ];

    let elements = build_message_list_elements(&messages);

    if let MessageListElement::Message {
        sender,
        forward_info,
        ..
    } = &elements[2]
    {
        assert!(sender.is_none(), "Should be grouped (no sender)");
        assert!(forward_info.is_some(), "Should have forward_info");
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
        all_text.contains("Forwarded from") && all_text.contains("Charlie"),
        "Grouped message should render forward: '{}'",
        all_text
    );
}
