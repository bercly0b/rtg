use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::domain::events::{AppEvent, ConnectivityStatus, KeyInput};

use super::super::{CrosstermEventSource, StubBackgroundResultSource, StubChatUpdatesSignalSource};
use super::{
    BurstyChatUpdatesSource, BurstyConnectivitySource, TestChatUpdatesSource,
    TestConnectivitySource, TestTerminalEventSource,
};

#[test]
fn keeps_tick_progress_with_frequent_connectivity_events() {
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
fn prioritizes_ready_input_over_connectivity_burst() {
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
fn does_not_starve_tick_under_bursty_connectivity() {
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
fn does_not_starve_input_under_bursty_connectivity() {
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
fn maps_enter_to_input_key() {
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
fn deduplicates_same_connectivity_status() {
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
fn keeps_non_connectivity_progress_with_channel_source() {
    use super::super::ChannelConnectivityStatusSource;

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
fn emits_chat_update_received_event() {
    let mut source = CrosstermEventSource::with_sources(
        Box::new(super::super::StubConnectivityStatusSource),
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
fn emits_update_per_chat_update_signal() {
    let mut source = CrosstermEventSource::with_sources(
        Box::new(super::super::StubConnectivityStatusSource),
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
fn does_not_starve_tick_under_bursty_chat_updates() {
    let mut source = CrosstermEventSource::with_sources(
        Box::new(super::super::StubConnectivityStatusSource),
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

#[test]
fn does_not_starve_input_under_bursty_chat_updates() {
    let mut source = CrosstermEventSource::with_sources(
        Box::new(super::super::StubConnectivityStatusSource),
        Box::new(BurstyChatUpdatesSource),
        Box::new(StubBackgroundResultSource),
    );
    let mut terminal = TestTerminalEventSource::with_polls_and_events(
        vec![true, true, true, true],
        vec![
            Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
        ],
    );

    for _ in 0..4 {
        assert_eq!(
            source
                .next_event_with_terminal(&mut terminal)
                .expect("event should be readable"),
            Some(AppEvent::QuitRequested),
        );
    }
}

#[test]
fn round_robin_between_chat_updates_and_connectivity_when_both_hot() {
    let mut source = CrosstermEventSource::with_sources(
        Box::new(BurstyConnectivitySource::default()),
        Box::new(BurstyChatUpdatesSource),
        Box::new(StubBackgroundResultSource),
    );
    let mut terminal = TestTerminalEventSource::with_polls(vec![false; 64]);

    let mut produced = Vec::new();
    for _ in 0..4 {
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
        "chat updates should be emitted when both sources are hot"
    );
    assert!(
        produced
            .iter()
            .any(|event| matches!(event, AppEvent::ConnectivityChanged(_))),
        "connectivity should not be starved when both sources are hot"
    );
}
