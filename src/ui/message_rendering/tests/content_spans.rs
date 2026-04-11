use super::*;
use crate::ui::message_rendering::content_spans::build_content_line_spans_linked;
use crate::ui::styles;
use ratatui::style::Modifier;
use ratatui::text::Span;

#[test]
fn spans_linked_plain_text_no_links() {
    let spans = build_content_line_spans_linked("Hello world", 0, &[]);
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].content.as_ref(), "Hello world");
    assert!(!spans[0].style.add_modifier.contains(Modifier::UNDERLINED));
}

#[test]
fn spans_linked_full_text_is_link() {
    let spans = build_content_line_spans_linked("Click here!", 0, &[(0, 11)]);
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].content.as_ref(), "Click here!");
    assert!(spans[0].style.add_modifier.contains(Modifier::UNDERLINED));
}

#[test]
fn spans_linked_link_in_middle() {
    let text = "Hello link world";
    let spans = build_content_line_spans_linked(text, 0, &[(6, 10)]);
    assert_eq!(spans.len(), 3);
    assert_eq!(spans[0].content.as_ref(), "Hello ");
    assert!(!spans[0].style.add_modifier.contains(Modifier::UNDERLINED));
    assert_eq!(spans[1].content.as_ref(), "link");
    assert!(spans[1].style.add_modifier.contains(Modifier::UNDERLINED));
    assert_eq!(spans[2].content.as_ref(), " world");
    assert!(!spans[2].style.add_modifier.contains(Modifier::UNDERLINED));
}

#[test]
fn spans_linked_with_content_offset() {
    let spans = build_content_line_spans_linked("link rest", 10, &[(10, 14)]);
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].content.as_ref(), "link");
    assert!(spans[0].style.add_modifier.contains(Modifier::UNDERLINED));
    assert_eq!(spans[1].content.as_ref(), " rest");
}

#[test]
fn spans_linked_non_overlapping_link_ignored() {
    let spans = build_content_line_spans_linked("Hello", 0, &[(100, 110)]);
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].content.as_ref(), "Hello");
    assert!(!spans[0].style.add_modifier.contains(Modifier::UNDERLINED));
}

#[test]
fn spans_linked_media_indicator_not_underlined() {
    let spans = build_content_line_spans_linked("[Photo]", 0, &[(0, 7)]);
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].content.as_ref(), "[Photo]");
    assert_eq!(spans[0].style, styles::message_media_style());
}

#[test]
fn spans_linked_multiple_links() {
    let spans = build_content_line_spans_linked("aa bb cc", 0, &[(0, 2), (6, 8)]);
    assert_eq!(spans.len(), 3);
    assert_eq!(spans[0].content.as_ref(), "aa");
    assert!(spans[0].style.add_modifier.contains(Modifier::UNDERLINED));
    assert_eq!(spans[1].content.as_ref(), " bb ");
    assert!(!spans[1].style.add_modifier.contains(Modifier::UNDERLINED));
    assert_eq!(spans[2].content.as_ref(), "cc");
    assert!(spans[2].style.add_modifier.contains(Modifier::UNDERLINED));
}

// ── Integration: message with links renders underlined ──

#[test]
fn message_with_text_url_entity_renders_underlined() {
    use crate::domain::message::TextLink;
    use crate::ui::message_rendering::{build_message_list_elements, element_to_text};

    let messages = vec![Message {
        id: 1,
        sender_name: "Alice".to_owned(),
        text: "Click here for details".to_owned(),
        timestamp_ms: FEB_14_2026_10AM,
        is_outgoing: false,
        media: MessageMedia::None,
        status: MessageStatus::Delivered,
        file_info: None,
        call_info: None,
        reply_to: None,
        forward_info: None,
        reaction_count: 0,
        links: vec![TextLink {
            offset: 0,
            length: 10,
            url: "https://example.com".to_owned(),
        }],
        is_edited: false,
    }];

    let elements = build_message_list_elements(&messages);
    let text = element_to_text(&elements[1], 80);

    let underlined_spans: Vec<&Span> = text
        .lines
        .iter()
        .flat_map(|l| l.spans.iter())
        .filter(|s| s.style.add_modifier.contains(Modifier::UNDERLINED))
        .collect();

    assert!(
        !underlined_spans.is_empty(),
        "Message with TextLink should have underlined spans"
    );
    assert_eq!(underlined_spans[0].content.as_ref(), "Click here");
}

#[test]
fn message_without_links_has_no_underline() {
    use crate::ui::message_rendering::{build_message_list_elements, element_to_text};

    let messages = vec![msg(
        1,
        "Alice",
        "Plain text message",
        FEB_14_2026_10AM,
        false,
    )];

    let elements = build_message_list_elements(&messages);
    let text = element_to_text(&elements[1], 80);

    let underlined_spans: Vec<&Span> = text
        .lines
        .iter()
        .flat_map(|l| l.spans.iter())
        .filter(|s| s.style.add_modifier.contains(Modifier::UNDERLINED))
        .collect();

    assert!(
        underlined_spans.is_empty(),
        "Plain message should have no underlined spans"
    );
}
