//! State for the reusable command execution popup overlay.
//!
//! The popup displays real-time output from an external command (e.g. ffmpeg
//! recording, media player) and handles a lifecycle of phases:
//! Running → Stopping → AwaitingConfirmation → closed.

use std::collections::VecDeque;

use unicode_width::UnicodeWidthStr;

/// Maximum number of output lines kept in the buffer.
/// Older lines are discarded when this limit is exceeded.
const MAX_OUTPUT_LINES: usize = 200;

/// Fallback limit for visible lines when no height hint is provided.
const FALLBACK_VISIBLE_LINES: usize = 20;

/// Distinguishes the purpose of the command popup so the orchestrator
/// can apply the correct lifecycle (e.g. recording asks "send?", playback auto-closes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandPopupKind {
    /// Voice recording — on exit asks "Send recording? (y/n)".
    Recording,
    /// Media playback — auto-closes on process exit, `q` stops early.
    Playback,
    /// Static content viewer (e.g. terminal image via chafa) — stays open
    /// after the process exits so the user can see the output. Any key closes.
    #[allow(dead_code, reason = "constructed only by tests so far")]
    Viewer,
}

/// Phase of the command execution lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandPhase {
    /// The external command is currently running.
    Running,
    /// The process is being terminated (non-blocking); waiting for exit.
    Stopping,
    /// The command has finished; the user must confirm an action (e.g. send or discard).
    AwaitingConfirmation { prompt: String },
    /// The command completed and the popup stays visible for review.
    /// Any key dismisses the popup (used by `Viewer`).
    Done,
    /// The command failed; displays an error message and closes on any key.
    Failed { message: String },
}

/// State for the command popup overlay.
///
/// Generic and reusable: not tied to any specific command or use case.
/// Tracks the command's output lines and current lifecycle phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPopupState {
    title: String,
    output_lines: VecDeque<String>,
    /// Display width of the widest line ever shown, including lines since
    /// replaced or evicted, so the popup does not narrow while it is open.
    widest_line_width: usize,
    phase: CommandPhase,
    kind: CommandPopupKind,
}

impl CommandPopupState {
    /// Creates a new popup state in the `Running` phase.
    pub fn new(title: impl Into<String>, kind: CommandPopupKind) -> Self {
        Self {
            title: title.into(),
            output_lines: VecDeque::new(),
            widest_line_width: 0,
            phase: CommandPhase::Running,
            kind,
        }
    }

