use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use crate::domain::forum_topic::ForumTopicSummary;

use super::styles;
use super::text_utils::{
    format_chat_timestamp, normalize_preview_for_chat_row, truncate_to_display_width,
};

pub(super) fn forum_topic_list_item_line(topic: &ForumTopicSummary, width: usize) -> Line<'static> {
    let timestamp = topic
        .last_message_unix_ms
        .map(format_chat_timestamp)
        .unwrap_or_else(|| "     ".to_owned());

    let raw_preview = topic
        .last_message_preview
        .as_deref()
        .map(normalize_preview_for_chat_row)
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "No messages yet".to_owned());

    let unread_badge = if topic.unread_count > 0 {
        format!(" [{}]", topic.unread_count)
    } else {
        String::new()
    };

    let state_marker = if topic.is_hidden {
        " [hidden]"
    } else if topic.is_closed {
        " [closed]"
    } else {
        ""
    };

    let title = topic.name.clone();

    // " | " separator after timestamp; the topic title is followed by state
    // marker (optional), then preview takes the remaining space.
    let fixed_prefix_width = 5 + 3; // timestamp (5) + " | " (3)
    let suffix_width = unread_badge.width();
    let title_width = title.width();
    let state_marker_width = state_marker.width();
    let content_width = fixed_prefix_width + title_width + state_marker_width + 1; // +1 for space
    let available_for_preview_and_padding = width.saturating_sub(content_width + suffix_width);

    let (display_preview, padding) =
        truncate_to_display_width(&raw_preview, available_for_preview_and_padding);

    let mut spans = vec![
        Span::styled(format!("{:>5}", timestamp), styles::timestamp_style()),
        Span::styled(" | ", styles::separator_style()),
        Span::styled(title, styles::chat_name_style()),
    ];

    if !state_marker.is_empty() {
        spans.push(Span::styled(state_marker, styles::section_header_style()));
    }

    spans.push(Span::raw(" "));
    spans.push(Span::styled(display_preview, styles::chat_preview_style()));

    if padding > 0 {
        spans.push(Span::raw(" ".repeat(padding)));
    }

    if !unread_badge.is_empty() {
        spans.push(Span::styled(unread_badge, styles::unread_count_style()));
    }

    Line::from(spans)
}
