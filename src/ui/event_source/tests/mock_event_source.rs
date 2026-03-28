use crate::{
    domain::events::{AppEvent, KeyInput},
    usecases::contracts::AppEventSource,
};

use super::super::MockEventSource;

#[test]
fn returns_none_when_queue_is_exhausted() {
    let mut source = MockEventSource::from(vec![AppEvent::Tick]);

    assert_eq!(
        source.next_event().expect("first event must be read"),
        Some(AppEvent::Tick)
    );
    assert_eq!(source.next_event().expect("queue must be empty"), None);
}

#[test]
fn keeps_tick_input_path_when_no_connectivity_event_available() {
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
