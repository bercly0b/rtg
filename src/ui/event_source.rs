use std::{sync::mpsc::Receiver, time::Duration};

#[cfg(test)]
use std::{collections::VecDeque, sync::mpsc};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::{
    domain::events::{AppEvent, BackgroundTaskResult, ChatUpdate, ConnectivityStatus, KeyInput},
    usecases::contracts::AppEventSource,
};

const EVENT_POLL_TIMEOUT: Duration = Duration::from_millis(100);
const NON_BLOCKING_POLL_TIMEOUT: Duration = Duration::from_millis(0);
const MAX_CONNECTIVITY_STREAK: u8 = 3;
const MAX_CHAT_UPDATE_STREAK: u8 = 8;
const MAX_CONNECTIVITY_DRAIN_PER_CYCLE: usize = 32;

pub trait ConnectivityStatusSource {
    fn next_status(&mut self) -> Option<ConnectivityStatus>;
}

pub trait ChatUpdatesSignalSource {
    fn pending_updates(&mut self) -> Option<Vec<ChatUpdate>>;
}

#[derive(Default)]
pub struct StubConnectivityStatusSource;

impl ConnectivityStatusSource for StubConnectivityStatusSource {
    fn next_status(&mut self) -> Option<ConnectivityStatus> {
        None
    }
}

#[derive(Default)]
pub struct StubChatUpdatesSignalSource;

impl ChatUpdatesSignalSource for StubChatUpdatesSignalSource {
    fn pending_updates(&mut self) -> Option<Vec<ChatUpdate>> {
        None
    }
}

pub trait BackgroundResultSource {
    fn next_result(&mut self) -> Option<BackgroundTaskResult>;
}

#[derive(Default)]
pub struct StubBackgroundResultSource;

impl BackgroundResultSource for StubBackgroundResultSource {
    fn next_result(&mut self) -> Option<BackgroundTaskResult> {
        None
    }
}

pub use crate::domain::events::CommandEvent;

pub trait CommandOutputSource {
    fn next_command_event(&mut self) -> Option<CommandEvent>;
}

#[derive(Default)]
pub struct StubCommandOutputSource;

impl CommandOutputSource for StubCommandOutputSource {
    fn next_command_event(&mut self) -> Option<CommandEvent> {
        None
    }
}

pub struct ChannelCommandOutputSource {
    receiver: Receiver<CommandEvent>,
}

impl ChannelCommandOutputSource {
    pub fn new(receiver: Receiver<CommandEvent>) -> Self {
        Self { receiver }
    }
}

