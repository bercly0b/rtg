use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

use crate::{
    domain::events::{AppEvent, KeyInput},
    usecases::contracts::AppEventSource,
};

const EVENT_POLL_TIMEOUT: Duration = Duration::from_millis(100);

#[derive(Default)]
pub struct CrosstermEventSource;

impl AppEventSource for CrosstermEventSource {
    fn next_event(&mut self) -> Result<Option<AppEvent>> {
        if !event::poll(EVENT_POLL_TIMEOUT)? {
            return Ok(Some(AppEvent::Tick));
        }

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                return Ok(None);
            }

            if key.code == KeyCode::Char('q')
                || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
            {
                return Ok(Some(AppEvent::QuitRequested));
            }

            if let KeyCode::Char(ch) = key.code {
                return Ok(Some(AppEvent::InputKey(KeyInput::new(
                    ch.to_string(),
                    key.modifiers.contains(KeyModifiers::CONTROL),
                ))));
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
pub struct MockEventSource {
    queue: std::collections::VecDeque<AppEvent>,
}

#[cfg(test)]
impl MockEventSource {
    pub fn from(events: Vec<AppEvent>) -> Self {
        Self {
            queue: events.into(),
        }
    }
}

#[cfg(test)]
impl AppEventSource for MockEventSource {
    fn next_event(&mut self) -> Result<Option<AppEvent>> {
        Ok(self.queue.pop_front())
    }
}
