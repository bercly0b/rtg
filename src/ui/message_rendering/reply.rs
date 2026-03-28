//! Reply preview rendering.
//!
//! Builds the `│ Sender: truncated text` line shown above replied-to messages.

use ratatui::text::{Line, Span};

use crate::domain::message::ReplyInfo;
use crate::ui::styles;

/// Builds a reply preview line: `indent + "│ " + SenderName + ": " + truncated text`.
///
/// The reply text is truncated to fit within `content_width` on a single line
/// with an ellipsis (`…`) appended when truncated.
pub(super) fn build_reply_line(
    reply: &ReplyInfo,
    indent: &str,
    content_width: usize,
) -> Line<'static> {
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
        spans.push(Span::styled(
            sender_part,
            styles::reply_sender_style(&reply.sender_name, reply.is_outgoing),
        ));
    }

    spans.push(Span::styled(truncated, styles::reply_text_style()));

    Line::from(spans)
}

/// Truncates text to fit within `max_width` terminal columns.
///
/// If the text exceeds the width, it is cut and `…` is appended.
pub(super) fn truncate_to_width(text: &str, max_width: usize) -> String {
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
