use std::{sync::mpsc::Receiver, time::Duration};

#[cfg(test)]
use std::{collections::VecDeque, sync::mpsc};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::{
    domain::events::{AppEvent, ConnectivityStatus, KeyInput},
    usecases::contracts::AppEventSource,
};

const EVENT_POLL_TIMEOUT: Duration = Duration::from_millis(100);
const NON_BLOCKING_POLL_TIMEOUT: Duration = Duration::from_millis(0);
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

pub struct ChannelConnectivityStatusSource {
    receiver: Receiver<ConnectivityStatus>,
    latest: Option<ConnectivityStatus>,
}

impl ChannelConnectivityStatusSource {
    pub fn new(receiver: Receiver<ConnectivityStatus>) -> Self {
        Self {
            receiver,
            latest: None,
        }
    }

    #[cfg(test)]
    pub fn from_values(statuses: Vec<ConnectivityStatus>) -> Self {
        let (tx, rx) = mpsc::channel();
        for status in statuses {
            tx.send(status)
                .expect("status should be sent into test channel");
        }

        Self::new(rx)
    }
}

impl ConnectivityStatusSource for ChannelConnectivityStatusSource {
    fn next_status(&mut self) -> Option<ConnectivityStatus> {
        while let Ok(status) = self.receiver.try_recv() {
            self.latest = Some(status);
        }

        self.latest.take()
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

        let has_ready_terminal_input = terminal.poll(NON_BLOCKING_POLL_TIMEOUT).unwrap_or(false);
        if has_ready_terminal_input {
            self.connectivity_streak = 0;
            if let Event::Key(key) = terminal.read()? {
                return Ok(map_key_event(key));
            }
            return Ok(None);
        }

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
    struct ScriptedDrainConnectivitySource {
        bursts: VecDeque<VecDeque<ConnectivityStatus>>,
        active_drain: VecDeque<ConnectivityStatus>,
        drain_boundary_pending: bool,
    }

    impl ScriptedDrainConnectivitySource {
        fn from_bursts(bursts: Vec<Vec<ConnectivityStatus>>) -> Self {
            Self {
                bursts: bursts.into_iter().map(Into::into).collect(),
                active_drain: VecDeque::new(),
                drain_boundary_pending: false,
            }
        }
    }

    impl ConnectivityStatusSource for ScriptedDrainConnectivitySource {
        fn next_status(&mut self) -> Option<ConnectivityStatus> {
            if self.drain_boundary_pending {
                self.drain_boundary_pending = false;
                return None;
            }

            if self.active_drain.is_empty() {
                self.active_drain = self.bursts.pop_front().unwrap_or_default();
                if self.active_drain.is_empty() {
                    return None;
                }
            }

            let status = self.active_drain.pop_front();
            if self.active_drain.is_empty() {
                self.drain_boundary_pending = true;
            }
            status
        }
    }

    #[derive(Default)]
    struct TestTerminalEventSource {
        immediate_polls: VecDeque<bool>,
        blocking_polls: VecDeque<bool>,
        events: VecDeque<Event>,
    }

    impl TestTerminalEventSource {
        fn with_blocking_polls(blocking_polls: Vec<bool>) -> Self {
            Self {
                immediate_polls: VecDeque::new(),
                blocking_polls: blocking_polls.into(),
                events: VecDeque::new(),
            }
        }

        fn with_non_blocking_polls_and_events(
            immediate_polls: Vec<bool>,
            events: Vec<Event>,
        ) -> Self {
            Self {
                immediate_polls: immediate_polls.into(),
                blocking_polls: VecDeque::new(),
                events: events.into(),
            }
        }
    }

    impl TerminalEventSource for TestTerminalEventSource {
        fn poll(&mut self, timeout: Duration) -> Result<bool> {
            if timeout == NON_BLOCKING_POLL_TIMEOUT {
                return Ok(self.immediate_polls.pop_front().unwrap_or(false));
            }

            Ok(self.blocking_polls.pop_front().unwrap_or(false))
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
            vec![
                AppEvent::Tick,
                AppEvent::InputKey(KeyInput::new("x", false)),
            ],
            vec![],
        );

        assert_eq!(
            source.next_event().expect("tick must be emitted"),
            Some(AppEvent::Tick)
        );
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
        let mut terminal = TestTerminalEventSource::with_blocking_polls(vec![false, false, false]);

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
    fn crossterm_event_source_prioritizes_ready_input_over_connectivity_burst() {
        let mut source = CrosstermEventSource::new(Box::new(TestConnectivitySource::from(vec![
            ConnectivityStatus::Connected,
            ConnectivityStatus::Disconnected,
            ConnectivityStatus::Connecting,
            ConnectivityStatus::Connected,
            ConnectivityStatus::Disconnected,
            ConnectivityStatus::Connecting,
        ])));

        let mut terminal = TestTerminalEventSource::with_non_blocking_polls_and_events(
            vec![true, true, false, false],
            vec![
                Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)),
                Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)),
            ],
        );

        let first = source
            .next_event_with_terminal(&mut terminal)
            .expect("first event should be readable");
        let second = source
            .next_event_with_terminal(&mut terminal)
            .expect("second event should be readable");
        let third = source
            .next_event_with_terminal(&mut terminal)
            .expect("third event should be readable");

        assert_eq!(first, Some(AppEvent::InputKey(KeyInput::new("x", false))));
        assert_eq!(second, Some(AppEvent::QuitRequested));
        assert!(matches!(third, Some(AppEvent::ConnectivityChanged(_))));
    }

    #[test]
    fn crossterm_event_source_emits_tick_after_connectivity_streak_limit() {
        let mut source = CrosstermEventSource::new(Box::new(
            ScriptedDrainConnectivitySource::from_bursts(vec![
                vec![
                    ConnectivityStatus::Connecting,
                    ConnectivityStatus::Connected,
                ],
                vec![
                    ConnectivityStatus::Connecting,
                    ConnectivityStatus::Disconnected,
                ],
                vec![
                    ConnectivityStatus::Connecting,
                    ConnectivityStatus::Connected,
                ],
                vec![
                    ConnectivityStatus::Connecting,
                    ConnectivityStatus::Disconnected,
                ],
                vec![
                    ConnectivityStatus::Connecting,
                    ConnectivityStatus::Connected,
                ],
                vec![
                    ConnectivityStatus::Connecting,
                    ConnectivityStatus::Disconnected,
                ],
                vec![
                    ConnectivityStatus::Connecting,
                    ConnectivityStatus::Connected,
                ],
                vec![
                    ConnectivityStatus::Connecting,
                    ConnectivityStatus::Disconnected,
                ],
            ]),
        ));
        let mut terminal = TestTerminalEventSource::with_blocking_polls(vec![false; 8]);

        let mut produced = Vec::new();
        for _ in 0..8 {
            produced.push(
                source
                    .next_event_with_terminal(&mut terminal)
                    .expect("event should be readable")
                    .expect("test sequence should produce events"),
            );
        }

        assert_eq!(
            produced,
            vec![
                AppEvent::ConnectivityChanged(ConnectivityStatus::Connected),
                AppEvent::ConnectivityChanged(ConnectivityStatus::Disconnected),
                AppEvent::ConnectivityChanged(ConnectivityStatus::Connected),
                AppEvent::Tick,
                AppEvent::Tick,
                AppEvent::ConnectivityChanged(ConnectivityStatus::Disconnected),
                AppEvent::ConnectivityChanged(ConnectivityStatus::Connected),
                AppEvent::ConnectivityChanged(ConnectivityStatus::Disconnected),
            ]
        );
    }

    #[test]
    fn crossterm_event_source_emits_ready_input_even_with_pending_connectivity() {
        let mut source = CrosstermEventSource::new(Box::new(
            ScriptedDrainConnectivitySource::from_bursts(vec![
                vec![
                    ConnectivityStatus::Connecting,
                    ConnectivityStatus::Connected,
                ],
                vec![
                    ConnectivityStatus::Connecting,
                    ConnectivityStatus::Disconnected,
                ],
                vec![
                    ConnectivityStatus::Connecting,
                    ConnectivityStatus::Connected,
                ],
                vec![
                    ConnectivityStatus::Connecting,
                    ConnectivityStatus::Disconnected,
                ],
            ]),
        ));
        let mut terminal = TestTerminalEventSource::with_non_blocking_polls_and_events(
            vec![false, false, false, true],
            vec![Event::Key(KeyEvent::new(
                KeyCode::Char('q'),
                KeyModifiers::NONE,
            ))],
        );

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
            Some(AppEvent::ConnectivityChanged(
                ConnectivityStatus::Disconnected
            ))
        );
        assert_eq!(
            source
                .next_event_with_terminal(&mut terminal)
                .expect("third event should be readable"),
            Some(AppEvent::ConnectivityChanged(ConnectivityStatus::Connected))
        );
        assert_eq!(
            source
                .next_event_with_terminal(&mut terminal)
                .expect("fourth event should be readable"),
            Some(AppEvent::QuitRequested)
        );
    }

    #[test]
    fn crossterm_event_source_deduplicates_same_connectivity_status() {
        let mut source = CrosstermEventSource::new(Box::new(TestConnectivitySource::from(vec![
            ConnectivityStatus::Connected,
            ConnectivityStatus::Connected,
            ConnectivityStatus::Connected,
        ])));
        let mut terminal = TestTerminalEventSource::with_blocking_polls(vec![false, false, false]);

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

    #[test]
    fn channel_connectivity_source_returns_latest_status_in_burst() {
        let mut source = ChannelConnectivityStatusSource::from_values(vec![
            ConnectivityStatus::Connecting,
            ConnectivityStatus::Disconnected,
            ConnectivityStatus::Connected,
        ]);

        assert_eq!(source.next_status(), Some(ConnectivityStatus::Connected));
        assert_eq!(source.next_status(), None);
    }

    #[test]
    fn channel_connectivity_source_is_non_blocking_when_channel_is_empty() {
        let (_tx, rx) = mpsc::channel::<ConnectivityStatus>();
        let mut source = ChannelConnectivityStatusSource::new(rx);

        assert_eq!(source.next_status(), None);
    }

    #[test]
    fn crossterm_event_source_keeps_non_connectivity_progress_with_channel_source() {
        let mut source = CrosstermEventSource::new(Box::new(
            ChannelConnectivityStatusSource::from_values(vec![
                ConnectivityStatus::Connecting,
                ConnectivityStatus::Connected,
                ConnectivityStatus::Disconnected,
                ConnectivityStatus::Connected,
            ]),
        ));

        let mut terminal = TestTerminalEventSource::with_non_blocking_polls_and_events(
            vec![true, false, false],
            vec![Event::Key(KeyEvent::new(
                KeyCode::Char('q'),
                KeyModifiers::NONE,
            ))],
        );

        assert_eq!(
            source
                .next_event_with_terminal(&mut terminal)
                .expect("quit event should be readable"),
            Some(AppEvent::QuitRequested)
        );

        assert!(matches!(
            source
                .next_event_with_terminal(&mut terminal)
                .expect("connectivity event should be readable"),
            Some(AppEvent::ConnectivityChanged(_))
        ));
    }
}
