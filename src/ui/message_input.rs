//! Message input field rendering.

use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Padding, Paragraph},
    Frame,
};
use unicode_width::UnicodeWidthChar;

use crate::domain::{
    message_input_state::{EditContext, MessageInputState, ReplyContext},
    shell_state::ActivePane,
};

use super::styles;

/// Placeholder text shown when the input is not focused and empty.
const PLACEHOLDER_TEXT: &str = "Press 'i' to type a message...";

/// Prompt symbol shown before the input text.
const PROMPT_SYMBOL: &str = "> ";

/// Returns the number of extra lines needed for the context preview (0 or 1).
pub fn reply_preview_height(input_state: &MessageInputState) -> u16 {
    if input_state.reply_to().is_some() || input_state.editing().is_some() {
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

    // Context preview line (reply or editing)
    if let Some(editing) = input_state.editing() {
        let effective_width = area.width.saturating_sub(2) as usize;
        lines.push(build_editing_preview_line(editing, effective_width));
    } else if let Some(reply) = input_state.reply_to() {
        let effective_width = area.width.saturating_sub(2) as usize; // padding
        lines.push(build_reply_preview_line(reply, effective_width));
    }

    let reply_lines = lines.len() as u16;

    let effective_width = area.width.saturating_sub(2) as usize;

    let (input_lines, cursor_row, cursor_col) =
        wrap_input_with_cursor(input_state, is_focused, effective_width);
    lines.extend(input_lines);

    let paragraph = Paragraph::new(ratatui::text::Text::from(lines))
        .block(Block::new().padding(Padding::horizontal(1)));

    frame.render_widget(paragraph, area);

    if is_focused {
        let cursor_x = area.x.saturating_add(1).saturating_add(cursor_col);
        let cursor_y = area
            .y
            .saturating_add(reply_lines)
            .saturating_add(cursor_row);
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}

fn wrap_input_with_cursor(
    input_state: &MessageInputState,
    is_focused: bool,
    max_width: usize,
) -> (Vec<Line<'static>>, u16, u16) {
    if !is_focused {
        let text_style = if input_state.is_empty() {
            styles::input_placeholder_style()
        } else {
            styles::input_text_style()
        };
        let display_text = if input_state.is_empty() {
            PLACEHOLDER_TEXT
        } else {
            input_state.text()
        };
        let line = Line::from(vec![
            Span::styled(PROMPT_SYMBOL.to_owned(), styles::input_prompt_style()),
            Span::styled(display_text.to_owned(), text_style),
        ]);
        return (vec![line], 0, 0);
    }

    let full_text: String = format!("{}{}", PROMPT_SYMBOL, input_state.text());
    let prompt_char_count = PROMPT_SYMBOL.chars().count();
    let cursor_char_idx = prompt_char_count + input_state.cursor_position();

    if max_width == 0 {
        let line = Line::from(vec![
            Span::styled(PROMPT_SYMBOL.to_owned(), styles::input_prompt_style()),
            Span::styled(input_state.text().to_owned(), styles::input_text_style()),
        ]);
        return (vec![line], 0, cursor_char_idx as u16);
    }

    let mut visual_lines: Vec<String> = Vec::new();
    let mut current_line = String::new();
    let mut current_width: usize = 0;
    let mut cursor_row: usize = 0;
    let mut cursor_col: usize = 0;
    let mut char_idx: usize = 0;

    for ch in full_text.chars() {
        let ch_w = UnicodeWidthChar::width(ch).unwrap_or(0);

        if current_width + ch_w > max_width && !current_line.is_empty() {
            visual_lines.push(std::mem::take(&mut current_line));
            current_width = 0;
        }

        if char_idx == cursor_char_idx {
            cursor_row = visual_lines.len();
            cursor_col = current_width;
        }

        current_line.push(ch);
        current_width += ch_w;
        char_idx += 1;
    }

    if !current_line.is_empty() {
        visual_lines.push(current_line);
    }
    if visual_lines.is_empty() {
        visual_lines.push(String::new());
    }

    if cursor_char_idx >= char_idx {
        cursor_row = visual_lines.len().saturating_sub(1);
        let last = visual_lines.last().unwrap_or(&String::new()).clone();
        cursor_col = last
            .chars()
            .map(|c| UnicodeWidthChar::width(c).unwrap_or(0))
            .sum();
    }

    let styled_lines: Vec<Line<'static>> = visual_lines
        .into_iter()
        .enumerate()
        .map(|(i, line_text)| {
            if i == 0 {
                let text_part = if line_text.len() >= PROMPT_SYMBOL.len() {
                    &line_text[PROMPT_SYMBOL.len()..]
                } else {
                    ""
                };
                Line::from(vec![
                    Span::styled(PROMPT_SYMBOL.to_owned(), styles::input_prompt_style()),
                    Span::styled(text_part.to_owned(), styles::input_text_style()),
                ])
            } else {
                Line::from(Span::styled(line_text, styles::input_text_style()))
            }
        })
        .collect();

    (styled_lines, cursor_row as u16, cursor_col as u16)
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
        let is_outgoing = reply.sender_name == "You";
        spans.push(Span::styled(
            sender,
            styles::reply_sender_style(&reply.sender_name, is_outgoing),
        ));
    }
    spans.push(Span::styled(text, styles::reply_text_style()));

    Line::from(spans)
}

fn build_editing_preview_line(editing: &EditContext, available_width: usize) -> Line<'static> {
    let bar = "│ ";
    let label = "Editing: ";
    let prefix_len = bar.chars().count() + label.chars().count();
    let max_text_width = available_width.saturating_sub(prefix_len);
    let first_line = editing.original_text.lines().next().unwrap_or("");
    let text = truncate_with_ellipsis(first_line, max_text_width);

    Line::from(vec![
        Span::styled(bar.to_owned(), styles::reply_bar_style()),
        Span::styled(
            label.to_owned(),
            styles::reply_sender_style("Editing", false),
        ),
        Span::styled(text, styles::reply_text_style()),
    ])
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

    fn lines_to_text(lines: &[Line<'_>]) -> String {
        lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn shows_placeholder_when_empty_and_unfocused() {
        let state = MessageInputState::default();
        let (lines, _, _) = wrap_input_with_cursor(&state, false, 80);
        let text = lines_to_text(&lines);
        assert!(text.contains(PLACEHOLDER_TEXT));
        assert!(text.starts_with(PROMPT_SYMBOL));
    }

    #[test]
    fn shows_empty_prompt_when_focused_and_empty() {
        let state = MessageInputState::default();
        let (lines, _, _) = wrap_input_with_cursor(&state, true, 80);
        let text = lines_to_text(&lines);
        assert!(!text.contains(PLACEHOLDER_TEXT));
        assert!(text.starts_with(PROMPT_SYMBOL));
    }

    #[test]
    fn shows_text_when_has_content() {
        let mut state = MessageInputState::default();
        state.insert_char('H');
        state.insert_char('i');
        let (lines, _, _) = wrap_input_with_cursor(&state, false, 80);
        let text = lines_to_text(&lines);
        assert!(text.contains("Hi"));
        assert!(!text.contains(PLACEHOLDER_TEXT));
    }

    #[test]
    fn cursor_position_on_first_line() {
        let mut state = MessageInputState::default();
        for ch in "Hello".chars() {
            state.insert_char(ch);
        }
        let (_, row, col) = wrap_input_with_cursor(&state, true, 80);
        assert_eq!(row, 0);
        assert_eq!(col, 7); // "> " (2) + "Hello" (5)
    }

    #[test]
    fn cursor_wraps_to_second_line() {
        let mut state = MessageInputState::default();
        // width=10, prompt "> " takes 2, so 8 chars fit on first line
        for ch in "abcdefghij".chars() {
            state.insert_char(ch);
        }
        // "> abcdefgh" = 10 chars on line 0, "ij" on line 1, cursor after "ij"
        let (lines, row, col) = wrap_input_with_cursor(&state, true, 10);
        assert_eq!(lines.len(), 2);
        assert_eq!(row, 1);
        assert_eq!(col, 2);
    }

    #[test]
    fn cursor_at_wrap_boundary() {
        let mut state = MessageInputState::default();
        // width=10, prompt "> " = 2, so first line fits 8 text chars exactly
        for ch in "abcdefgh".chars() {
            state.insert_char(ch);
        }
        // "> abcdefgh" = exactly 10, cursor at end of line 0
        let (lines, row, col) = wrap_input_with_cursor(&state, true, 10);
        assert_eq!(lines.len(), 1);
        assert_eq!(row, 0);
        assert_eq!(col, 10);
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
