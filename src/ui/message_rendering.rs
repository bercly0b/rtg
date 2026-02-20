//! Message list rendering logic.
//!
//! Handles visual formatting of messages including:
//! - Multi-line message display (time + sender on first line, text on second)
//! - Sender grouping (consecutive messages from same sender show name only once)
//! - Date separators between messages from different days
//! - Media type indicators

use chrono::{Local, TimeZone};
use ratatui::{
    layout::Alignment,
    text::{Line, Span},
    widgets::ListItem,
};

use crate::domain::message::Message;

use super::styles;

/// Represents a visual element in the messages list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageListElement {
    /// Date separator line (e.g., "——— 14 Feb 2026 ———").
    DateSeparator(String),
    /// A message with optional sender display.
    Message {
        time: String,
        sender: Option<String>,
        content: String,
    },
}

/// Builds a list of visual elements from messages.
///
/// Groups consecutive messages from the same sender and inserts date separators.
pub fn build_message_list_elements(messages: &[Message]) -> Vec<MessageListElement> {
    let mut elements = Vec::new();
    let mut prev_date: Option<chrono::NaiveDate> = None;
    let mut prev_sender: Option<&str> = None;

    for message in messages {
        let msg_date = timestamp_to_date(message.timestamp_ms);

        // Insert date separator if date changed
        if prev_date != Some(msg_date) {
            elements.push(MessageListElement::DateSeparator(format_date(msg_date)));
            prev_sender = None; // Reset sender grouping on date change
        }

        let sender_name = effective_sender_name(message);

        // Show sender only if different from previous message
        let show_sender = prev_sender != Some(sender_name);
        let sender = if show_sender {
            Some(sender_name.to_owned())
        } else {
            None
        };

        elements.push(MessageListElement::Message {
            time: format_time(message.timestamp_ms),
            sender,
            content: message.display_content(),
        });

        prev_date = Some(msg_date);
        prev_sender = Some(sender_name);
    }

    elements
}

/// Converts a message index to the corresponding element index in the list.
///
/// Since the element list contains both messages and date separators,
/// this function finds the element index for a given message index.
/// Returns `None` if the message index is out of range.
pub fn message_index_to_element_index(
    elements: &[MessageListElement],
    message_index: usize,
) -> Option<usize> {
    let mut msg_count = 0;

    for (elem_idx, element) in elements.iter().enumerate() {
        if matches!(element, MessageListElement::Message { .. }) {
            if msg_count == message_index {
                return Some(elem_idx);
            }
            msg_count += 1;
        }
    }

    None
}

/// Converts a list element to a ListItem for ratatui rendering.
pub fn element_to_list_item(element: &MessageListElement) -> ListItem<'static> {
    match element {
        MessageListElement::DateSeparator(date) => date_separator_item(date),
        MessageListElement::Message {
            time,
            sender,
            content,
        } => message_item(time, sender.as_deref(), content),
    }
}

fn date_separator_item(date: &str) -> ListItem<'static> {
    let separator = format!("——— {} ———", date);
    let line = Line::from(vec![Span::styled(
        separator,
        styles::date_separator_style(),
    )])
    .alignment(Alignment::Center);
    ListItem::new(vec![Line::default(), line, Line::default()])
}

