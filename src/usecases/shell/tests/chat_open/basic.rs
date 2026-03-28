use super::*;

#[test]
fn enter_key_dispatches_load_messages_and_switches_pane() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General"), chat(2, "Backend")]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    assert_eq!(o.dispatcher.messages_dispatch_count(), 1);
    assert_eq!(o.state().open_chat().chat_id(), Some(1));
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);
    assert_eq!(o.state().active_pane(), ActivePane::Messages);
}

#[test]
fn messages_loaded_result_sets_ready_state() {
    let o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);

    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
    assert_eq!(o.state().open_chat().messages().len(), 1);
}

#[test]
fn messages_loaded_error_sets_error_state() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessagesLoaded {
            chat_id: 1,
            result: Err(BackgroundError::new("MESSAGES_UNAVAILABLE")),
        },
    ))
    .unwrap();

    assert_eq!(o.state().open_chat().chat_id(), Some(1));
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Error);
}

#[test]
fn stale_messages_result_is_discarded() {
    let mut o = orchestrator_with_chats(vec![chat(1, "A"), chat(2, "B")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessagesLoaded {
            chat_id: 1,
            result: Ok(vec![message(1, "Stale")]),
        },
    ))
    .unwrap();

    assert_eq!(o.state().open_chat().chat_id(), Some(2));
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);
}
