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
/// Columns consumed by borders (left + right) and padding (left + right).
const CHROME_COLUMNS: u16 = 4;
/// The popup grows with its output up to this share of the screen.
const MAX_SIZE_PERCENT: u16 = 60;

/// Renders the command popup as an overlay on top of existing content.
pub fn render_command_popup(frame: &mut Frame<'_>, area: Rect, state: &CommandPopupState) {
    let footer = footer_line(state.phase(), state.kind());
    let popup_area = popup_rect(area, state, &footer);

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
    lines.push(footer);

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, popup_area);
}

/// Sizes the popup to its output, capped at `MAX_SIZE_PERCENT` of the area.
///
/// While the popup is open the buffered line count never decreases and the
/// state remembers the widest line ever shown, so streaming output only
/// grows the popup instead of making it jump back and forth. Lines wider
/// than the cap are cut off rather than wrapped, keeping one row per line.
fn popup_rect(area: Rect, state: &CommandPopupState, footer: &Line<'_>) -> Rect {
    let max = popup_utils::centered_rect(area, MAX_SIZE_PERCENT, MAX_SIZE_PERCENT);

    let content_width = state.widest_line_width().max(footer.width());
    let width = popup_utils::saturating_u16(content_width)
        .saturating_add(CHROME_COLUMNS)
        .max(popup_utils::MIN_CONTENT_WIDTH)
        .min(max.width);
    let height = popup_utils::saturating_u16(state.line_count())
        .saturating_add(CHROME_LINES + FOOTER_LINES)
        .min(max.height);

    popup_utils::centered(area, width, height)
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

    fn recording_with_lines(lines: impl IntoIterator<Item = String>) -> CommandPopupState {
        let mut state = CommandPopupState::new("Recording Voice", CommandPopupKind::Recording);
        for line in lines {
            state.push_line(line);
        }
        state
    }

    fn render(state: &CommandPopupState) -> ratatui::buffer::Buffer {
        use ratatui::{backend::TestBackend, Terminal};

        let mut terminal = Terminal::new(TestBackend::new(100, 40)).unwrap();
        terminal
            .draw(|frame| render_command_popup(frame, frame.area(), state))
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn rendered_popup_rect(state: &CommandPopupState) -> Rect {
        popup_utils::rendered_border_rect(&render(state)).unwrap()
    }

    fn rendered_rows(buffer: &ratatui::buffer::Buffer) -> Vec<String> {
        let area = buffer.area;
        (area.top()..area.bottom())
            .map(|y| {
                (area.left()..area.right())
                    .map(|x| buffer[(x, y)].symbol())
                    .collect()
            })
            .collect()
    }

    #[test]
    fn popup_without_output_fits_footer_at_minimum_width() {
        let state = CommandPopupState::new("Playing", CommandPopupKind::Playback);
        assert_eq!(rendered_popup_rect(&state), Rect::new(35, 17, 30, 6));
    }

    #[test]
    fn popup_height_follows_output_lines() {
        let state = recording_with_lines((0..3).map(|i| format!("line {i}")));
        let rect = rendered_popup_rect(&state);
        assert_eq!(rect.height, 3 + CHROME_LINES + FOOTER_LINES);
        assert_eq!(rect.y, (40 - rect.height) / 2);
    }

    #[test]
    fn popup_width_follows_widest_line() {
        let state = recording_with_lines(["short".to_owned(), "x".repeat(50)]);
        let rect = rendered_popup_rect(&state);
        assert_eq!(rect.width, 50 + CHROME_COLUMNS);
        assert_eq!(rect.x, (100 - rect.width) / 2);
    }

    #[test]
    fn popup_does_not_narrow_when_widest_line_is_replaced() {
        let mut state = recording_with_lines(["x".repeat(50)]);
        state.replace_last_line("short".into());
        assert_eq!(rendered_popup_rect(&state).width, 50 + CHROME_COLUMNS);
    }

    #[test]
    fn replacing_last_line_keeps_popup_size() {
        let mut state = recording_with_lines(["banner".to_owned(), "size=1kB".to_owned()]);
        let before = rendered_popup_rect(&state);
        state.replace_last_line("size=2kB".into());
        assert_eq!(rendered_popup_rect(&state), before);
    }

    #[test]
    fn popup_caps_width_at_sixty_percent_and_cuts_long_lines() {
        let state = recording_with_lines(["x".repeat(200)]);
        let rect = rendered_popup_rect(&state);
        assert_eq!(rect.width, 60);
        assert_eq!(rect.height, 1 + CHROME_LINES + FOOTER_LINES);
    }

    #[test]
    fn popup_caps_height_at_sixty_percent_and_shows_latest_lines() {
        let state = recording_with_lines((0..50).map(|i| format!("line {i:02}")));
        let buffer = render(&state);
        let rect = popup_utils::rendered_border_rect(&buffer).unwrap();
        assert_eq!(rect.height, 24);

        let rows = rendered_rows(&buffer).join("\n");
        let visible = usize::from(24 - CHROME_LINES - FOOTER_LINES);
        assert!(rows.contains(&format!("line {:02}", 50 - visible)));
        assert!(!rows.contains(&format!("line {:02}", 50 - visible - 1)));
        assert!(rows.contains("line 49"));
        assert!(rows.contains("Press q to stop"));
    }

    #[test]
    fn popup_grows_to_fit_wider_footer() {
        let mut state = recording_with_lines(["short".to_owned()]);
        state.set_phase(CommandPhase::Failed {
            message: "e".repeat(40),
        });
        assert_eq!(rendered_popup_rect(&state).width, 40 + CHROME_COLUMNS);
    }
}
