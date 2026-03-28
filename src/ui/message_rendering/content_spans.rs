//! Styled span construction for message content lines.
//!
//! Handles media indicator styling and hyperlink highlighting.

use ratatui::text::Span;

use crate::ui::styles;

/// Builds styled spans for a content line, highlighting media indicators and links.
///
/// `content_offset` is the byte offset of `text` within the full `content` string.
/// `link_ranges` contains `(start, end)` byte ranges in content space for links.
pub(super) fn build_content_line_spans_linked(
    text: &str,
    content_offset: usize,
    link_ranges: &[(usize, usize)],
) -> Vec<Span<'static>> {
    // Check if text starts with a media indicator like [Photo], [Voice], etc.
    if text.starts_with('[') {
        if let Some(end_bracket) = text.find(']') {
            let media_part = &text[..=end_bracket];
            let rest = text[end_bracket + 1..].trim_start();

            if rest.is_empty() {
                return vec![Span::styled(
                    media_part.to_owned(),
                    styles::message_media_style(),
                )];
            } else {
                return vec![
                    Span::styled(media_part.to_owned(), styles::message_media_style()),
                    Span::raw(" ".to_owned()),
                    Span::styled(rest.to_owned(), styles::message_text_style()),
                ];
            }
        }
    }

    // Build spans splitting at link boundaries
    let text_start = content_offset;
    let text_end = content_offset + text.len();

    let mut spans = Vec::new();
    let mut pos = 0usize; // byte position within `text`

    for &(link_start, link_end) in link_ranges {
        // Skip links that don't overlap with this text segment
        if link_end <= text_start || link_start >= text_end {
            continue;
        }
        let overlap_start = link_start.saturating_sub(text_start).max(pos);
        let overlap_end = (link_end - text_start).min(text.len());

        // Skip if offsets land on invalid char boundaries (defensive guard)
        if !text.is_char_boundary(overlap_start) || !text.is_char_boundary(overlap_end) {
            continue;
        }

        // Text before link
        if overlap_start > pos {
            spans.push(Span::styled(
                text[pos..overlap_start].to_owned(),
                styles::message_text_style(),
            ));
        }
        // Link text (underlined)
        if overlap_end > overlap_start {
            spans.push(Span::styled(
                text[overlap_start..overlap_end].to_owned(),
                styles::message_link_style(),
            ));
        }
        pos = overlap_end;
    }

    // Remaining text after all links
    if pos < text.len() {
        spans.push(Span::styled(
            text[pos..].to_owned(),
            styles::message_text_style(),
        ));
    }

    if spans.is_empty() {
        spans.push(Span::styled(text.to_owned(), styles::message_text_style()));
    }

    spans
}
