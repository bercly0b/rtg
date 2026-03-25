//! Message input field rendering.

use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Padding, Paragraph, Wrap},
    Frame,
};

use crate::domain::{
    message_input_state::{MessageInputState, ReplyContext},
    shell_state::ActivePane,
};

use super::styles;

/// Placeholder text shown when the input is not focused and empty.
const PLACEHOLDER_TEXT: &str = "Press 'i' to type a message...";

/// Prompt symbol shown before the input text.
const PROMPT_SYMBOL: &str = "> ";

/// Returns the number of extra lines needed for the reply preview (0 or 1).
pub fn reply_preview_height(input_state: &MessageInputState) -> u16 {
    if input_state.reply_to().is_some() {
        1
    } else {
        0
    }
}

/// Renders the message input field, including a reply preview line if active.
pub fn render_message_input(
    frame: &mut Frame<'_>,
    area: Rect,
    input_state: &MessageInputState,
    active_pane: ActivePane,
) {
    let is_focused = active_pane == ActivePane::MessageInput;

    let mut lines: Vec<Line<'static>> = Vec::new();

    // Reply preview line (if reply context is active)
    if let Some(reply) = input_state.reply_to() {
        let effective_width = area.width.saturating_sub(2) as usize; // padding
        lines.push(build_reply_preview_line(reply, effective_width));
    }

    let reply_lines = lines.len() as u16;

    // Input line
    lines.push(build_input_line(input_state, is_focused));

    let paragraph = Paragraph::new(ratatui::text::Text::from(lines))
        .block(Block::new().padding(Padding::horizontal(1)))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);

    // Set cursor position when focused
    if is_focused {
        // Compute cursor position accounting for text wrapping.
        // Available width = area width - padding (1 left + 1 right)
        let effective_width = area.width.saturating_sub(2) as usize;
        let prompt_len = PROMPT_SYMBOL.len();
        let cursor_offset = prompt_len + input_state.cursor_position();

        let (cursor_row, cursor_col) = if effective_width == 0 {
            (0, cursor_offset)
        } else {
            (
                cursor_offset / effective_width,
                cursor_offset % effective_width,
            )
        };

        let cursor_x = area.x.saturating_add(1).saturating_add(cursor_col as u16);
        let cursor_y = area
            .y
            .saturating_add(reply_lines)
            .saturating_add(cursor_row as u16);
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}

/// Builds the line content for the input field.
fn build_input_line(input_state: &MessageInputState, is_focused: bool) -> Line<'static> {
    let prompt_style = styles::input_prompt_style();

    if is_focused {
        // Show prompt and current text (or empty)
        Line::from(vec![
            Span::styled(PROMPT_SYMBOL.to_owned(), prompt_style),
            Span::styled(input_state.text().to_owned(), styles::input_text_style()),
        ])
    } else if input_state.is_empty() {
        // Show placeholder when not focused and empty
        Line::from(vec![
            Span::styled(PROMPT_SYMBOL.to_owned(), prompt_style),
            Span::styled(
                PLACEHOLDER_TEXT.to_owned(),
                styles::input_placeholder_style(),
            ),
        ])
    } else {
        // Show existing text when not focused
        Line::from(vec![
            Span::styled(PROMPT_SYMBOL.to_owned(), prompt_style),
            Span::styled(input_state.text().to_owned(), styles::input_text_style()),
        ])
    }
}

fn build_reply_preview_line(reply: &ReplyContext, available_width: usize) -> Line<'static> {
    let bar = "│ ";
    let sender = if reply.sender_name.is_empty() {
        String::new()
    } else {
        format!("{}: ", reply.sender_name)
    };

    let content_prefix = format!("{}{}", bar, sender);
    let max_text_width = available_width.saturating_sub(content_prefix.chars().count());
    let first_line = reply.text.lines().next().unwrap_or("");
    let text = truncate_with_ellipsis(first_line, max_text_width);

    let mut spans = vec![Span::styled(bar.to_owned(), styles::reply_bar_style())];
    if !sender.is_empty() {
        spans.push(Span::styled(sender, styles::reply_sender_style()));
    }
    spans.push(Span::styled(text, styles::reply_text_style()));

    Line::from(spans)
}

fn truncate_with_ellipsis(text: &str, max_len: usize) -> String {
    if max_len == 0 {
        return String::new();
    }

    let char_count = text.chars().count();
    if char_count <= max_len {
        return text.to_owned();
    }

    if max_len == 1 {
        return "…".to_owned();
    }

    let mut out: String = text.chars().take(max_len - 1).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::message_input_state::ReplyContext;

    #[test]
    fn build_input_line_shows_placeholder_when_empty_and_unfocused() {
        let state = MessageInputState::default();
        let line = build_input_line(&state, false);

        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains(PLACEHOLDER_TEXT));
        assert!(text.starts_with(PROMPT_SYMBOL));
    }

    #[test]
    fn build_input_line_shows_empty_prompt_when_focused_and_empty() {
        let state = MessageInputState::default();
        let line = build_input_line(&state, true);

        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(!text.contains(PLACEHOLDER_TEXT));
        assert!(text.starts_with(PROMPT_SYMBOL));
    }

    #[test]
    fn build_input_line_shows_text_when_has_content() {
        let mut state = MessageInputState::default();
        state.insert_char('H');
        state.insert_char('i');

        let line = build_input_line(&state, false);
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();

        assert!(text.contains("Hi"));
        assert!(!text.contains(PLACEHOLDER_TEXT));
    }

    #[test]
    fn reply_preview_height_is_one_when_reply_set() {
        let mut state = MessageInputState::default();
        state.set_reply_to(ReplyContext {
            message_id: 1,
            sender_name: "Alice".to_owned(),
            text: "Hello".to_owned(),
        });

        assert_eq!(reply_preview_height(&state), 1);
    }

    #[test]
    fn build_reply_preview_line_contains_sender_and_text() {
        let reply = ReplyContext {
            message_id: 1,
            sender_name: "Alice".to_owned(),
            text: "Hello there".to_owned(),
        };

        let line = build_reply_preview_line(&reply, 80);
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();

        assert!(text.contains("│"));
        assert!(text.contains("Alice"));
        assert!(text.contains("Hello there"));
    }

    #[test]
    fn truncate_with_ellipsis_truncates_long_text() {
        assert_eq!(truncate_with_ellipsis("abcdef", 4), "abc…");
        assert_eq!(truncate_with_ellipsis("abc", 4), "abc");
    }
}
