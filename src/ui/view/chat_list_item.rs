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

    let outgoing_suffix = build_outgoing_status_suffix(chat);
    let outgoing_suffix_width = outgoing_suffix
        .as_ref()
        .map(|(t, _)| t.width())
        .unwrap_or(0);

    // Forums show the number of unread topics, not the chat-level message
    // count — TDLib's `unread_count` is unreliable for forums (see ChatSummary).
    let badge_count = if chat.is_forum {
        chat.unread_topic_count.unwrap_or(0)
    } else {
        chat.unread_count
    };
    let unread_badge = if badge_count > 0 {
        format!(" [{}]", badge_count)
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

    // The unread badge and trailing icons must always be visible: reserve their
    // width up front and truncate the middle (title + sender prefix + preview)
    // into whatever budget remains. Segments lay out left-to-right and truncate
    // from the tail — the preview yields first, then the sender prefix, then the
    // title — so the suffix is never pushed off the row by a long title.
    let middle_budget = width.saturating_sub(fixed_prefix_width + suffix_width);

    let mut middle_segments = Vec::with_capacity(prefix_segments.len() + 3);
    middle_segments.push(PrefixSegment {
        text: chat.title.clone(),
        style: styles::chat_name_style(),
    });
    middle_segments.push(PrefixSegment {
        text: " ".to_owned(),
        style: Style::default(),
    });
    middle_segments.extend(prefix_segments);
    middle_segments.push(PrefixSegment {
        text: raw_preview,
        style: styles::chat_preview_style(),
    });

    let (middle_spans, middle_used) = truncate_segments_to_width(middle_segments, middle_budget);
    let padding = middle_budget.saturating_sub(middle_used);

    let mut spans = vec![
        Span::styled(format!("{:>5}", timestamp), styles::timestamp_style()),
        Span::styled(" | ", styles::separator_style()),
    ];
    spans.extend(middle_spans);

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
    use crate::domain::chat::OutgoingReadStatus;

    match chat.outgoing_status {
        OutgoingReadStatus::NotOutgoing => None,
        OutgoingReadStatus::Outgoing { is_read } => {
            let (text, style) = if is_read {
                (" \u{2713}\u{2713}", styles::outgoing_read_style())
            } else {
                (" \u{2713}", styles::outgoing_unread_style())
            };
            Some((text.to_owned(), style))
        }
    }
}

/// Lays out `segments` left-to-right into spans, capped at `max_width`.
///
/// Earlier segments are placed first; the segment that overflows the budget is
/// truncated (with an ellipsis) and any remaining segments are dropped. Returns
/// the spans and the total display width they consumed (`<= max_width`).
fn truncate_segments_to_width(
    segments: Vec<PrefixSegment>,
    max_width: usize,
) -> (Vec<Span<'static>>, usize) {
    let mut spans = Vec::with_capacity(segments.len());
    let mut used = 0usize;

    for segment in segments {
        let remaining = max_width.saturating_sub(used);
        if remaining == 0 {
            break;
        }

        let seg_width = segment.text.width();
        if seg_width <= remaining {
            used += seg_width;
            spans.push(Span::styled(segment.text, segment.style));
        } else {
            let (truncated, _) = truncate_to_display_width(&segment.text, remaining);
            if !truncated.is_empty() {
                used += truncated.width();
                spans.push(Span::styled(truncated, segment.style));
            }
            break;
        }
    }

    (spans, used)
}