impl CommandOutputSource for ChannelCommandOutputSource {
    fn next_command_event(&mut self) -> Option<CommandEvent> {
        self.receiver.try_recv().ok()
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

pub struct ChannelChatUpdatesSignalSource {
    receiver: Receiver<ChatUpdate>,
}

impl ChannelChatUpdatesSignalSource {
    pub fn new(receiver: Receiver<ChatUpdate>) -> Self {
        Self { receiver }
    }

    #[cfg(test)]
    pub fn from_updates(updates: Vec<ChatUpdate>) -> Self {
        let (tx, rx) = mpsc::channel();
        for update in updates {
            tx.send(update).expect("update should be sent");
        }
        Self::new(rx)
    }
}

impl ChatUpdatesSignalSource for ChannelChatUpdatesSignalSource {
    fn pending_updates(&mut self) -> Option<Vec<ChatUpdate>> {
        let mut updates = Vec::new();
        while let Ok(update) = self.receiver.try_recv() {
            updates.push(update);
        }
        if updates.is_empty() {
            None
        } else {
            Some(updates)
        }
    }
}

pub struct ChannelBackgroundResultSource {
    receiver: Receiver<BackgroundTaskResult>,
}

impl ChannelBackgroundResultSource {
    pub fn new(receiver: Receiver<BackgroundTaskResult>) -> Self {
        Self { receiver }
    }
}

impl BackgroundResultSource for ChannelBackgroundResultSource {
    fn next_result(&mut self) -> Option<BackgroundTaskResult> {
        self.receiver.try_recv().ok()
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
    chat_updates_source: Box<dyn ChatUpdatesSignalSource>,
    background_result_source: Box<dyn BackgroundResultSource>,
    command_output_source: Box<dyn CommandOutputSource>,
    pending_connectivity: Option<ConnectivityStatus>,
    last_emitted_connectivity: Option<ConnectivityStatus>,
    connectivity_streak: u8,
    chat_update_streak: u8,
}

impl Default for CrosstermEventSource {
    fn default() -> Self {
        Self {
            connectivity_source: Box::new(StubConnectivityStatusSource),
            chat_updates_source: Box::new(StubChatUpdatesSignalSource),
            background_result_source: Box::new(StubBackgroundResultSource),
            command_output_source: Box::new(StubCommandOutputSource),
            pending_connectivity: None,
            last_emitted_connectivity: None,
            connectivity_streak: 0,
            chat_update_streak: 0,
        }
    }
}

impl CrosstermEventSource {
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn new(connectivity_source: Box<dyn ConnectivityStatusSource>) -> Self {
        Self::with_sources(
            connectivity_source,
            Box::new(StubChatUpdatesSignalSource),
            Box::new(StubBackgroundResultSource),
        )
    }

    pub fn with_sources(
        connectivity_source: Box<dyn ConnectivityStatusSource>,
        chat_updates_source: Box<dyn ChatUpdatesSignalSource>,
        background_result_source: Box<dyn BackgroundResultSource>,
    ) -> Self {
        Self {
            connectivity_source,
            chat_updates_source,
            background_result_source,
            command_output_source: Box::new(StubCommandOutputSource),
            pending_connectivity: None,
            last_emitted_connectivity: None,
            connectivity_streak: 0,
            chat_update_streak: 0,
        }
    }

    /// Replaces the command output source (called when an external command starts).
    pub fn set_command_output_source(&mut self, source: Box<dyn CommandOutputSource>) {
        self.command_output_source = source;
    }

    /// Clears the command output source (called when the command popup closes).
    pub fn clear_command_output_source(&mut self) {
        self.command_output_source = Box::new(StubCommandOutputSource);
    }

    fn next_event_with_terminal<T: TerminalEventSource>(
        &mut self,
        terminal: &mut T,
    ) -> Result<Option<AppEvent>> {
        self.refresh_pending_connectivity();

        let has_ready_terminal_input = terminal.poll(NON_BLOCKING_POLL_TIMEOUT).unwrap_or(false);
        if has_ready_terminal_input {
            self.connectivity_streak = 0;
            self.chat_update_streak = 0;
            if let Event::Key(key) = terminal.read()? {
                return Ok(map_key_event(key));
            }
            return Ok(None);
        }

        // Command output has the highest non-input priority so the popup
        // updates in real time while the external process is running.
        if let Some(cmd_event) = self.command_output_source.next_command_event() {
            self.connectivity_streak = 0;
            self.chat_update_streak = 0;
            return Ok(Some(match cmd_event {
                CommandEvent::OutputLine { text, replace_last } => {
                    AppEvent::CommandOutputLine { text, replace_last }
                }
                CommandEvent::Exited { success } => AppEvent::CommandExited { success },
            }));
        }

        // Background task results have high priority — deliver them before
        // chat updates to keep the UI responsive after dispatched operations.
        if let Some(result) = self.background_result_source.next_result() {
            self.connectivity_streak = 0;
            self.chat_update_streak = 0;
            return Ok(Some(AppEvent::BackgroundTaskCompleted(result)));
        }

        if self.chat_update_streak < MAX_CHAT_UPDATE_STREAK {
            if let Some(updates) = self.chat_updates_source.pending_updates() {
                self.connectivity_streak = 0;
                self.chat_update_streak += 1;
                tracing::debug!(
                    update_count = updates.len(),
                    "event source emitted chat update received"
                );
                return Ok(Some(AppEvent::ChatUpdateReceived { updates }));
            }
        }

        if self.connectivity_streak < MAX_CONNECTIVITY_STREAK {
            if let Some(status) = self.pending_connectivity.take() {
                self.connectivity_streak += 1;
                self.last_emitted_connectivity = Some(status);
                return Ok(Some(AppEvent::ConnectivityChanged(status)));
            }
        }

        self.connectivity_streak = 0;
        self.chat_update_streak = 0;

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

    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Some(AppEvent::QuitRequested);
    }

    if let KeyCode::Char(ch) = key.code {
        return Some(AppEvent::InputKey(KeyInput::new(
            ch.to_string(),
            key.modifiers.contains(KeyModifiers::CONTROL),
        )));
    }

    let special_key = match key.code {
        KeyCode::Enter => Some("enter"),
        KeyCode::Esc => Some("esc"),
        KeyCode::Backspace => Some("backspace"),
        KeyCode::Delete => Some("delete"),
        KeyCode::Left => Some("left"),
        KeyCode::Right => Some("right"),
        KeyCode::Home => Some("home"),
        KeyCode::End => Some("end"),
        _ => None,
    };

    special_key.map(|k| AppEvent::InputKey(KeyInput::new(k, false)))
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

    struct TestChatUpdatesSource {
        results: VecDeque<Option<Vec<ChatUpdate>>>,
    }

    impl TestChatUpdatesSource {
        fn from_bools(bools: Vec<bool>) -> Self {
            Self {
                results: bools
                    .into_iter()
                    .map(|b| {
                        if b {
                            Some(vec![ChatUpdate::ChatMetadataChanged { chat_id: 1 }])
                        } else {
                            None
                        }
                    })
                    .collect(),
            }
        }
    }

    impl ChatUpdatesSignalSource for TestChatUpdatesSource {
        fn pending_updates(&mut self) -> Option<Vec<ChatUpdate>> {
            self.results.pop_front().flatten()
        }
    }

    #[derive(Default)]
    struct BurstyChatUpdatesSource;

    impl ChatUpdatesSignalSource for BurstyChatUpdatesSource {
        fn pending_updates(&mut self) -> Option<Vec<ChatUpdate>> {
            Some(vec![ChatUpdate::ChatMetadataChanged { chat_id: 1 }])
        }
    }

    #[derive(Default)]
    struct BurstyConnectivitySource {
        connected: bool,
    }

    impl ConnectivityStatusSource for BurstyConnectivitySource {
        fn next_status(&mut self) -> Option<ConnectivityStatus> {
            self.connected = !self.connected;
            Some(if self.connected {
                ConnectivityStatus::Connected
            } else {
                ConnectivityStatus::Disconnected
            })
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
    fn crossterm_event_source_prioritizes_ready_input_over_connectivity_burst() {
        let mut source = CrosstermEventSource::new(Box::new(TestConnectivitySource::from(vec![
            ConnectivityStatus::Connected,
            ConnectivityStatus::Disconnected,
            ConnectivityStatus::Connecting,
            ConnectivityStatus::Connected,
            ConnectivityStatus::Disconnected,
            ConnectivityStatus::Connecting,
        ])));

        let mut terminal = TestTerminalEventSource::with_polls_and_events(
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
        assert_eq!(second, Some(AppEvent::InputKey(KeyInput::new("q", false))));
        assert!(matches!(third, Some(AppEvent::ConnectivityChanged(_))));
    }

    #[test]
    fn crossterm_event_source_does_not_starve_tick_under_bursty_connectivity() {
        let mut source = CrosstermEventSource::new(Box::new(BurstyConnectivitySource::default()));
        let mut terminal = TestTerminalEventSource::with_polls(vec![false; 32]);

        let mut produced = Vec::new();
        for _ in 0..8 {
            produced.push(
                source
                    .next_event_with_terminal(&mut terminal)
                    .expect("event should be readable")
                    .expect("test sequence should produce events"),
            );
        }

        assert!(
            produced.iter().any(|event| matches!(event, AppEvent::Tick)),
            "tick should be emitted even when connectivity changes every cycle"
        );
    }

    #[test]
    fn crossterm_event_source_does_not_starve_input_under_bursty_connectivity() {
        let mut source = CrosstermEventSource::new(Box::new(BurstyConnectivitySource::default()));
        let mut terminal = TestTerminalEventSource::with_polls_and_events(
            vec![true, true, true, true],
            vec![
                Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)),
                Event::Key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE)),
                Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE)),
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
        let fourth = source
            .next_event_with_terminal(&mut terminal)
            .expect("fourth event should be readable");

        assert_eq!(first, Some(AppEvent::InputKey(KeyInput::new("a", false))));
        assert_eq!(second, Some(AppEvent::InputKey(KeyInput::new("b", false))));
        assert_eq!(third, Some(AppEvent::InputKey(KeyInput::new("c", false))));
        assert_eq!(fourth, Some(AppEvent::InputKey(KeyInput::new("q", false))));
    }

    #[test]
    fn crossterm_event_source_maps_enter_to_open_chat_intent_key() {
        let mut source = CrosstermEventSource::default();
        let mut terminal = TestTerminalEventSource::with_polls_and_events(
            vec![true],
            vec![Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            ))],
        );

        assert_eq!(
            source
                .next_event_with_terminal(&mut terminal)
                .expect("enter key event should be readable"),
            Some(AppEvent::InputKey(KeyInput::new("enter", false)))
        );
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
        let mut source = CrosstermEventSource::with_sources(
            Box::new(ChannelConnectivityStatusSource::from_values(vec![
                ConnectivityStatus::Connecting,
                ConnectivityStatus::Connected,
                ConnectivityStatus::Disconnected,
                ConnectivityStatus::Connected,
            ])),
            Box::new(StubChatUpdatesSignalSource),
            Box::new(StubBackgroundResultSource),
        );

        let mut terminal = TestTerminalEventSource::with_polls_and_events(
            vec![true, false, false],
            vec![Event::Key(KeyEvent::new(
                KeyCode::Char('q'),
                KeyModifiers::NONE,
            ))],
        );

        assert_eq!(
            source
                .next_event_with_terminal(&mut terminal)
                .expect("q key event should be readable"),
            Some(AppEvent::InputKey(KeyInput::new("q", false)))
        );

        assert!(matches!(
            source
                .next_event_with_terminal(&mut terminal)
                .expect("connectivity event should be readable"),
            Some(AppEvent::ConnectivityChanged(_))
        ));
    }

