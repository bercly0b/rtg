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

use crate::domain::command_popup_state::{CommandPhase, CommandPopupKind, CommandPopupState};

use super::{popup_utils, styles};

/// Lines reserved for the footer section (empty separator + footer text).
const FOOTER_LINES: u16 = 2;
/// Lines consumed by borders (top + bottom) and padding (top + bottom).
/// Derived from: Borders::ALL (2) + Padding::new(1, 1, 1, 1) vertical (2).
const CHROME_LINES: u16 = 4;

/// Renders the command popup as an overlay on top of existing content.
pub fn render_command_popup(frame: &mut Frame<'_>, area: Rect, state: &CommandPopupState) {
    let popup_area = popup_utils::centered_rect(area, 60, 60);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(format!(" {} ", state.title()))
        .borders(Borders::ALL)
        .border_style(styles::command_popup_border_style())
        .padding(Padding::new(1, 1, 1, 1));

    // Compute how many output lines fit without clipping the footer.
    let max_output_lines = popup_area
        .height
        .saturating_sub(CHROME_LINES + FOOTER_LINES) as usize;

    let mut lines: Vec<Line<'_>> = state
        .visible_lines(max_output_lines)
        .into_iter()
        .map(|line| {
            Line::from(Span::styled(
                line.to_owned(),
                styles::command_popup_output_style(),
            ))
        })
        .collect();

    lines.push(Line::from(""));
    lines.push(footer_line(state.phase(), state.kind()));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, popup_area);
}

fn footer_line(phase: &CommandPhase, kind: CommandPopupKind) -> Line<'static> {
    match phase {
        CommandPhase::Running => {
            let hint = match kind {
                CommandPopupKind::Recording => "Press q to stop",
                CommandPopupKind::Playback | CommandPopupKind::Viewer => "Press q or Esc to stop",
            };
            Line::from(Span::styled(
                hint.to_owned(),
                styles::command_popup_footer_style(),
            ))
        }
        CommandPhase::Stopping => Line::from(Span::styled(
            "Stopping...".to_owned(),
            styles::command_popup_footer_style(),
        )),
        CommandPhase::AwaitingConfirmation { prompt } => Line::from(Span::styled(
            prompt.clone(),
            styles::command_popup_footer_style(),
        )),
        CommandPhase::Done => Line::from(Span::styled(
            "Press any key to close".to_owned(),
            styles::command_popup_footer_style(),
        )),
        CommandPhase::Failed { message } => Line::from(Span::styled(
            message.clone(),
            styles::command_popup_error_style(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn footer_line_running_recording_shows_q_only_stop_hint() {
        let line = footer_line(&CommandPhase::Running, CommandPopupKind::Recording);
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(text, "Press q to stop");
    }

    #[test]
    fn footer_line_running_playback_mentions_esc() {
        let line = footer_line(&CommandPhase::Running, CommandPopupKind::Playback);
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(text, "Press q or Esc to stop");
    }

    #[test]
    fn footer_line_running_viewer_mentions_esc() {
        let line = footer_line(&CommandPhase::Running, CommandPopupKind::Viewer);
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(text, "Press q or Esc to stop");
    }

    #[test]
    fn footer_line_awaiting_shows_prompt() {
        let line = footer_line(
            &CommandPhase::AwaitingConfirmation {
                prompt: "Send recording? (y/n)".into(),
            },
            CommandPopupKind::Recording,
        );
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(text, "Send recording? (y/n)");
    }

    #[test]
    fn footer_line_uses_footer_style() {
        let line = footer_line(&CommandPhase::Running, CommandPopupKind::Recording);
        assert_eq!(line.spans[0].style, styles::command_popup_footer_style());
    }

    #[test]
    fn footer_line_stopping_shows_wait_hint() {
        let line = footer_line(&CommandPhase::Stopping, CommandPopupKind::Recording);
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(text, "Stopping...");
    }

    #[test]
    fn footer_line_stopping_uses_footer_style() {
        let line = footer_line(&CommandPhase::Stopping, CommandPopupKind::Recording);
        assert_eq!(line.spans[0].style, styles::command_popup_footer_style());
    }

    #[test]
    fn footer_line_failed_shows_error_message() {
        let line = footer_line(
            &CommandPhase::Failed {
                message: "Recording failed".into(),
            },
            CommandPopupKind::Recording,
        );
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(text, "Recording failed");
    }

    #[test]
    fn footer_line_failed_uses_error_style() {
        let line = footer_line(
            &CommandPhase::Failed {
                message: "err".into(),
            },
            CommandPopupKind::Recording,
        );
        assert_eq!(line.spans[0].style, styles::command_popup_error_style());
    }

    #[test]
    fn max_output_lines_formula_subtracts_chrome_and_footer() {
        // Popup height = 20 → 20 - 4 (chrome) - 2 (footer) = 14 lines for output
        let popup_height: u16 = 20;
        let max = popup_height.saturating_sub(CHROME_LINES + FOOTER_LINES) as usize;
        assert_eq!(max, 14);
    }

    #[test]
    fn max_output_lines_saturates_at_zero_for_tiny_popup() {
        // Popup height = 5 → 5 - 6 = 0 (saturating)
        let popup_height: u16 = 5;
        let max = popup_height.saturating_sub(CHROME_LINES + FOOTER_LINES) as usize;
        assert_eq!(max, 0);
    }

    #[test]
    fn chrome_and_footer_constants_are_correct() {
        // Borders::ALL = 2 lines (top + bottom)
        // Padding vertical = 2 lines (top + bottom)
        assert_eq!(CHROME_LINES, 4);
        // Empty separator line + footer text line
        assert_eq!(FOOTER_LINES, 2);
    }

    #[test]
    fn footer_line_done_shows_close_hint() {
        let line = footer_line(&CommandPhase::Done, CommandPopupKind::Viewer);
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(text, "Press any key to close");
    }

    #[test]
    fn footer_line_done_uses_footer_style() {
        let line = footer_line(&CommandPhase::Done, CommandPopupKind::Viewer);
        assert_eq!(line.spans[0].style, styles::command_popup_footer_style());
    }
}
