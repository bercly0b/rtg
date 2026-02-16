use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

use crate::usecases::context::AppContext;

use super::{state::AppState, terminal::TerminalSession, view};

const EVENT_POLL_TIMEOUT: Duration = Duration::from_millis(100);

pub fn start(context: &AppContext) -> Result<()> {
    tracing::info!(
        log_level = %context.config.logging.level,
        telegram_adapter = ?context.telegram,
        cache_adapter = ?context.cache,
        "starting TUI shell"
    );

    let mut terminal = TerminalSession::new()?;
    let mut state = AppState::default();

    while state.is_running() {
        terminal.draw(|frame| view::render(frame, &state))?;

        if !event::poll(EVENT_POLL_TIMEOUT)? {
            continue;
        }

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                handle_key_event(&mut state, key.code, key.modifiers);
            }
        }
    }

    Ok(())
}

fn handle_key_event(state: &mut AppState, code: KeyCode, modifiers: KeyModifiers) {
    if code == KeyCode::Char('q')
        || (code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL))
    {
        state.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stops_on_q() {
        let mut state = AppState::default();

        handle_key_event(&mut state, KeyCode::Char('q'), KeyModifiers::NONE);

        assert!(!state.is_running());
    }

    #[test]
    fn stops_on_ctrl_c() {
        let mut state = AppState::default();

        handle_key_event(&mut state, KeyCode::Char('c'), KeyModifiers::CONTROL);

        assert!(!state.is_running());
    }

    #[test]
    fn keeps_running_for_other_keys() {
        let mut state = AppState::default();

        handle_key_event(&mut state, KeyCode::Char('x'), KeyModifiers::NONE);

        assert!(state.is_running());
    }
}
