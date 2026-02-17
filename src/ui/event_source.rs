use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

use crate::{
    domain::events::{AppEvent, ConnectivityStatus, KeyInput},
    usecases::contracts::AppEventSource,
};

const EVENT_POLL_TIMEOUT: Duration = Duration::from_millis(100);

pub trait ConnectivityStatusSource {
    fn next_status(&mut self) -> Option<ConnectivityStatus>;
}

#[derive(Default)]
pub struct StubConnectivityStatusSource;

impl ConnectivityStatusSource for StubConnectivityStatusSource {
    fn next_status(&mut self) -> Option<ConnectivityStatus> {
        None
    }
}

pub struct CrosstermEventSource {
    connectivity_source: Box<dyn ConnectivityStatusSource>,
}

impl Default for CrosstermEventSource {
    fn default() -> Self {
        Self {
            connectivity_source: Box::new(StubConnectivityStatusSource),
        }
    }
}

impl CrosstermEventSource {
    pub fn new(connectivity_source: Box<dyn ConnectivityStatusSource>) -> Self {
        Self {
            connectivity_source,
        }
    }
}

impl AppEventSource for CrosstermEventSource {
    fn next_event(&mut self) -> Result<Option<AppEvent>> {
        if let Some(status) = self.connectivity_source.next_status() {
            return Ok(Some(AppEvent::ConnectivityChanged(status)));
        }

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
    connectivity_queue: std::collections::VecDeque<ConnectivityStatus>,
}

#[cfg(test)]
impl MockEventSource {
    pub fn from(events: Vec<AppEvent>) -> Self {
        Self {
            queue: events.into(),
            connectivity_queue: Default::default(),
        }
    }

    pub fn with_connectivity(events: Vec<AppEvent>, connectivity: Vec<ConnectivityStatus>) -> Self {
        Self {
            queue: events.into(),
            connectivity_queue: connectivity.into(),
        }
    }
}

#[cfg(test)]
impl AppEventSource for MockEventSource {
    fn next_event(&mut self) -> Result<Option<AppEvent>> {
        if let Some(status) = self.connectivity_queue.pop_front() {
            return Ok(Some(AppEvent::ConnectivityChanged(status)));
        }

        Ok(self.queue.pop_front())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_event_source_returns_none_when_queue_is_exhausted() {
        let mut source = MockEventSource::from(vec![AppEvent::Tick]);

        assert_eq!(
            source.next_event().expect("first event must be read"),
            Some(AppEvent::Tick)
        );
        assert_eq!(source.next_event().expect("queue must be empty"), None);
    }

    #[test]
    fn mock_event_source_keeps_tick_input_path_when_no_connectivity_event_available() {
        let mut source = MockEventSource::with_connectivity(
            vec![AppEvent::Tick, AppEvent::InputKey(KeyInput::new("x", false))],
            vec![],
        );

        assert_eq!(source.next_event().expect("tick must be emitted"), Some(AppEvent::Tick));
        assert_eq!(
            source.next_event().expect("input must be emitted"),
            Some(AppEvent::InputKey(KeyInput::new("x", false)))
        );
    }
}
