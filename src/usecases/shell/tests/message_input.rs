use super::*;

#[test]
fn l_key_does_nothing_when_no_chat_selected() {
    let mut o = orchestrator_with_chats(vec![]);
    // ui_state is Empty when no chats
    o.handle_event(AppEvent::InputKey(KeyInput::new("l", false)))
        .unwrap();
    assert_eq!(o.state().active_pane(), ActivePane::ChatList);
    assert!(!o.state().open_chat().is_open());
}

#[test]
fn i_key_switches_to_message_input_mode_when_chat_is_open() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
        .unwrap();
    assert_eq!(o.state().active_pane(), ActivePane::MessageInput);
}

#[test]
fn i_key_does_nothing_when_no_chat_is_open() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General")]);
    o.state.set_active_pane(ActivePane::Messages);

    o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
        .unwrap();
    assert_eq!(o.state().active_pane(), ActivePane::Messages);
}

#[test]
fn esc_key_switches_from_message_input_to_messages_pane() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
        .unwrap();
    assert_eq!(o.state().active_pane(), ActivePane::MessageInput);

    o.handle_event(AppEvent::InputKey(KeyInput::new("esc", false)))
        .unwrap();
    assert_eq!(o.state().active_pane(), ActivePane::Messages);
}

#[test]
fn text_input_in_message_input_mode() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
        .unwrap();

    o.handle_event(AppEvent::InputKey(KeyInput::new("H", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
        .unwrap();

    assert_eq!(o.state().message_input().text(), "Hi");
    assert_eq!(o.state().active_pane(), ActivePane::MessageInput);
}

#[test]
fn backspace_deletes_character_in_message_input_mode() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
        .unwrap();

    o.handle_event(AppEvent::InputKey(KeyInput::new("H", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("backspace", false)))
        .unwrap();

    assert_eq!(o.state().message_input().text(), "H");
}

#[test]
fn cursor_navigation_in_message_input_mode() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
        .unwrap();

    for ch in ['a', 'b', 'c'] {
        o.handle_event(AppEvent::InputKey(KeyInput::new(ch.to_string(), false)))
            .unwrap();
    }
    assert_eq!(o.state().message_input().cursor_position(), 3);

    o.handle_event(AppEvent::InputKey(KeyInput::new("left", false)))
        .unwrap();
    assert_eq!(o.state().message_input().cursor_position(), 2);

    o.handle_event(AppEvent::InputKey(KeyInput::new("home", false)))
        .unwrap();
    assert_eq!(o.state().message_input().cursor_position(), 0);

    o.handle_event(AppEvent::InputKey(KeyInput::new("end", false)))
        .unwrap();
    assert_eq!(o.state().message_input().cursor_position(), 3);
}

#[test]
fn q_key_types_q_in_message_input_mode_instead_of_quitting() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
        .unwrap();

    o.handle_event(AppEvent::InputKey(KeyInput::new("q", false)))
        .unwrap();

    assert!(o.state().is_running());
    assert_eq!(o.state().message_input().text(), "q");
}

#[test]
fn message_input_state_preserved_when_switching_panes() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
        .unwrap();

    o.handle_event(AppEvent::InputKey(KeyInput::new("H", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
        .unwrap();

    o.handle_event(AppEvent::InputKey(KeyInput::new("esc", false)))
        .unwrap();
    assert_eq!(o.state().message_input().text(), "Hi");

    o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
        .unwrap();
    assert_eq!(o.state().message_input().text(), "Hi");
}

#[test]
fn enter_key_dispatches_send_message_and_clears_input() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
        .unwrap();

    o.handle_event(AppEvent::InputKey(KeyInput::new("H", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
        .unwrap();
    assert_eq!(o.state().message_input().text(), "Hi");

    // Press enter to send
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    // Input should be cleared optimistically
    assert_eq!(o.state().message_input().text(), "");
    assert_eq!(o.dispatcher.send_dispatch_count(), 1);
    assert_eq!(o.dispatcher.last_send(), Some((1, "Hi".to_owned(), None)));
    assert_eq!(o.state().active_pane(), ActivePane::MessageInput);
}

#[test]
fn message_sent_success_keeps_input_cleared() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("H", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    // Successful send result arrives
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessageSent {
            chat_id: 1,
            original_text: "Hi".to_owned(),
            result: Ok(()),
        },
    ))
    .unwrap();

    assert_eq!(o.state().message_input().text(), "");
}

#[test]
fn message_sent_error_restores_text_in_input() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
        .unwrap();

    for c in "Test message".chars() {
        o.handle_event(AppEvent::InputKey(KeyInput::new(&c.to_string(), false)))
            .unwrap();
    }

    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    assert_eq!(o.state().message_input().text(), "");

    // Send failure result arrives — text should be restored
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessageSent {
            chat_id: 1,
            original_text: "Test message".to_owned(),
            result: Err(BackgroundError::new("SEND_UNAVAILABLE")),
        },
    ))
    .unwrap();

    assert_eq!(o.state().message_input().text(), "Test message");
    assert_eq!(o.state().active_pane(), ActivePane::MessageInput);
}

#[test]
fn message_sent_refresh_updates_messages() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
    assert_eq!(o.state().open_chat().messages().len(), 1);

    // After a successful send, the refresh result arrives
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessageSentRefreshCompleted {
            chat_id: 1,
            result: Ok(vec![message(1, "Hello"), message(2, "Hi")]),
        },
    ))
    .unwrap();

    assert_eq!(o.state().open_chat().messages().len(), 2);
}

#[test]
fn enter_key_with_empty_input_does_nothing() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
        .unwrap();

    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    assert_eq!(o.state().message_input().text(), "");
    assert_eq!(o.dispatcher.send_dispatch_count(), 0);
}

#[test]
fn enter_key_with_whitespace_only_does_nothing() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
        .unwrap();

    o.handle_event(AppEvent::InputKey(KeyInput::new(" ", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new(" ", false)))
        .unwrap();

    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    assert_eq!(o.state().message_input().text(), "  ");
    assert_eq!(o.dispatcher.send_dispatch_count(), 0);
}

#[test]
fn rapid_pane_switching_maintains_consistent_state() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General"), chat(2, "Backend")]);

    // Open chat 1
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    inject_messages(&mut o, 1, vec![message(1, "Hello")]);
    assert_eq!(o.state().active_pane(), ActivePane::Messages);

    o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
        .unwrap();
    assert_eq!(o.state().active_pane(), ActivePane::ChatList);

    o.handle_event(AppEvent::InputKey(KeyInput::new("l", false)))
        .unwrap();
    assert_eq!(o.state().active_pane(), ActivePane::Messages);

    o.handle_event(AppEvent::InputKey(KeyInput::new("esc", false)))
        .unwrap();
    assert_eq!(o.state().active_pane(), ActivePane::ChatList);

    assert!(o.state().is_running());
    assert!(o.state().open_chat().is_open());
}
