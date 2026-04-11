//! Message line construction.
//!
//! Assembles the visual lines for a single message: header, reply preview,
//! wrapped content with link highlighting, metadata badges, and status indicators.

use ratatui::text::{Line, Span};

use crate::domain::message::{ForwardInfo, MessageStatus, ReplyInfo, TextLink};
use crate::ui::styles;

use super::content_spans::build_content_line_spans_linked;
use super::forward::build_forward_line;
use super::reply::build_reply_line;
use super::text_utils::wrap_line;

#[allow(clippy::too_many_arguments)]
pub(super) fn build_message_lines(
    time: &str,
    show_time: bool,
    sender: Option<&str>,
    is_outgoing: bool,
    content: &str,
    status: MessageStatus,
    file_meta: Option<&str>,
    reply_info: Option<&ReplyInfo>,
    forward_info: Option<&ForwardInfo>,
    reaction_count: u32,
    links: &[TextLink],
    max_width: usize,
    is_edited: bool,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let indent = "      "; // 6 spaces to align with time column
    let content_width = max_width.saturating_sub(indent.len());

    // Compute link byte ranges in content space.
    // Links have offsets into Message::text, but content may have a media label prefix.
    let link_offset_adj = if content.starts_with('[') {
        content.find('\n').map(|p| p + 1).unwrap_or(0)
    } else {
        0
    };
    let link_ranges: Vec<(usize, usize)> = links
        .iter()
        .map(|l| {
            (
                l.offset + link_offset_adj,
                l.offset + l.length + link_offset_adj,
            )
        })
        .collect();

    if sender.is_some() {
        // First message in group: header line (time + sender), then content on separate lines
        let header_line = build_message_header_line(time, show_time, sender, is_outgoing);
        lines.push(header_line);

        if let Some(reply) = reply_info {
            lines.push(build_reply_line(reply, indent, content_width));
        }

        if let Some(fwd) = forward_info {
            lines.push(build_forward_line(fwd, indent, content_width));
        }

        let mut content_pos = 0usize;
        for text_line in content.lines() {
            let mut seg_offset = 0;
            for wrapped in wrap_line(text_line, content_width) {
                let content_spans = build_content_line_spans_linked(
                    &wrapped,
                    content_pos + seg_offset,
                    &link_ranges,
                );
                let mut line_spans = vec![Span::raw(indent.to_owned())];
                line_spans.extend(content_spans);
                lines.push(Line::from(line_spans));
                seg_offset += wrapped.len();
            }
            content_pos += text_line.len() + 1; // +1 for '\n'
        }

        if content.is_empty() {
            lines.push(Line::from(vec![
                Span::raw(indent.to_owned()),
                Span::styled("[Empty message]".to_owned(), styles::message_media_style()),
            ]));
        }
    } else {
        // Grouped message (no sender): time/blank + first line of content on same line

        if let Some(reply) = reply_info {
            lines.push(build_reply_line(reply, indent, content_width));
        }

        if let Some(fwd) = forward_info {
            lines.push(build_forward_line(fwd, indent, content_width));
        }

        let time_span = if show_time {
            Span::styled(format!("{:>5} ", time), styles::message_time_style())
        } else {
            Span::raw(indent.to_owned())
        };

        let mut content_lines = content.lines();
        let mut content_pos = 0usize;

        if let Some(first_line) = content_lines.next() {
            let first_line_wrapped = wrap_line(first_line, content_width);
            let mut first_iter = first_line_wrapped.iter();
            let mut seg_offset = 0;

            if let Some(first_wrapped) = first_iter.next() {
                let mut spans = vec![time_span];
                spans.extend(build_content_line_spans_linked(
                    first_wrapped,
                    content_pos + seg_offset,
                    &link_ranges,
                ));
                lines.push(Line::from(spans));
                seg_offset += first_wrapped.len();

                for wrapped in first_iter {
                    let content_spans = build_content_line_spans_linked(
                        wrapped,
                        content_pos + seg_offset,
                        &link_ranges,
                    );
                    let mut line_spans = vec![Span::raw(indent.to_owned())];
                    line_spans.extend(content_spans);
                    lines.push(Line::from(line_spans));
                    seg_offset += wrapped.len();
                }
            }
            content_pos += first_line.len() + 1;

            // Remaining lines with indent
            for text_line in content_lines {
                let mut seg_offset = 0;
                for wrapped in wrap_line(text_line, content_width) {
                    let content_spans = build_content_line_spans_linked(
                        &wrapped,
                        content_pos + seg_offset,
                        &link_ranges,
                    );
                    let mut line_spans = vec![Span::raw(indent.to_owned())];
                    line_spans.extend(content_spans);
                    lines.push(Line::from(line_spans));
                    seg_offset += wrapped.len();
                }
                content_pos += text_line.len() + 1;
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

    // Append edited indicator on the same line as the last content line
    if is_edited {
        if let Some(last_line) = lines.last_mut() {
            last_line
                .spans
                .push(Span::styled(" edited", styles::message_edited_style()));
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
    // Fallback: append to the first content line (for media without bracket labels)
    if let Some(last_line) = lines.last_mut() {
        last_line.spans.push(Span::styled(
            format!(" {}", meta),
            styles::message_sending_style(),
        ));
    }
}

pub(super) fn build_message_header_line(
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
