use ratatui::{
    style::Style,
    text::{Line, Span},
};
use unicode_width::UnicodeWidthStr;

use crate::domain::chat::ChatSummary;

use super::styles;
use super::text_utils::{
    format_chat_timestamp, normalize_preview_for_chat_row, truncate_to_display_width,
};

pub(super) fn chat_list_item_line(chat: &ChatSummary, width: usize) -> Line<'static> {
    use crate::domain::chat::ChatType;

    let timestamp = chat
        .last_message_unix_ms
        .map(format_chat_timestamp)
        .unwrap_or_else(|| "     ".to_owned());

    let raw_preview = chat
        .last_message_preview
        .as_deref()
        .map(normalize_preview_for_chat_row)
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "No messages yet".to_owned());

    let prefix_segments = build_preview_prefix_segments(chat);
    let prefix_total_width = prefix_segments_width(&prefix_segments);

    let outgoing_suffix = build_outgoing_status_suffix(chat);
    let outgoing_suffix_width = outgoing_suffix
        .as_ref()
        .map(|(t, _)| t.width())
        .unwrap_or(0);

    let unread_badge = if chat.unread_count > 0 {
        format!(" [{}]", chat.unread_count)
    } else {
        String::new()
    };

    let reaction_badge = if chat.unread_reaction_count > 0 {
        " [\u{2661}]" // ♡
    } else {
        ""
    };

    let online_indicator =
        if chat.chat_type == ChatType::Private && !chat.is_bot && chat.is_online == Some(true) {
            " \u{2022}" // bullet
        } else {
            ""
        };

    let fixed_prefix_width = 5 + 3; // timestamp (5) + " | " (3)
    let suffix_width = outgoing_suffix_width
        + reaction_badge.width()
        + unread_badge.width()
        + online_indicator.width();
    let name_width = chat.title.width();

    let content_width = fixed_prefix_width + name_width + 1 + prefix_total_width;
    let available_for_preview_and_padding = width.saturating_sub(content_width + suffix_width);

    let (display_preview, padding) =
        truncate_to_display_width(&raw_preview, available_for_preview_and_padding);

    let mut spans = vec![
        Span::styled(format!("{:>5}", timestamp), styles::timestamp_style()),
        Span::styled(" | ", styles::separator_style()),
        Span::styled(chat.title.clone(), styles::chat_name_style()),
        Span::raw(" "),
    ];

    for segment in prefix_segments {
        spans.push(Span::styled(segment.text, segment.style));
    }

    spans.push(Span::styled(display_preview, styles::chat_preview_style()));

    if padding > 0 {
        spans.push(Span::raw(" ".repeat(padding)));
    }

    if let Some((text, style)) = outgoing_suffix {
        spans.push(Span::styled(text, style));
    }

    if !online_indicator.is_empty() {
        spans.push(Span::styled(
            online_indicator.to_owned(),
            styles::online_indicator_style(),
        ));
    }

    if !reaction_badge.is_empty() {
        spans.push(Span::styled(
            reaction_badge.to_owned(),
            styles::reaction_badge_style(),
        ));
    }

    if !unread_badge.is_empty() {
        spans.push(Span::styled(unread_badge, styles::unread_count_style()));
    }

    Line::from(spans)
}

struct PrefixSegment {
    text: String,
    style: Style,
}

fn build_preview_prefix_segments(chat: &ChatSummary) -> Vec<PrefixSegment> {
    use crate::domain::chat::ChatType;

    let mut segments = Vec::new();

    if chat.chat_type == ChatType::Group {
        if let Some(ref sender) = chat.last_message_sender {
            segments.push(PrefixSegment {
                text: format!("{}: ", sender),
                style: styles::group_sender_style(),
            });
        }
    }

    segments
}

fn build_outgoing_status_suffix(chat: &ChatSummary) -> Option<(String, Style)> {
    if chat.outgoing_status.is_outgoing {
        let (text, style) = if chat.outgoing_status.is_read {
            (" \u{2713}\u{2713}", styles::outgoing_read_style())
        } else {
            (" \u{2713}", styles::outgoing_unread_style())
        };
        Some((text.to_owned(), style))
    } else {
        None
    }
}

fn prefix_segments_width(segments: &[PrefixSegment]) -> usize {
    segments.iter().map(|s| s.text.width()).sum()
}
