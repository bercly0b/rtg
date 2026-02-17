use std::{collections::VecDeque, time::Duration};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::{
    domain::events::{AppEvent, ConnectivityStatus, KeyInput},
    usecases::contracts::AppEventSource,
};

const EVENT_POLL_TIMEOUT: Duration = Duration::from_millis(100);
const MAX_CONNECTIVITY_STREAK: u8 = 3;
const MAX_CONNECTIVITY_DRAIN_PER_CYCLE: usize = 32;

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

trait TerminalEventSource {
    fn poll(&mut self, timeout: Duration) -> Result<bool>;
    fn read(&mut self) -> Result<Event>;
}

struct CrosstermTerminalEventSource;

impl TerminalEventSource for CrosstermTerminalEventSource {
    fn poll(&mut self, timeout: Duration) -> Result<bool> {
        Ok(event::poll(timeout)?)
    }

    fn read(&mut self) -> Result<Event> {
        Ok(event::read()?)
    }
}

pub struct CrosstermEventSource {
    connectivity_source: Box<dyn ConnectivityStatusSource>,
    pending_connectivity: Option<ConnectivityStatus>,
    last_emitted_connectivity: Option<ConnectivityStatus>,
    connectivity_streak: u8,
}

impl Default for CrosstermEventSource {
    fn default() -> Self {
        Self {
            connectivity_source: Box::new(StubConnectivityStatusSource),
            pending_connectivity: None,
            last_emitted_connectivity: None,
            connectivity_streak: 0,
        }
    }
}

impl CrosstermEventSource {
    pub fn new(connectivity_source: Box<dyn ConnectivityStatusSource>) -> Self {
        Self {
            connectivity_source,
            pending_connectivity: None,
            last_emitted_connectivity: None,
            connectivity_streak: 0,
        }
    }

    fn next_event_with_terminal<T: TerminalEventSource>(
        &mut self,
        terminal: &mut T,
    ) -> Result<Option<AppEvent>> {
        self.refresh_pending_connectivity();

        if self.connectivity_streak < MAX_CONNECTIVITY_STREAK {
            if let Some(status) = self.pending_connectivity.take() {
                self.connectivity_streak += 1;
                self.last_emitted_connectivity = Some(status);
                return Ok(Some(AppEvent::ConnectivityChanged(status)));
            }
        }

        self.connectivity_streak = 0;

        if !terminal.poll(EVENT_POLL_TIMEOUT)? {
            return Ok(Some(AppEvent::Tick));
        }

        if let Event::Key(key) = terminal.read()? {
            return Ok(map_key_event(key));
        }

        Ok(None)
    }

    fn refresh_pending_connectivity(&mut self) {
        for _ in 0..MAX_CONNECTIVITY_DRAIN_PER_CYCLE {
            let Some(status) = self.connectivity_source.next_status() else {
                break;
            };

            self.pending_connectivity = Some(status);
        }

        if self.pending_connectivity == self.last_emitted_connectivity {
            self.pending_connectivity = None;
        }
    }
}

impl AppEventSource for CrosstermEventSource {
    fn next_event(&mut self) -> Result<Option<AppEvent>> {
        let mut terminal = CrosstermTerminalEventSource;
        self.next_event_with_terminal(&mut terminal)
    }
}

fn map_key_event(key: KeyEvent) -> Option<AppEvent> {
    if key.kind != KeyEventKind::Press {
        return None;
    }

    if key.code == KeyCode::Char('q')
        || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
    {
        return Some(AppEvent::QuitRequested);
    }

    if let KeyCode::Char(ch) = key.code {
        return Some(AppEvent::InputKey(KeyInput::new(
            ch.to_string(),
            key.modifiers.contains(KeyModifiers::CONTROL),
        )));
    }

    None
}

#[cfg(test)]
pub struct MockEventSource {
    queue: VecDeque<AppEvent>,
    connectivity_queue: VecDeque<ConnectivityStatus>,
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

    struct TestConnectivitySource {
        statuses: VecDeque<ConnectivityStatus>,
    }

    impl TestConnectivitySource {
        fn from(statuses: Vec<ConnectivityStatus>) -> Self {
            Self {
                statuses: statuses.into(),
            }
        }
    }

    impl ConnectivityStatusSource for TestConnectivitySource {
        fn next_status(&mut self) -> Option<ConnectivityStatus> {
            self.statuses.pop_front()
        }
    }

    #[derive(Default)]
    struct TestTerminalEventSource {
        polled: VecDeque<bool>,
        events: VecDeque<Event>,
    }

    impl TestTerminalEventSource {
        fn with_polls(polls: Vec<bool>) -> Self {
            Self {
                polled: polls.into(),
                events: VecDeque::new(),
            }
        }

