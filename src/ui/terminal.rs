use std::io::{self, Stdout, Write};

use anyhow::Result;
use crossterm::{
    cursor::Show,
    event::{KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
    execute,
    style::Print,
    terminal::{
        disable_raw_mode, enable_raw_mode, supports_keyboard_enhancement, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use ratatui::{backend::CrosstermBackend, Frame, Terminal};

use crate::infra::secrets::set_panic_stderr_suppressed;

/// xterm `modifyOtherKeys=2` and its reset. Used where the kitty keyboard
/// protocol is unavailable — notably inside tmux, which reports modified keys
/// only when the application asks for them.
const MODIFY_OTHER_KEYS_ON: &str = "\x1b[>4;2m";
const MODIFY_OTHER_KEYS_OFF: &str = "\x1b[>4m";

/// How the terminal was asked to report modified keys such as `Shift+Enter`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum ExtendedKeys {
    /// No request was accepted; modified keys keep their legacy encoding.
    #[default]
    Legacy,
    Kitty,
    ModifyOtherKeys,
}

impl ExtendedKeys {
    fn enable<W: Write>(stdout: &mut W) -> Self {
        if matches!(supports_keyboard_enhancement(), Ok(true))
            && execute!(
                stdout,
                PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
            )
            .is_ok()
        {
            return Self::Kitty;
        }

        if execute!(stdout, Print(MODIFY_OTHER_KEYS_ON)).is_ok() {
            return Self::ModifyOtherKeys;
        }

        Self::Legacy
    }

    fn disable<W: Write>(self, stdout: &mut W) {
        match self {
            Self::Legacy => {}
            Self::Kitty => {
                let _ = execute!(stdout, PopKeyboardEnhancementFlags);
            }
            Self::ModifyOtherKeys => {
                let _ = execute!(stdout, Print(MODIFY_OTHER_KEYS_OFF));
            }
        }
    }
}

pub struct TerminalSession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    extended_keys: ExtendedKeys,
}

#[derive(Default)]
struct InitState {
    raw_mode_enabled: bool,
    alternate_screen_entered: bool,
    extended_keys: ExtendedKeys,
}

impl InitState {
    fn rollback<W: Write>(&self, stdout: &mut W) {
        self.extended_keys.disable(stdout);

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

        // Shift+Enter is indistinguishable from Enter until the terminal is
        // asked to report modified keys.
        init_state.extended_keys = ExtendedKeys::enable(&mut stdout);

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
            extended_keys: init_state.extended_keys,
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
        self.extended_keys.disable(self.terminal.backend_mut());
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

#[cfg(test)]
mod tests {
    use super::{ExtendedKeys, InitState, MODIFY_OTHER_KEYS_OFF};

    #[test]
    fn rollback_writes_leave_alt_and_show_when_alt_screen_was_entered() {
        let mut output = Vec::new();
        let state = InitState {
            raw_mode_enabled: true,
            alternate_screen_entered: true,
            extended_keys: ExtendedKeys::Legacy,
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
            extended_keys: ExtendedKeys::Legacy,
        };

        state.rollback(&mut output);

        assert!(output.is_empty());
    }

    #[test]
    fn disable_resets_modify_other_keys() {
        let mut output = Vec::new();

        ExtendedKeys::ModifyOtherKeys.disable(&mut output);

        assert_eq!(String::from_utf8(output).unwrap(), MODIFY_OTHER_KEYS_OFF);
    }

    #[test]
    fn disable_pops_kitty_enhancement_flags() {
        let mut output = Vec::new();

        ExtendedKeys::Kitty.disable(&mut output);

        assert!(!output.is_empty());
    }

    #[test]
    fn disable_writes_nothing_for_legacy_terminals() {
        let mut output = Vec::new();

        ExtendedKeys::Legacy.disable(&mut output);

        assert!(output.is_empty());
    }
}
