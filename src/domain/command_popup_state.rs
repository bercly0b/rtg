//! State for the reusable command execution popup overlay.
//!
//! The popup displays real-time output from an external command (e.g. ffmpeg
//! recording, media player) and handles a lifecycle of phases:
//! Running → Stopping → AwaitingConfirmation → closed.

use std::collections::VecDeque;

/// Maximum number of output lines kept in the buffer.
/// Older lines are discarded when this limit is exceeded.
const MAX_OUTPUT_LINES: usize = 200;

/// Fallback limit for visible lines when no height hint is provided.
const FALLBACK_VISIBLE_LINES: usize = 20;

/// Phase of the command execution lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandPhase {
    /// The external command is currently running.
    Running,
    /// The process is being terminated (non-blocking); waiting for exit.
    Stopping,
    /// The command has finished; the user must confirm an action (e.g. send or discard).
    AwaitingConfirmation { prompt: String },
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
    phase: CommandPhase,
}

impl CommandPopupState {
    /// Creates a new popup state in the `Running` phase.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            output_lines: VecDeque::new(),
            phase: CommandPhase::Running,
        }
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
        self.output_lines.push_back(line);
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

    #[test]
    fn new_popup_starts_in_running_phase() {
        let state = CommandPopupState::new("Recording");
        assert_eq!(state.phase(), &CommandPhase::Running);
        assert_eq!(state.title(), "Recording");
        assert!(state.visible_lines(20).is_empty());
    }

    #[test]
    fn push_line_adds_output() {
        let mut state = CommandPopupState::new("Test");
        state.push_line("line 1".into());
        state.push_line("line 2".into());
        assert_eq!(state.visible_lines(20), vec!["line 1", "line 2"]);
    }

    #[test]
    fn visible_lines_returns_last_n_when_exceeding_max() {
        let mut state = CommandPopupState::new("Test");
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
        let mut state = CommandPopupState::new("Test");
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
        let mut state = CommandPopupState::new("Test");
        for i in 0..30 {
            state.push_line(format!("line {i}"));
        }
        let visible = state.visible_lines(0);
        assert_eq!(visible.len(), FALLBACK_VISIBLE_LINES);
    }

    #[test]
    fn visible_lines_returns_all_when_under_max() {
        let mut state = CommandPopupState::new("Test");
        for i in 0..5 {
            state.push_line(format!("line {i}"));
        }
        assert_eq!(state.visible_lines(20).len(), 5);
    }

    #[test]
    fn push_line_evicts_oldest_when_buffer_full() {
        let mut state = CommandPopupState::new("Test");
        for i in 0..MAX_OUTPUT_LINES + 10 {
            state.push_line(format!("line {i}"));
        }
        assert_eq!(state.all_lines().len(), MAX_OUTPUT_LINES);
        assert_eq!(state.all_lines()[0], "line 10");
    }

    #[test]
    fn set_phase_transitions_state() {
        let mut state = CommandPopupState::new("Test");
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
        let state = CommandPopupState::new("");
        assert_eq!(state.title(), "");
    }

    #[test]
    fn push_empty_line_is_tracked() {
        let mut state = CommandPopupState::new("Test");
        state.push_line(String::new());
        assert_eq!(state.visible_lines(20), vec![""]);
    }

    #[test]
    fn set_phase_to_failed() {
        let mut state = CommandPopupState::new("Test");
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
        let mut state = CommandPopupState::new("Test");
        state.set_phase(CommandPhase::Stopping);
        assert_eq!(state.phase(), &CommandPhase::Stopping);
    }
}