    #[test]
    fn channel_chat_updates_source_drains_burst_into_single_batch() {
        let mut source = ChannelChatUpdatesSignalSource::from_updates(vec![
            ChatUpdate::ChatMetadataChanged { chat_id: 1 },
            ChatUpdate::ChatMetadataChanged { chat_id: 2 },
            ChatUpdate::ChatMetadataChanged { chat_id: 3 },
        ]);

        let updates = source.pending_updates();
        assert_eq!(updates.as_ref().map(|u| u.len()), Some(3));
        assert_eq!(source.pending_updates(), None);
    }

    #[test]
    fn crossterm_event_source_emits_chat_update_received_event() {
        let mut source = CrosstermEventSource::with_sources(
            Box::new(StubConnectivityStatusSource),
            Box::new(TestChatUpdatesSource::from_bools(vec![true, false])),
            Box::new(StubBackgroundResultSource),
        );
        let mut terminal = TestTerminalEventSource::with_polls(vec![false, false]);

        assert!(matches!(
            source
                .next_event_with_terminal(&mut terminal)
                .expect("chat update event should be readable"),
            Some(AppEvent::ChatUpdateReceived { .. })
        ));
        assert_eq!(
            source
                .next_event_with_terminal(&mut terminal)
                .expect("next event should remain available"),
            Some(AppEvent::Tick)
        );
    }