fn message_item(time: &str, sender: Option<&str>, content: &str) -> ListItem<'static> {
    let mut lines = Vec::new();
    let indent = "      "; // 6 spaces to align with time column

    if sender.is_some() {
        // First message in group: header line (time + sender), then content on separate lines
        let header_line = build_message_header_line(time, sender);
        lines.push(header_line);

        for text_line in content.lines() {
            let content_spans = build_content_line_spans(text_line);
            let mut line_spans = vec![Span::raw(indent.to_owned())];
            line_spans.extend(content_spans);
            lines.push(Line::from(line_spans));
        }

        if content.is_empty() {
            lines.push(Line::from(vec![
                Span::raw(indent.to_owned()),
                Span::styled("[Empty message]".to_owned(), styles::message_media_style()),
            ]));
        }
    } else {
        // Grouped message (no sender): time + first line of content on same line
        let mut content_lines = content.lines();

        if let Some(first_line) = content_lines.next() {
            // Time + first content line on same row
            let mut spans = vec![Span::styled(
                format!("{:>5} ", time),
                styles::message_time_style(),
            )];
            spans.extend(build_content_line_spans(first_line));
            lines.push(Line::from(spans));

            // Remaining lines with indent
            for text_line in content_lines {
                let content_spans = build_content_line_spans(text_line);
                let mut line_spans = vec![Span::raw(indent.to_owned())];
                line_spans.extend(content_spans);
                lines.push(Line::from(line_spans));
            }
        } else {
            // Empty content
            let mut spans = vec![Span::styled(
                format!("{:>5} ", time),
                styles::message_time_style(),
            )];
            spans.push(Span::styled(
                "[Empty message]".to_owned(),
                styles::message_media_style(),
            ));
            lines.push(Line::from(spans));
        }
    }

    ListItem::new(lines)
}

fn build_message_header_line(time: &str, sender: Option<&str>) -> Line<'static> {
    let mut spans = vec![Span::styled(
        format!("{:>5} ", time),
        styles::message_time_style(),
    )];

    if let Some(name) = sender {
        spans.push(Span::styled(
            format!("{}:", name),
            styles::message_sender_style(),
        ));
    }

    Line::from(spans)
}

/// Builds styled spans for content line, highlighting media indicators in cyan.
fn build_content_line_spans(text: &str) -> Vec<Span<'static>> {
    // Check if text starts with a media indicator like [Photo], [Voice], etc.
    if text.starts_with('[') {
        if let Some(end_bracket) = text.find(']') {
            let media_part = &text[..=end_bracket];
            let rest = text[end_bracket + 1..].trim_start();

            if rest.is_empty() {
                // Media indicator only
                return vec![Span::styled(
                    media_part.to_owned(),
                    styles::message_media_style(),
                )];
            } else {
                // Media indicator + text
                return vec![
                    Span::styled(media_part.to_owned(), styles::message_media_style()),
                    Span::raw(" ".to_owned()),
                    Span::styled(rest.to_owned(), styles::message_text_style()),
                ];
            }
        }
    }

    // Regular text
    vec![Span::styled(text.to_owned(), styles::message_text_style())]
}

fn effective_sender_name(message: &Message) -> &str {
    if message.is_outgoing {
        "You"
    } else {
        &message.sender_name
    }
}

fn timestamp_to_date(timestamp_ms: i64) -> chrono::NaiveDate {
    match Local.timestamp_millis_opt(timestamp_ms) {
        chrono::LocalResult::Single(dt) => dt.date_naive(),
        chrono::LocalResult::Ambiguous(dt, _) => dt.date_naive(),
        chrono::LocalResult::None => Local::now().date_naive(),
    }
}

fn format_date(date: chrono::NaiveDate) -> String {
    // Format: "14 Feb 2026"
    date.format("%-d %b %Y").to_string()
}

