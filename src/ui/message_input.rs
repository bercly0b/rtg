//! Message input field rendering.

use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Padding, Paragraph, Wrap},
    Frame,
};

use crate::domain::{message_input_state::MessageInputState, shell_state::ActivePane};

use super::styles;

/// Placeholder text shown when the input is not focused and empty.
const PLACEHOLDER_TEXT: &str = "Press 'i' to type a message...";

/// Prompt symbol shown before the input text.
const PROMPT_SYMBOL: &str = "> ";

/// Renders the message input field.
pub fn render_message_input(
    frame: &mut Frame<'_>,
    area: Rect,
    input_state: &MessageInputState,
    active_pane: ActivePane,
) {
    let is_focused = active_pane == ActivePane::MessageInput;

    let line = build_input_line(input_state, is_focused);

    let paragraph = Paragraph::new(line)
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
        let cursor_y = area.y.saturating_add(cursor_row as u16);
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
