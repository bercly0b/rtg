use super::*;
use crate::ui::message_rendering::{build_message_list_elements, element_to_text};

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