fn format_time(timestamp_ms: i64) -> String {
    match Local.timestamp_millis_opt(timestamp_ms) {
        chrono::LocalResult::Single(dt) => dt.format("%H:%M").to_string(),
        chrono::LocalResult::Ambiguous(dt, _) => dt.format("%H:%M").to_string(),
        chrono::LocalResult::None => "??:??".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::message::MessageMedia;

    fn msg(id: i32, sender: &str, text: &str, ts_ms: i64, outgoing: bool) -> Message {
        Message {
            id,
            sender_name: sender.to_owned(),
            text: text.to_owned(),
            timestamp_ms: ts_ms,
            is_outgoing: outgoing,
            media: MessageMedia::None,
        }
    }

    fn msg_with_media(
        id: i32,
        sender: &str,
        text: &str,
        ts_ms: i64,
        media: MessageMedia,
    ) -> Message {
        Message {
            id,
            sender_name: sender.to_owned(),
            text: text.to_owned(),
            timestamp_ms: ts_ms,
            is_outgoing: false,
            media,
        }
    }

    // Note: These timestamps are in UTC. Tests use Local timezone for conversion,
    // so the displayed time may vary by timezone. However, the date grouping logic
    // (same day vs different day) should work correctly regardless of timezone.
    const FEB_14_2026_10AM: i64 = 1771059600000; // 2026-02-14 10:00:00 UTC
    const FEB_15_2026_1PM: i64 = 1771156800000; // 2026-02-15 13:00:00 UTC

    #[test]
    fn builds_date_separator_for_first_message() {
        let messages = vec![msg(1, "Alice", "Hello", FEB_14_2026_10AM, false)];

        let elements = build_message_list_elements(&messages);

        assert_eq!(elements.len(), 2);
        assert!(matches!(&elements[0], MessageListElement::DateSeparator(_)));
    }

    #[test]
    fn groups_consecutive_messages_from_same_sender() {
        let messages = vec![
            msg(1, "Alice", "First", FEB_14_2026_10AM, false),
            msg(2, "Alice", "Second", FEB_14_2026_10AM + 60000, false),
        ];

        let elements = build_message_list_elements(&messages);

        // DateSeparator + Message1 (with sender) + Message2 (no sender)
        assert_eq!(elements.len(), 3);

        if let MessageListElement::Message { sender, .. } = &elements[1] {
            assert!(sender.is_some());
        } else {
            panic!("Expected Message element");
        }

        if let MessageListElement::Message { sender, .. } = &elements[2] {
            assert!(sender.is_none());
        } else {
            panic!("Expected Message element");
        }
    }

    #[test]
    fn shows_sender_when_sender_changes() {
        let messages = vec![
            msg(1, "Alice", "Hi", FEB_14_2026_10AM, false),
            msg(2, "Bob", "Hello", FEB_14_2026_10AM + 60000, false),
        ];

        let elements = build_message_list_elements(&messages);

        // DateSeparator + Message1 (Alice) + Message2 (Bob)
        assert_eq!(elements.len(), 3);

        if let MessageListElement::Message { sender, .. } = &elements[1] {
            assert_eq!(sender.as_deref(), Some("Alice"));
        }

        if let MessageListElement::Message { sender, .. } = &elements[2] {
            assert_eq!(sender.as_deref(), Some("Bob"));
        }
    }

    #[test]
    fn inserts_date_separator_on_date_change() {
        let messages = vec![
            msg(1, "Alice", "Day 1", FEB_14_2026_10AM, false),
            msg(2, "Alice", "Day 2", FEB_15_2026_1PM, false),
        ];

        let elements = build_message_list_elements(&messages);

        // DateSeparator1 + Message1 + DateSeparator2 + Message2
        assert_eq!(elements.len(), 4);
        assert!(matches!(&elements[0], MessageListElement::DateSeparator(_)));
        assert!(matches!(&elements[2], MessageListElement::DateSeparator(_)));
    }

    #[test]
    fn resets_sender_grouping_on_date_change() {
        let messages = vec![
            msg(1, "Alice", "Day 1", FEB_14_2026_10AM, false),
            msg(2, "Alice", "Day 2", FEB_15_2026_1PM, false),
        ];

        let elements = build_message_list_elements(&messages);

        // Both messages should show sender (after date separators)
        if let MessageListElement::Message { sender, .. } = &elements[1] {
            assert!(sender.is_some(), "First message should show sender");
        }

        if let MessageListElement::Message { sender, .. } = &elements[3] {
            assert!(
                sender.is_some(),
                "Message after date change should show sender"
            );
        }
    }

    #[test]
    fn uses_you_for_outgoing_messages() {
        let messages = vec![msg(1, "MyName", "Hello", FEB_14_2026_10AM, true)];

        let elements = build_message_list_elements(&messages);

        if let MessageListElement::Message { sender, .. } = &elements[1] {
            assert_eq!(sender.as_deref(), Some("You"));
        }
    }

    #[test]
    fn media_message_shows_indicator() {
        let messages = vec![msg_with_media(
            1,
            "Alice",
            "",
            FEB_14_2026_10AM,
            MessageMedia::Photo,
        )];

        let elements = build_message_list_elements(&messages);

        if let MessageListElement::Message { content, .. } = &elements[1] {
            assert_eq!(content, "[Photo]");
        }
    }

    #[test]
    fn media_message_with_text_shows_both() {
        let messages = vec![msg_with_media(
            1,
            "Alice",
            "Check this out",
            FEB_14_2026_10AM,
            MessageMedia::Photo,
        )];

        let elements = build_message_list_elements(&messages);

        if let MessageListElement::Message { content, .. } = &elements[1] {
            assert_eq!(content, "[Photo] Check this out");
        }
    }

    #[test]
    fn format_date_produces_correct_format() {
        let date = chrono::NaiveDate::from_ymd_opt(2026, 2, 14).unwrap();

        let formatted = format_date(date);

        assert_eq!(formatted, "14 Feb 2026");
    }

    #[test]
    fn format_time_produces_hh_mm() {
        // Note: this test may be timezone-dependent
        let time = format_time(FEB_14_2026_10AM);

        assert_eq!(time.len(), 5);
        assert!(time.contains(':'));
    }

    #[test]
    fn message_index_to_element_index_maps_first_message() {
        let messages = vec![msg(1, "Alice", "Hello", FEB_14_2026_10AM, false)];
        let elements = build_message_list_elements(&messages);

        // Elements: [DateSeparator, Message]
        // Message index 0 -> Element index 1
        assert_eq!(message_index_to_element_index(&elements, 0), Some(1));
    }

    #[test]
    fn message_index_to_element_index_accounts_for_date_separators() {
        let messages = vec![
            msg(1, "Alice", "Day 1", FEB_14_2026_10AM, false),
            msg(2, "Alice", "Day 2", FEB_15_2026_1PM, false),
        ];
        let elements = build_message_list_elements(&messages);

        // Elements: [DateSeparator1, Message1, DateSeparator2, Message2]
        // Message index 0 -> Element index 1
        // Message index 1 -> Element index 3
        assert_eq!(message_index_to_element_index(&elements, 0), Some(1));
        assert_eq!(message_index_to_element_index(&elements, 1), Some(3));
    }

    #[test]
    fn message_index_to_element_index_handles_multiple_messages_same_day() {
        let messages = vec![
            msg(1, "Alice", "First", FEB_14_2026_10AM, false),
            msg(2, "Alice", "Second", FEB_14_2026_10AM + 60000, false),
            msg(3, "Bob", "Third", FEB_14_2026_10AM + 120000, false),
        ];
        let elements = build_message_list_elements(&messages);

        // Elements: [DateSeparator, Message1, Message2, Message3]
        assert_eq!(message_index_to_element_index(&elements, 0), Some(1));
        assert_eq!(message_index_to_element_index(&elements, 1), Some(2));
        assert_eq!(message_index_to_element_index(&elements, 2), Some(3));
    }

    #[test]
    fn message_index_to_element_index_returns_none_for_out_of_range() {
        let messages = vec![msg(1, "Alice", "Hello", FEB_14_2026_10AM, false)];
        let elements = build_message_list_elements(&messages);

        assert_eq!(message_index_to_element_index(&elements, 5), None);
    }

    #[test]
    fn message_index_to_element_index_returns_none_for_empty_elements() {
        let elements: Vec<MessageListElement> = vec![];

        assert_eq!(message_index_to_element_index(&elements, 0), None);
    }
}
