use super::*;

#[test]
fn stops_on_quit_event() {
    let mut o = make_orchestrator();
    o.handle_event(AppEvent::QuitRequested).unwrap();
    assert!(!o.state().is_running());
}

#[test]
fn keeps_running_on_regular_key() {
    let mut o = make_orchestrator();
    o.handle_event(AppEvent::InputKey(KeyInput::new("x", false)))
        .unwrap();
    assert!(o.state().is_running());
}

#[test]
fn updates_connectivity_status_on_connectivity_event() {
    let mut o = make_orchestrator();
    o.handle_event(AppEvent::ConnectivityChanged(
        ConnectivityStatus::Disconnected,
    ))
    .unwrap();
    assert_eq!(
        o.state().connectivity_status(),
        ConnectivityStatus::Disconnected
    );
}
#[test]
fn integration_smoke_happy_path_startup_load_navigate_and_open_chat() {
    let mut o = make_orchestrator();
    assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Loading);

    // Tick dispatches chat list load
    o.handle_event(AppEvent::Tick).unwrap();
    // Simulate result
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::ChatListLoaded {
            result: Ok(vec![chat(1, "General"), chat(2, "Backend"), chat(3, "Ops")]),
        },
    ))
    .unwrap();

    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    // Simulate messages loaded
    inject_messages(&mut o, 2, vec![message(1, "Hello")]);

    assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Ready);
    assert_eq!(o.state().chat_list().selected_index(), Some(1));
    assert_eq!(
        o.state().chat_list().selected_chat().map(|c| c.chat_id),
        Some(2)
    );
    assert_eq!(o.state().open_chat().chat_id(), Some(2));
    assert_eq!(o.state().open_chat().chat_title(), "Backend");
    assert_eq!(o.state().open_chat().messages().len(), 1);
    assert_eq!(o.storage.last_action, Some("open_chat".to_owned()));
}

#[test]
fn integration_smoke_fallback_error_then_empty_list() {
    let mut o = make_orchestrator();

    // Tick dispatches
    o.handle_event(AppEvent::Tick).unwrap();
    // Error result
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::ChatListLoaded {
            result: Err(BackgroundError::new("CHAT_LIST_UNAVAILABLE")),
        },
    ))
    .unwrap();
    assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Error);

    // Press r to retry
    o.handle_event(AppEvent::InputKey(KeyInput::new("r", false)))
        .unwrap();
    // Empty list result
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::ChatListLoaded { result: Ok(vec![]) },
    ))
    .unwrap();

    assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Empty);
    assert_eq!(o.state().chat_list().selected_index(), None);
    assert!(o.state().is_running());
}
