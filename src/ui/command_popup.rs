//! Renders a centered popup overlay showing external command output.
//!
//! This popup is reusable for any command execution scenario:
//! voice recording, audio playback, image viewing, etc.

use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph},
    Frame,
};

use crate::domain::command_popup_state::{CommandPhase, CommandPopupState};

use super::{popup_utils, styles};

/// Renders the command popup as an overlay on top of existing content.
pub fn render_command_popup(frame: &mut Frame<'_>, area: Rect, state: &CommandPopupState) {
    let popup_area = popup_utils::centered_rect(area, 60, 60);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(format!(" {} ", state.title()))
        .borders(Borders::ALL)
        .border_style(styles::command_popup_border_style())
        .padding(Padding::new(1, 1, 1, 1));

    let mut lines: Vec<Line<'_>> = state
        .visible_lines()
        .into_iter()
        .map(|line| {
            Line::from(Span::styled(
                line.to_owned(),
                styles::command_popup_output_style(),
            ))
        })
        .collect();

    lines.push(Line::from(""));
    lines.push(footer_line(state.phase()));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, popup_area);
}

fn footer_line(phase: &CommandPhase) -> Line<'static> {
    let text = match phase {
        CommandPhase::Running => "Press q to stop",
        CommandPhase::AwaitingConfirmation { prompt } => {
            return Line::from(Span::styled(
                prompt.clone(),
                styles::command_popup_footer_style(),
            ));
        }
    };
    Line::from(Span::styled(
        text.to_owned(),
        styles::command_popup_footer_style(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn footer_line_running_shows_stop_hint() {
        let line = footer_line(&CommandPhase::Running);
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(text, "Press q to stop");
    }

    #[test]
    fn footer_line_awaiting_shows_prompt() {
        let line = footer_line(&CommandPhase::AwaitingConfirmation {
            prompt: "Send recording? (y/n)".into(),
        });
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(text, "Send recording? (y/n)");
    }

    #[test]
    fn footer_line_uses_footer_style() {
        let line = footer_line(&CommandPhase::Running);
        assert_eq!(line.spans[0].style, styles::command_popup_footer_style());
    }
}