    #[test]
    fn crossterm_event_source_emits_update_per_chat_update_signal() {
        let mut source = CrosstermEventSource::with_sources(
            Box::new(StubConnectivityStatusSource),
            Box::new(TestChatUpdatesSource::from_bools(vec![true, true, false])),
            Box::new(StubBackgroundResultSource),
        );
        let mut terminal = TestTerminalEventSource::with_polls(vec![false, false, false]);

        assert!(matches!(
            source
                .next_event_with_terminal(&mut terminal)
                .expect("first chat update event should be readable"),
            Some(AppEvent::ChatUpdateReceived { .. })
        ));
        assert!(matches!(
            source
                .next_event_with_terminal(&mut terminal)
                .expect("second chat update event should be readable"),
            Some(AppEvent::ChatUpdateReceived { .. })
        ));
        assert_eq!(
            source
                .next_event_with_terminal(&mut terminal)
                .expect("third event should remain available"),
            Some(AppEvent::Tick)
        );
    }

    #[test]
    fn crossterm_event_source_does_not_starve_tick_under_bursty_chat_updates() {
        let mut source = CrosstermEventSource::with_sources(
            Box::new(StubConnectivityStatusSource),
            Box::new(BurstyChatUpdatesSource),
            Box::new(StubBackgroundResultSource),
        );
        let mut terminal = TestTerminalEventSource::with_polls(vec![false; 32]);

        let mut produced = Vec::new();
        for _ in 0..12 {
            produced.push(
                source
                    .next_event_with_terminal(&mut terminal)
                    .expect("event should be readable")
                    .expect("test sequence should produce events"),
            );
        }

        assert!(
            produced
                .iter()
                .any(|event| matches!(event, AppEvent::ChatUpdateReceived { .. })),
            "chat updates should be emitted during burst"
        );
        assert!(
            produced.iter().any(|event| matches!(event, AppEvent::Tick)),
            "tick should still be emitted under chat update burst"
        );
    }