        fn with_polls_and_events(polls: Vec<bool>, events: Vec<Event>) -> Self {
            Self {
                polled: polls.into(),
                events: events.into(),
            }
        }
    }

    impl TerminalEventSource for TestTerminalEventSource {
        fn poll(&mut self, _timeout: Duration) -> Result<bool> {
            Ok(self.polled.pop_front().unwrap_or(false))
        }

        fn read(&mut self) -> Result<Event> {
            Ok(self
                .events
                .pop_front()
                .expect("read is called only after poll=true in tests"))
        }
    }

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

    #[test]
    fn crossterm_event_source_keeps_tick_progress_with_frequent_connectivity_events() {
        let mut source = CrosstermEventSource::new(Box::new(TestConnectivitySource::from(vec![
            ConnectivityStatus::Connected,
            ConnectivityStatus::Disconnected,
            ConnectivityStatus::Connecting,
            ConnectivityStatus::Connected,
            ConnectivityStatus::Disconnected,
            ConnectivityStatus::Connecting,
            ConnectivityStatus::Connected,
            ConnectivityStatus::Disconnected,
            ConnectivityStatus::Connecting,
        ])));
        let mut terminal = TestTerminalEventSource::with_polls(vec![false, false, false]);

        let mut produced = Vec::new();
        for _ in 0..6 {
            produced.push(
                source
                    .next_event_with_terminal(&mut terminal)
                    .expect("event should be readable")
                    .expect("test sequence should produce events"),
            );
        }

        assert!(
            produced.iter().any(|event| matches!(event, AppEvent::Tick)),
            "tick should still be emitted under connectivity burst"
        );
    }

    #[test]
    fn crossterm_event_source_keeps_input_and_quit_path_under_connectivity_burst() {
        let mut source = CrosstermEventSource::new(Box::new(TestConnectivitySource::from(vec![
            ConnectivityStatus::Connected,
            ConnectivityStatus::Disconnected,
            ConnectivityStatus::Connecting,
            ConnectivityStatus::Connected,
            ConnectivityStatus::Disconnected,
            ConnectivityStatus::Connecting,
        ])));

        let mut terminal = TestTerminalEventSource::with_polls_and_events(
            vec![true, true],
            vec![
                Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)),
                Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)),
            ],
        );

        let first_non_connectivity = source
            .next_event_with_terminal(&mut terminal)
            .expect("first event should be readable");
        let second_non_connectivity = source
            .next_event_with_terminal(&mut terminal)
            .expect("second event should be readable");
        let third_non_connectivity = source
            .next_event_with_terminal(&mut terminal)
            .expect("third event should be readable");
        let fourth_non_connectivity = source
            .next_event_with_terminal(&mut terminal)
            .expect("fourth event should be readable");

        assert!(matches!(first_non_connectivity, Some(AppEvent::ConnectivityChanged(_))));
        assert!(matches!(second_non_connectivity, Some(AppEvent::ConnectivityChanged(_))));
        assert!(matches!(third_non_connectivity, Some(AppEvent::ConnectivityChanged(_))));
        assert_eq!(
            fourth_non_connectivity,
            Some(AppEvent::InputKey(KeyInput::new("x", false)))
        );

        let fifth = source
            .next_event_with_terminal(&mut terminal)
            .expect("fifth event should be readable");
        let sixth = source
            .next_event_with_terminal(&mut terminal)
            .expect("sixth event should be readable");
        let seventh = source
            .next_event_with_terminal(&mut terminal)
            .expect("seventh event should be readable");
        let eighth = source
            .next_event_with_terminal(&mut terminal)
            .expect("eighth event should be readable");

        assert!(matches!(fifth, Some(AppEvent::ConnectivityChanged(_))));
        assert!(matches!(sixth, Some(AppEvent::ConnectivityChanged(_))));
        assert!(matches!(seventh, Some(AppEvent::ConnectivityChanged(_))));
        assert_eq!(eighth, Some(AppEvent::QuitRequested));
    }

    #[test]
    fn crossterm_event_source_deduplicates_same_connectivity_status() {
        let mut source = CrosstermEventSource::new(Box::new(TestConnectivitySource::from(vec![
            ConnectivityStatus::Connected,
            ConnectivityStatus::Connected,
            ConnectivityStatus::Connected,
        ])));
        let mut terminal = TestTerminalEventSource::with_polls(vec![false, false, false]);

        assert_eq!(
            source
                .next_event_with_terminal(&mut terminal)
                .expect("first event should be readable"),
            Some(AppEvent::ConnectivityChanged(ConnectivityStatus::Connected))
        );
        assert_eq!(
            source
                .next_event_with_terminal(&mut terminal)
                .expect("second event should be readable"),
            Some(AppEvent::Tick)
        );
    }
}
