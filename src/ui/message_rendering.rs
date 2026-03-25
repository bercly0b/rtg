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
};

use crate::domain::message::{Message, MessageStatus, ReplyInfo};

use super::styles;

/// Represents a visual element in the messages list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageListElement {
    /// Date separator line (e.g., "——— 14 Feb 2026 ———").
    DateSeparator(String),
    /// A message with optional sender display.
    Message {
        time: String,
        show_time: bool,
        sender: Option<String>,
        is_outgoing: bool,
        content: String,
        status: MessageStatus,
        /// File metadata line (e.g. "download=yes, size=15.5KB, duration=0:03").
        file_meta: Option<String>,
        /// Reply preview: sender name and text of the replied-to message.
        reply_info: Option<ReplyInfo>,
        /// Total number of reactions on this message.
        reaction_count: u32,
    },
}

/// Builds a list of visual elements from messages.
///
/// Groups consecutive messages from the same sender and inserts date separators.
pub fn build_message_list_elements(messages: &[Message]) -> Vec<MessageListElement> {
    let mut elements = Vec::new();
    let mut prev_date: Option<chrono::NaiveDate> = None;
    let mut prev_sender: Option<&str> = None;
    let mut prev_time: Option<String> = None;

    for message in messages {
        let msg_date = timestamp_to_date(message.timestamp_ms);

        // Insert date separator if date changed
        if prev_date != Some(msg_date) {
            elements.push(MessageListElement::DateSeparator(format_date(msg_date)));
            prev_sender = None; // Reset sender grouping on date change
            prev_time = None;
        }

        let sender_name = effective_sender_name(message);
        let time = format_time(message.timestamp_ms);

        // Show sender only if different from previous message
        let show_sender = prev_sender != Some(sender_name);
        let sender = if show_sender {
            Some(sender_name.to_owned())
        } else {
            None
        };

        // Show time only on the first message in a same-sender group,
        // or when HH:MM changes within the group.
        let show_time = show_sender || prev_time.as_deref() != Some(&time);

        let file_meta = message
            .file_info
            .as_ref()
            .map(|fi| crate::domain::message::build_file_metadata_display(message.media, fi));

        elements.push(MessageListElement::Message {
            time: time.clone(),
            show_time,
            sender,
            is_outgoing: message.is_outgoing,
            content: message.display_content(),
            status: message.status,
            file_meta,
            reply_info: message.reply_to.clone(),
            reaction_count: message.reaction_count,
        });

        prev_date = Some(msg_date);
        prev_sender = Some(sender_name);
        prev_time = Some(time);
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

/// Converts a list element to `Text` for the custom `ChatMessageList` widget.
///
/// `max_width` is the available width in terminal columns for wrapping long lines.
/// Pass `0` to disable wrapping.
pub fn element_to_text(
    element: &MessageListElement,
    max_width: usize,
) -> ratatui::text::Text<'static> {
    match element {
        MessageListElement::DateSeparator(date) => {
            let separator = format!("——— {} ———", date);
            let line = Line::from(vec![Span::styled(
                separator,
                styles::date_separator_style(),
            )])
            .alignment(Alignment::Center);
            ratatui::text::Text::from(vec![Line::default(), line, Line::default()])
        }
        MessageListElement::Message {
            time,
            show_time,
            sender,
            is_outgoing,
            content,
            status,
            file_meta,
            reply_info,
            reaction_count,
        } => {
            let lines = build_message_lines(
                time,
                *show_time,
                sender.as_deref(),
                *is_outgoing,
                content,
                *status,
                file_meta.as_deref(),
                reply_info.as_ref(),
                *reaction_count,
                max_width,
            );
            ratatui::text::Text::from(lines)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build_message_lines(
    time: &str,
    show_time: bool,
    sender: Option<&str>,
    is_outgoing: bool,
    content: &str,
    status: MessageStatus,
    file_meta: Option<&str>,
    reply_info: Option<&ReplyInfo>,
    reaction_count: u32,
    max_width: usize,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let indent = "      "; // 6 spaces to align with time column
    let content_width = max_width.saturating_sub(indent.len());

    if sender.is_some() {
        // First message in group: header line (time + sender), then content on separate lines
        let header_line = build_message_header_line(time, show_time, sender, is_outgoing);
        lines.push(header_line);

        // Reply line (if replying to another message)
        if let Some(reply) = reply_info {
            lines.push(build_reply_line(reply, indent, content_width));
        }

        for text_line in content.lines() {
            for wrapped in wrap_line(text_line, content_width) {
                let content_spans = build_content_line_spans(&wrapped);
                let mut line_spans = vec![Span::raw(indent.to_owned())];
                line_spans.extend(content_spans);
                lines.push(Line::from(line_spans));
            }
        }

        if content.is_empty() {
            lines.push(Line::from(vec![
                Span::raw(indent.to_owned()),
                Span::styled("[Empty message]".to_owned(), styles::message_media_style()),
            ]));
        }
    } else {
        // Grouped message (no sender): time/blank + first line of content on same line

        // Reply line for grouped messages (shown before content)
        if let Some(reply) = reply_info {
            lines.push(build_reply_line(reply, indent, content_width));
        }

        let time_span = if show_time {
            Span::styled(format!("{:>5} ", time), styles::message_time_style())
        } else {
            Span::raw(indent.to_owned())
        };

        let mut content_lines = content.lines();

        if let Some(first_line) = content_lines.next() {
            let first_line_wrapped = wrap_line(first_line, content_width);
            let mut first_iter = first_line_wrapped.iter();

            if let Some(first_wrapped) = first_iter.next() {
                let mut spans = vec![time_span];
                spans.extend(build_content_line_spans(first_wrapped));
                lines.push(Line::from(spans));

                for wrapped in first_iter {
                    let content_spans = build_content_line_spans(wrapped);
                    let mut line_spans = vec![Span::raw(indent.to_owned())];
                    line_spans.extend(content_spans);
                    lines.push(Line::from(line_spans));
                }
            }

            // Remaining lines with indent
            for text_line in content_lines {
                for wrapped in wrap_line(text_line, content_width) {
                    let content_spans = build_content_line_spans(&wrapped);
                    let mut line_spans = vec![Span::raw(indent.to_owned())];
                    line_spans.extend(content_spans);
                    lines.push(Line::from(line_spans));
                }
            }
        } else {
            // Empty content
            let mut spans = vec![time_span];
            spans.push(Span::styled(
                "[Empty message]".to_owned(),
                styles::message_media_style(),
            ));
            lines.push(Line::from(spans));
        }
    }

    // Append file metadata on the same line as the media label
    if let Some(meta) = file_meta {
        append_file_meta_to_media_line(&mut lines, meta);
    }

    if reaction_count > 0 {
        append_reaction_indicator(&mut lines, reaction_count);
    }

    // Append sending status indicator on the same line as the last content line
    if status == MessageStatus::Sending {
        if let Some(last_line) = lines.last_mut() {
            last_line
                .spans
                .push(Span::styled(" sending...", styles::message_sending_style()));
        }
    }

    lines
}

fn append_reaction_indicator(lines: &mut [Line<'static>], reaction_count: u32) {
    if let Some(last_line) = lines.last_mut() {
        let badge = if reaction_count == 1 {
            " [♡]".to_owned()
        } else {
            format!(" [♡×{}]", reaction_count)
        };
        last_line
            .spans
            .push(Span::styled(badge, styles::message_reaction_style()));
    }
}

/// Appends file metadata to the line containing the `[Media]` indicator.
///
/// Finds the first content line that starts with a media bracket (after indent),
/// and appends the metadata as a DarkGray span on that same line.
fn append_file_meta_to_media_line(lines: &mut [Line<'static>], meta: &str) {
    for line in lines.iter_mut() {
        let has_media_bracket = line.spans.iter().any(|span| {
            let text = span.content.trim();
            text.starts_with('[') && text.contains(']')
        });
        if has_media_bracket {
            line.spans.push(Span::styled(
                format!(" {}", meta),
                styles::message_sending_style(),
            ));
            return;
        }
    }
}

fn build_message_header_line(
    time: &str,
    show_time: bool,
    sender: Option<&str>,
    is_outgoing: bool,
) -> Line<'static> {
    let time_span = if show_time {
        Span::styled(format!("{:>5} ", time), styles::message_time_style())
    } else {
        Span::raw("      ".to_owned()) // 6 spaces to preserve alignment
    };

    let mut spans = vec![time_span];

    if let Some(name) = sender {
        spans.push(Span::styled(
            format!("{}:", name),
            styles::sender_name_style(name, is_outgoing),
        ));
    }

    Line::from(spans)
}

/// Builds a reply preview line: `indent + "│ " + SenderName + ": " + truncated text`.
///
/// The reply text is truncated to fit within `content_width` on a single line
/// with an ellipsis (`…`) appended when truncated.
fn build_reply_line(reply: &ReplyInfo, indent: &str, content_width: usize) -> Line<'static> {
    use unicode_width::UnicodeWidthStr;

    let bar = "│ ";
    let bar_width = UnicodeWidthStr::width(bar);

    let sender_part = if reply.sender_name.is_empty() {
        String::new()
    } else {
        format!("{}: ", reply.sender_name)
    };
    let sender_width = UnicodeWidthStr::width(sender_part.as_str());

    let reply_text = reply.text.lines().next().unwrap_or("");
    let available = content_width
        .saturating_sub(bar_width)
        .saturating_sub(sender_width);
    let truncated = truncate_to_width(reply_text, available);

    let mut spans = vec![
        Span::raw(indent.to_owned()),
        Span::styled(bar.to_owned(), styles::reply_bar_style()),
    ];

    if !sender_part.is_empty() {
        spans.push(Span::styled(sender_part, styles::reply_sender_style()));
    }

    spans.push(Span::styled(truncated, styles::reply_text_style()));

    Line::from(spans)
}

/// Truncates text to fit within `max_width` terminal columns.
///
/// If the text exceeds the width, it is cut and `…` is appended.
fn truncate_to_width(text: &str, max_width: usize) -> String {
    use unicode_width::UnicodeWidthChar;

    if max_width == 0 {
        return String::new();
    }

    let total_width: usize = text
        .chars()
        .map(|c| UnicodeWidthChar::width(c).unwrap_or(0))
        .sum();

    if total_width <= max_width {
        return text.to_owned();
    }

    // Reserve 1 column for the ellipsis
    let target = max_width.saturating_sub(1);
    let mut width = 0;
    let mut result = String::new();

    for ch in text.chars() {
        let ch_w = UnicodeWidthChar::width(ch).unwrap_or(0);
        if width + ch_w > target {
            break;
        }
        result.push(ch);
        width += ch_w;
    }

    result.push('…');
    result
}

/// Wraps a text line to fit within `max_width` terminal columns.
///
/// Uses character-level breaking with Unicode width awareness.
/// Returns at least one element (possibly empty string for empty input).
fn wrap_line(text: &str, max_width: usize) -> Vec<String> {
    use unicode_width::UnicodeWidthChar;

    if max_width == 0 || text.is_empty() {
        return vec![text.to_owned()];
    }

    let text_width: usize = text
        .chars()
        .map(|c| UnicodeWidthChar::width(c).unwrap_or(0))
        .sum();
    if text_width <= max_width {
        return vec![text.to_owned()];
    }

    let mut result = Vec::new();
    let mut current = String::new();
    let mut current_width = 0;

    for ch in text.chars() {
        let ch_w = UnicodeWidthChar::width(ch).unwrap_or(0);
        if current_width + ch_w > max_width && !current.is_empty() {
            result.push(std::mem::take(&mut current));
            current_width = 0;
        }
        current.push(ch);
        current_width += ch_w;
    }
    if !current.is_empty() {
        result.push(current);
    }
    if result.is_empty() {
        result.push(String::new());
    }
    result
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

    fn msg(id: i64, sender: &str, text: &str, ts_ms: i64, outgoing: bool) -> Message {
        Message {
            id,
            sender_name: sender.to_owned(),
            text: text.to_owned(),
            timestamp_ms: ts_ms,
            is_outgoing: outgoing,
            media: MessageMedia::None,
            status: crate::domain::message::MessageStatus::Delivered,
            file_info: None,
            reply_to: None,
            reaction_count: 0,
        }
    }

    fn msg_with_media(
        id: i64,
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
            status: crate::domain::message::MessageStatus::Delivered,
            file_info: None,
            reply_to: None,
            reaction_count: 0,
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
    fn outgoing_message_sets_is_outgoing_flag() {
        let messages = vec![msg(1, "MyName", "Hello", FEB_14_2026_10AM, true)];

        let elements = build_message_list_elements(&messages);

        if let MessageListElement::Message {
            is_outgoing,
            sender,
            ..
        } = &elements[1]
        {
            assert!(is_outgoing, "Outgoing message should have is_outgoing=true");
            assert_eq!(sender.as_deref(), Some("You"));
        } else {
            panic!("Expected Message element");
        }
    }

    #[test]
    fn incoming_message_sets_is_outgoing_false() {
        let messages = vec![msg(1, "Alice", "Hi", FEB_14_2026_10AM, false)];

        let elements = build_message_list_elements(&messages);

        if let MessageListElement::Message { is_outgoing, .. } = &elements[1] {
            assert!(
                !is_outgoing,
                "Incoming message should have is_outgoing=false"
            );
        } else {
            panic!("Expected Message element");
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
            assert_eq!(content, "[Photo]\nCheck this out");
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

    #[test]
    fn hides_duplicate_time_for_same_sender_same_minute() {
        // Two messages from Alice at exactly the same timestamp (same HH:MM)
        let messages = vec![
            msg(1, "Alice", "First", FEB_14_2026_10AM, false),
            msg(2, "Alice", "Second", FEB_14_2026_10AM + 5000, false), // +5s, same minute
        ];

        let elements = build_message_list_elements(&messages);

        // First message should show time
        if let MessageListElement::Message { show_time, .. } = &elements[1] {
            assert!(show_time, "First message in group should show time");
        } else {
            panic!("Expected Message element");
        }

        // Second message (same sender, same minute) should hide time
        if let MessageListElement::Message { show_time, .. } = &elements[2] {
            assert!(!show_time, "Same sender + same minute should hide time");
        } else {
            panic!("Expected Message element");
        }
    }

    #[test]
    fn shows_time_when_minute_changes_within_same_sender_group() {
        // Two messages from Alice, 1 minute apart (different HH:MM)
        let messages = vec![
            msg(1, "Alice", "First", FEB_14_2026_10AM, false),
            msg(2, "Alice", "Second", FEB_14_2026_10AM + 60000, false), // +1 min
        ];

        let elements = build_message_list_elements(&messages);

        if let MessageListElement::Message { show_time, .. } = &elements[1] {
            assert!(show_time, "First message should show time");
        }

        if let MessageListElement::Message { show_time, .. } = &elements[2] {
            assert!(show_time, "Different minute in same group should show time");
        }
    }

    #[test]
    fn shows_time_when_sender_changes_even_if_same_minute() {
        // Same timestamp but different senders
        let messages = vec![
            msg(1, "Alice", "Hi", FEB_14_2026_10AM, false),
            msg(2, "Bob", "Hello", FEB_14_2026_10AM + 5000, false), // same minute
        ];

        let elements = build_message_list_elements(&messages);

        if let MessageListElement::Message { show_time, .. } = &elements[1] {
            assert!(show_time, "First message should show time");
        }

        if let MessageListElement::Message { show_time, .. } = &elements[2] {
            assert!(
                show_time,
                "Different sender should always show time even if same minute"
            );
        }
    }

    #[test]
    fn resets_time_grouping_on_date_change() {
        let messages = vec![
            msg(1, "Alice", "Day 1", FEB_14_2026_10AM, false),
            msg(2, "Alice", "Day 2", FEB_15_2026_1PM, false),
        ];

        let elements = build_message_list_elements(&messages);

        // Both messages should show time (date separator resets grouping)
        if let MessageListElement::Message { show_time, .. } = &elements[1] {
            assert!(show_time, "First message should show time");
        }

        if let MessageListElement::Message { show_time, .. } = &elements[3] {
            assert!(show_time, "Message after date change should show time");
        }
    }

    #[test]
    fn first_message_always_shows_time() {
        let messages = vec![msg(1, "Alice", "Hello", FEB_14_2026_10AM, false)];

        let elements = build_message_list_elements(&messages);

        if let MessageListElement::Message { show_time, .. } = &elements[1] {
            assert!(show_time, "Single message should always show time");
        }
    }

    #[test]
    fn three_messages_same_sender_same_minute_only_first_shows_time() {
        let messages = vec![
            msg(1, "Alice", "One", FEB_14_2026_10AM, false),
            msg(2, "Alice", "Two", FEB_14_2026_10AM + 10_000, false), // +10s
            msg(3, "Alice", "Three", FEB_14_2026_10AM + 20_000, false), // +20s
        ];

        let elements = build_message_list_elements(&messages);

        if let MessageListElement::Message { show_time, .. } = &elements[1] {
            assert!(show_time, "First should show time");
        }
        if let MessageListElement::Message { show_time, .. } = &elements[2] {
            assert!(!show_time, "Second should hide time");
        }
        if let MessageListElement::Message { show_time, .. } = &elements[3] {
            assert!(!show_time, "Third should hide time");
        }
    }

    // ── wrap_line tests ──

    #[test]
    fn wrap_line_short_text_no_wrapping() {
        let result = wrap_line("hello", 10);
        assert_eq!(result, vec!["hello"]);
    }

    #[test]
    fn wrap_line_exact_fit() {
        let result = wrap_line("12345", 5);
        assert_eq!(result, vec!["12345"]);
    }

    #[test]
    fn wrap_line_splits_long_text() {
        let result = wrap_line("abcdefghij", 5);
        assert_eq!(result, vec!["abcde", "fghij"]);
    }

    #[test]
    fn wrap_line_splits_into_three() {
        let result = wrap_line("abcdefghijklmno", 5);
        assert_eq!(result, vec!["abcde", "fghij", "klmno"]);
    }

    #[test]
    fn wrap_line_handles_remainder() {
        let result = wrap_line("abcdefgh", 5);
        assert_eq!(result, vec!["abcde", "fgh"]);
    }

    #[test]
    fn wrap_line_empty_text() {
        let result = wrap_line("", 10);
        assert_eq!(result, vec![""]);
    }

    #[test]
    fn wrap_line_zero_width_returns_original() {
        let result = wrap_line("hello", 0);
        assert_eq!(result, vec!["hello"]);
    }

    #[test]
    fn wrap_line_unicode_emoji() {
        // 🚀 is 2 cells wide, "ab" is 2 cells. Total = 4 cells.
        let result = wrap_line("🚀ab", 4);
        assert_eq!(result, vec!["🚀ab"]);
    }

    #[test]
    fn wrap_line_unicode_emoji_wraps_correctly() {
        // 🚀 = 2 cells, a = 1 cell. Total "🚀a🚀b" = 2+1+2+1 = 6 cells.
        // With width 4: first line "🚀a" (3 cells), second "🚀b" (3 cells)
        let result = wrap_line("🚀a🚀b", 4);
        assert_eq!(result, vec!["🚀a", "🚀b"]);
    }

    // ── sending status inline tests ──

    #[test]
    fn sending_status_on_same_line_as_content() {
        let messages = vec![Message {
            id: 1,
            sender_name: "User".to_owned(),
            text: "Hello".to_owned(),
            timestamp_ms: FEB_14_2026_10AM,
            is_outgoing: true,
            media: MessageMedia::None,
            status: crate::domain::message::MessageStatus::Sending,
            file_info: None,
            reply_to: None,
            reaction_count: 0,
        }];

        let elements = build_message_list_elements(&messages);

        // The message element should NOT have a separate "sending..." line
        let msg_text = element_to_text(&elements[1], 80);
        let line_count = msg_text.lines.len();

        // Header line (time + sender) + content line with "sending..." appended = 2 lines
        assert_eq!(
            line_count, 2,
            "Expected header + content/status on same line, got {} lines",
            line_count
        );

        // Verify "sending..." is on the content line, not a separate line
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
            status: crate::domain::message::MessageStatus::Delivered,
            file_info: None,
            reply_to: None,
            reaction_count: 0,
        }];

        let elements = build_message_list_elements(&messages);
        let msg_text = element_to_text(&elements[1], 80);

        // Should be 2 lines: header + content (no "sending..." at all)
        assert_eq!(msg_text.lines.len(), 2);
        let all_text: String = msg_text
            .lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(!all_text.contains("sending..."));
    }

    // ── media on separate line tests ──

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

        // Header line + [Photo] line + text line = 3 lines
        assert_eq!(
            msg_text.lines.len(),
            3,
            "Expected 3 lines (header + media + text), got {}",
            msg_text.lines.len()
        );

        // Second line should contain [Photo]
        let media_line: String = msg_text.lines[1]
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert!(
            media_line.contains("[Photo]"),
            "Second line should be media indicator"
        );

        // Third line should contain the text
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

        // Header line + [Photo] line = 2 lines
        assert_eq!(msg_text.lines.len(), 2);
    }

    // ── text wrapping in message rendering ──

    #[test]
    fn long_message_wraps_within_width() {
        let long_text = "a".repeat(50);
        let messages = vec![msg(1, "Alice", &long_text, FEB_14_2026_10AM, false)];

        let elements = build_message_list_elements(&messages);
        // Use narrow width: 30 total - 6 indent = 24 content width
        let msg_text = element_to_text(&elements[1], 30);

        // Header (1) + wrapped content lines (50 chars / 24 = 3 lines)
        assert!(
            msg_text.lines.len() >= 3,
            "Long text should wrap into multiple lines, got {} lines",
            msg_text.lines.len()
        );
    }

    // ── file metadata display tests ──

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
            status: crate::domain::message::MessageStatus::Delivered,
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
            reply_to: None,
            reaction_count: 0,
        }];

        let elements = build_message_list_elements(&messages);

        // Check that the element has file_meta
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

        // Check that metadata is rendered inline with the media label
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
    fn text_message_has_no_file_metadata() {
        let messages = vec![msg(1, "Alice", "Hello", FEB_14_2026_10AM, false)];

        let elements = build_message_list_elements(&messages);

        if let MessageListElement::Message { file_meta, .. } = &elements[1] {
            assert!(file_meta.is_none(), "text message should have no file_meta");
        } else {
            panic!("Expected Message element");
        }
    }

    // ── reply rendering tests ──

    fn msg_with_reply(
        id: i64,
        sender: &str,
        text: &str,
        ts_ms: i64,
        reply_sender: &str,
        reply_text: &str,
    ) -> Message {
        Message {
            id,
            sender_name: sender.to_owned(),
            text: text.to_owned(),
            timestamp_ms: ts_ms,
            is_outgoing: false,
            media: MessageMedia::None,
            status: crate::domain::message::MessageStatus::Delivered,
            file_info: None,
            reply_to: Some(ReplyInfo {
                sender_name: reply_sender.to_owned(),
                text: reply_text.to_owned(),
            }),
            reaction_count: 0,
        }
    }

    #[test]
    fn message_with_multiple_reactions_shows_count() {
        let messages = vec![Message {
            id: 1,
            sender_name: "Alice".to_owned(),
            text: "Hello".to_owned(),
            timestamp_ms: FEB_14_2026_10AM,
            is_outgoing: false,
            media: MessageMedia::None,
            status: crate::domain::message::MessageStatus::Delivered,
            file_info: None,
            reply_to: None,
            reaction_count: 3,
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
            status: crate::domain::message::MessageStatus::Delivered,
            file_info: None,
            reply_to: None,
            reaction_count: 1,
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

        // Reply line should contain bar and sender
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

        // Header + content = 2 lines (no reply)
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
        // No ": " prefix from empty sender
        assert!(!reply_line.contains(": Some"));
    }

    // ── truncate_to_width tests ──

    #[test]
    fn truncate_short_text_unchanged() {
        assert_eq!(truncate_to_width("hello", 10), "hello");
    }

    #[test]
    fn truncate_exact_fit_unchanged() {
        assert_eq!(truncate_to_width("hello", 5), "hello");
    }

    #[test]
    fn truncate_long_text_adds_ellipsis() {
        let result = truncate_to_width("hello world", 8);
        assert!(
            result.ends_with('…'),
            "Should end with ellipsis: '{}'",
            result
        );
        assert!(
            result.len() <= 10,
            "Truncated should be short: '{}'",
            result
        );
        assert!(
            result.starts_with("hello w"),
            "Should keep prefix: '{}'",
            result
        );
    }

    #[test]
    fn truncate_zero_width_returns_empty() {
        assert_eq!(truncate_to_width("hello", 0), "");
    }

    #[test]
    fn truncate_width_one_returns_ellipsis_for_long() {
        let result = truncate_to_width("hello", 1);
        assert_eq!(result, "…");
    }

    #[test]
    fn truncate_empty_text() {
        assert_eq!(truncate_to_width("", 10), "");
    }

    #[test]
    fn truncate_unicode_text() {
        let result = truncate_to_width("Привет мир!", 8);
        assert!(result.ends_with('…'));
    }

    // ── reply in grouped message ──

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
                status: crate::domain::message::MessageStatus::Delivered,
                file_info: None,
                reply_to: Some(ReplyInfo {
                    sender_name: "Bob".to_owned(),
                    text: "Original text".to_owned(),
                }),
                reaction_count: 0,
            },
        ];

        let elements = build_message_list_elements(&messages);

        // Second message is grouped (no sender)
        if let MessageListElement::Message {
            sender, reply_info, ..
        } = &elements[2]
        {
            assert!(sender.is_none(), "Should be grouped (no sender)");
            assert!(reply_info.is_some(), "Should have reply_info");
        } else {
            panic!("Expected Message element");
        }

        // Render and verify reply line exists
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
}