    // ── map_key_event unit tests ──

    #[test]
    fn map_key_event_q_produces_input_key_not_quit() {
        let event = map_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert_eq!(event, Some(AppEvent::InputKey(KeyInput::new("q", false))));
    }

    #[test]
    fn map_key_event_ctrl_c_produces_quit_requested() {
        let event = map_key_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert_eq!(event, Some(AppEvent::QuitRequested));
    }

    #[test]
    fn map_key_event_question_mark_produces_input_key() {
        let event = map_key_event(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
        assert_eq!(event, Some(AppEvent::InputKey(KeyInput::new("?", false))));
    }

    #[test]
    fn map_key_event_ignores_release_events() {
        let mut event = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        event.kind = KeyEventKind::Release;
        assert_eq!(map_key_event(event), None);
    }

    #[test]
    fn map_key_event_special_keys_produce_named_input() {
        let cases = vec![
            (KeyCode::Enter, "enter"),
            (KeyCode::Esc, "esc"),
            (KeyCode::Backspace, "backspace"),
            (KeyCode::Delete, "delete"),
            (KeyCode::Left, "left"),
            (KeyCode::Right, "right"),
            (KeyCode::Home, "home"),
            (KeyCode::End, "end"),
        ];
        for (code, expected) in cases {
            let event = map_key_event(KeyEvent::new(code, KeyModifiers::NONE));
            assert_eq!(
                event,
                Some(AppEvent::InputKey(KeyInput::new(expected, false))),
                "failed for key code {:?}",
                code
            );
        }
    }

    #[test]
    fn map_key_event_ctrl_o_produces_ctrl_input_key() {
        let event = map_key_event(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL));
        assert_eq!(event, Some(AppEvent::InputKey(KeyInput::new("o", true))));
    }

    #[test]
    fn map_key_event_unknown_special_key_returns_none() {
        let event = map_key_event(KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE));
        assert_eq!(event, None);
    }
}