    pub fn kind(&self) -> CommandPopupKind {
        self.kind
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn phase(&self) -> &CommandPhase {
        &self.phase
    }

    pub fn set_phase(&mut self, phase: CommandPhase) {
        self.phase = phase;
    }

    /// Appends a line of command output, discarding the oldest if the buffer is full.
    pub fn push_line(&mut self, line: String) {
        if self.output_lines.len() >= MAX_OUTPUT_LINES {
            self.output_lines.pop_front();
        }
        self.widest_line_width = self.widest_line_width.max(line.width());
        self.output_lines.push_back(line);
    }

    /// Replaces the last visible output line, or appends when buffer is empty.
    pub fn replace_last_line(&mut self, line: String) {
        if let Some(last) = self.output_lines.back_mut() {
            self.widest_line_width = self.widest_line_width.max(line.width());
            *last = line;
            return;
        }

        self.push_line(line);
    }

    pub fn line_count(&self) -> usize {
        self.output_lines.len()
    }

    pub fn widest_line_width(&self) -> usize {
        self.widest_line_width
    }

    /// Returns the last N lines that should be visible in the popup viewport.
    ///
    /// `max_lines` limits how many output lines to show. The caller should
    /// compute this from the available popup height minus the footer.
    pub fn visible_lines(&self, max_lines: usize) -> Vec<&str> {
        let limit = if max_lines == 0 {
            FALLBACK_VISIBLE_LINES
        } else {
            max_lines
        };
        let total = self.output_lines.len();
        let skip = total.saturating_sub(limit);
        self.output_lines
            .iter()
            .skip(skip)
            .map(|s| s.as_str())
            .collect()
    }

    /// Returns all buffered output lines.
    #[cfg(test)]
    pub fn all_lines(&self) -> Vec<&str> {
        self.output_lines.iter().map(|s| s.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_popup(title: &str) -> CommandPopupState {
        CommandPopupState::new(title, CommandPopupKind::Recording)
    }

    #[test]
    fn new_popup_starts_in_running_phase() {
        let state = test_popup("Recording");
        assert_eq!(state.phase(), &CommandPhase::Running);
        assert_eq!(state.title(), "Recording");
        assert!(state.visible_lines(20).is_empty());
    }

    #[test]
    fn push_line_adds_output() {
        let mut state = test_popup("Test");
        state.push_line("line 1".into());
        state.push_line("line 2".into());
        assert_eq!(state.visible_lines(20), vec!["line 1", "line 2"]);
    }

    #[test]
    fn replace_last_line_updates_tail() {
        let mut state = test_popup("Test");
        state.push_line("line 1".into());
        state.push_line("line 2".into());
        state.replace_last_line("line 2 updated".into());
        assert_eq!(state.visible_lines(20), vec!["line 1", "line 2 updated"]);
    }

    #[test]
    fn replace_last_line_appends_when_empty() {
        let mut state = test_popup("Test");
        state.replace_last_line("line 1".into());
        assert_eq!(state.visible_lines(20), vec!["line 1"]);
    }

    #[test]
    fn visible_lines_returns_last_n_when_exceeding_max() {
        let mut state = test_popup("Test");
        for i in 0..30 {
            state.push_line(format!("line {i}"));
        }
        let visible = state.visible_lines(20);
        assert_eq!(visible.len(), 20);
        assert_eq!(visible[0], "line 10");
        assert_eq!(visible[19], "line 29");
    }

    #[test]
    fn visible_lines_respects_dynamic_limit() {
        let mut state = test_popup("Test");
        for i in 0..30 {
            state.push_line(format!("line {i}"));
        }
        let visible = state.visible_lines(10);
        assert_eq!(visible.len(), 10);
        assert_eq!(visible[0], "line 20");
        assert_eq!(visible[9], "line 29");
    }

    #[test]
    fn visible_lines_zero_uses_fallback() {
        let mut state = test_popup("Test");
        for i in 0..30 {
            state.push_line(format!("line {i}"));
        }
        let visible = state.visible_lines(0);
        assert_eq!(visible.len(), FALLBACK_VISIBLE_LINES);
    }

    #[test]
    fn visible_lines_returns_all_when_under_max() {
        let mut state = test_popup("Test");
        for i in 0..5 {
            state.push_line(format!("line {i}"));
        }
        assert_eq!(state.visible_lines(20).len(), 5);
    }

    #[test]
    fn push_line_evicts_oldest_when_buffer_full() {
        let mut state = test_popup("Test");
        for i in 0..MAX_OUTPUT_LINES + 10 {
            state.push_line(format!("line {i}"));
        }
        assert_eq!(state.all_lines().len(), MAX_OUTPUT_LINES);
        assert_eq!(state.all_lines()[0], "line 10");
    }

    #[test]
    fn set_phase_transitions_state() {
        let mut state = test_popup("Test");
        assert_eq!(state.phase(), &CommandPhase::Running);

        state.set_phase(CommandPhase::AwaitingConfirmation {
            prompt: "Send? (y/n)".into(),
        });
        assert_eq!(
            state.phase(),
            &CommandPhase::AwaitingConfirmation {
                prompt: "Send? (y/n)".into(),
            }
        );
    }

    #[test]
    fn empty_title_is_allowed() {
        let state = CommandPopupState::new("", CommandPopupKind::Playback);
        assert_eq!(state.title(), "");
    }

    #[test]
    fn push_empty_line_is_tracked() {
        let mut state = test_popup("Test");
        state.push_line(String::new());
        assert_eq!(state.visible_lines(20), vec![""]);
    }

    #[test]
    fn set_phase_to_failed() {
        let mut state = test_popup("Test");
        state.set_phase(CommandPhase::Failed {
            message: "Recording failed".into(),
        });
        assert_eq!(
            state.phase(),
            &CommandPhase::Failed {
                message: "Recording failed".into(),
            }
        );
    }

    #[test]
    fn set_phase_to_stopping() {
        let mut state = test_popup("Test");
        state.set_phase(CommandPhase::Stopping);
        assert_eq!(state.phase(), &CommandPhase::Stopping);
    }

    #[test]
    fn new_popup_stores_kind() {
        let rec = CommandPopupState::new("Rec", CommandPopupKind::Recording);
        assert_eq!(rec.kind(), CommandPopupKind::Recording);

        let play = CommandPopupState::new("Play", CommandPopupKind::Playback);
        assert_eq!(play.kind(), CommandPopupKind::Playback);

        let viewer = CommandPopupState::new("View", CommandPopupKind::Viewer);
        assert_eq!(viewer.kind(), CommandPopupKind::Viewer);
    }

    #[test]
    fn line_count_counts_buffered_lines() {
        let mut state = test_popup("Test");
        assert_eq!(state.line_count(), 0);
        state.push_line("line 1".into());
        state.push_line("line 2".into());
        state.replace_last_line("line 2 updated".into());
        assert_eq!(state.line_count(), 2);
    }

    #[test]
    fn widest_line_width_is_zero_without_output() {
        assert_eq!(test_popup("Test").widest_line_width(), 0);
    }

    #[test]
    fn widest_line_width_follows_widest_pushed_line() {
        let mut state = test_popup("Test");
        state.push_line("abc".into());
        state.push_line("abcdef".into());
        state.push_line("ab".into());
        assert_eq!(state.widest_line_width(), 6);
    }

    #[test]
    fn widest_line_width_measures_display_width() {
        let mut state = test_popup("Test");
        state.push_line("Привет 👍".into());
        assert_eq!(state.widest_line_width(), 9);
    }

    #[test]
    fn widest_line_width_grows_on_wider_replacement() {
        let mut state = test_popup("Test");
        state.push_line("size=1kB".into());
        state.replace_last_line("size=1024kB".into());
        assert_eq!(state.widest_line_width(), 11);
    }

    #[test]
    fn widest_line_width_keeps_replaced_line() {
        let mut state = test_popup("Test");
        state.push_line("size=1024kB".into());
        state.replace_last_line("done".into());
        assert_eq!(state.widest_line_width(), 11);
    }

    #[test]
    fn widest_line_width_keeps_evicted_line() {
        let mut state = test_popup("Test");
        state.push_line("x".repeat(50));
        for _ in 0..MAX_OUTPUT_LINES {
            state.push_line("short".into());
        }
        assert!(!state.all_lines().contains(&"x".repeat(50).as_str()));
        assert_eq!(state.widest_line_width(), 50);
    }

    #[test]
    fn set_phase_to_done() {
        let mut state = test_popup("Test");
        state.set_phase(CommandPhase::Done);
        assert_eq!(state.phase(), &CommandPhase::Done);
    }
}
