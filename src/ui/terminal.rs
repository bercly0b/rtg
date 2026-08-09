use std::io::{self, Stdout, Write};

use anyhow::Result;
use crossterm::{
    cursor::Show,
    event::{KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, supports_keyboard_enhancement, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use ratatui::{backend::CrosstermBackend, Frame, Terminal};

use crate::infra::secrets::set_panic_stderr_suppressed;

pub struct TerminalSession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    keyboard_enhanced: bool,
}

#[derive(Default)]
struct InitState {
    raw_mode_enabled: bool,
    alternate_screen_entered: bool,
    keyboard_enhanced: bool,
}

impl InitState {
    fn rollback<W: Write>(&self, stdout: &mut W) {
        if self.keyboard_enhanced {
            let _ = execute!(stdout, PopKeyboardEnhancementFlags);
        }

        if self.alternate_screen_entered {
            set_panic_stderr_suppressed(false);
            let _ = execute!(stdout, LeaveAlternateScreen, Show);
        }

        if self.raw_mode_enabled {
            let _ = disable_raw_mode();
        }
    }
}

impl TerminalSession {
    pub fn new() -> Result<Self> {
        let mut init_state = InitState::default();

        enable_raw_mode()?;
        init_state.raw_mode_enabled = true;

        let mut stdout = io::stdout();
        if let Err(err) = execute!(stdout, EnterAlternateScreen) {
            init_state.rollback(&mut stdout);
            return Err(err.into());
        }
        init_state.alternate_screen_entered = true;
        set_panic_stderr_suppressed(true);

        // Shift+Enter is indistinguishable from Enter unless the terminal
        // disambiguates modified keys via the kitty keyboard protocol.
        if matches!(supports_keyboard_enhancement(), Ok(true))
            && execute!(
                stdout,
                PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
            )
            .is_ok()
        {
            init_state.keyboard_enhanced = true;
        }

        let backend = CrosstermBackend::new(stdout);
        let terminal = match Terminal::new(backend) {
            Ok(terminal) => terminal,
            Err(err) => {
                let mut stdout = io::stdout();
                init_state.rollback(&mut stdout);
                return Err(err.into());
            }
        };

        Ok(Self {
            terminal,
            keyboard_enhanced: init_state.keyboard_enhanced,
        })
    }

    pub fn draw<F>(&mut self, render: F) -> Result<()>
    where
        F: FnOnce(&mut Frame<'_>),
    {
        self.terminal.draw(render)?;
        Ok(())
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        set_panic_stderr_suppressed(false);
        if self.keyboard_enhanced {
            let _ = execute!(self.terminal.backend_mut(), PopKeyboardEnhancementFlags);
        }
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

#[cfg(test)]
mod tests {
    use super::InitState;

    #[test]
    fn rollback_writes_leave_alt_and_show_when_alt_screen_was_entered() {
        let mut output = Vec::new();
        let state = InitState {
            raw_mode_enabled: true,
            alternate_screen_entered: true,
            keyboard_enhanced: false,
        };

        state.rollback(&mut output);

        assert!(!output.is_empty());
    }

    #[test]
    fn rollback_pops_keyboard_enhancement_when_pushed() {
        let mut output = Vec::new();
        let state = InitState {
            raw_mode_enabled: true,
            alternate_screen_entered: false,
            keyboard_enhanced: true,
        };

        state.rollback(&mut output);

        assert!(!output.is_empty());
    }

    #[test]
    fn rollback_skips_alt_screen_commands_when_not_entered() {
        let mut output = Vec::new();
        let state = InitState {
            raw_mode_enabled: true,
            alternate_screen_entered: false,
            keyboard_enhanced: false,
        };

        state.rollback(&mut output);

        assert!(output.is_empty());
    }
}
