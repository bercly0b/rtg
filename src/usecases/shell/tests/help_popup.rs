use super::*;

// ── Help popup tests ──

#[test]
fn question_mark_opens_help_from_chat_list() {
    let mut o = make_orchestrator();
    o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
        .unwrap();
    assert!(o.state().help_visible());
    assert!(o.state().is_running());
}

#[test]
fn question_mark_opens_help_from_messages() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hi")]);
    assert_eq!(o.state().active_pane(), ActivePane::Messages);
    o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
        .unwrap();
    assert!(o.state().help_visible());
}

#[test]
fn question_mark_types_in_message_input_instead_of_opening_help() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hi")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
        .unwrap();
    assert!(!o.state().help_visible());
    assert_eq!(o.state().message_input().text(), "?");
}

#[test]
fn q_closes_help_without_quitting() {
    let mut o = make_orchestrator();
    o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
        .unwrap();
    assert!(o.state().help_visible());

    o.handle_event(AppEvent::InputKey(KeyInput::new("q", false)))
        .unwrap();
    assert!(!o.state().help_visible());
    assert!(o.state().is_running());
}

#[test]
fn question_mark_closes_help_when_already_open() {
    let mut o = make_orchestrator();
    o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
        .unwrap();
    assert!(!o.state().help_visible());
    assert!(o.state().is_running());
}

#[test]
fn esc_closes_help() {
    let mut o = make_orchestrator();
    o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("esc", false)))
        .unwrap();
    assert!(!o.state().help_visible());
    assert!(o.state().is_running());
}

#[test]
fn other_keys_ignored_while_help_is_open() {
    let mut o = orchestrator_with_chats(vec![chat(1, "A"), chat(2, "B")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
        .unwrap();

    // j should not move selection while help is visible
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    assert_eq!(o.state().chat_list().selected_index(), Some(0));
    assert!(o.state().help_visible());
}

#[test]
fn ctrl_c_quits_even_when_help_is_open() {
    let mut o = make_orchestrator();
    o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
        .unwrap();
    o.handle_event(AppEvent::QuitRequested).unwrap();
    assert!(!o.state().is_running());
}

#[test]
fn q_quits_from_chat_list_when_help_is_not_open() {
    let mut o = make_orchestrator();
    o.handle_event(AppEvent::InputKey(KeyInput::new("q", false)))
        .unwrap();
    assert!(!o.state().is_running());
}

#[test]
fn q_quits_from_messages_when_help_is_not_open() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hi")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("q", false)))
        .unwrap();
    assert!(!o.state().is_running());
}

#[test]
fn help_does_not_change_active_pane() {
    let mut o = make_orchestrator();
    assert_eq!(o.state().active_pane(), ActivePane::ChatList);
    o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
        .unwrap();
    assert_eq!(o.state().active_pane(), ActivePane::ChatList);
}

#[test]
fn help_does_not_change_active_pane_in_messages() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hi")]);
    assert_eq!(o.state().active_pane(), ActivePane::Messages);
    o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
        .unwrap();
    assert_eq!(o.state().active_pane(), ActivePane::Messages);
}

#[test]
fn enter_key_ignored_while_help_is_open() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    // Should still be on chat list, not opened a chat
    assert_eq!(o.state().active_pane(), ActivePane::ChatList);
    assert!(o.state().help_visible());
}

#[test]
fn help_close_then_q_quits() {
    let mut o = make_orchestrator();
    o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("q", false)))
        .unwrap();
    assert!(!o.state().help_visible());
    assert!(o.state().is_running());

    // Now q again should quit
    o.handle_event(AppEvent::InputKey(KeyInput::new("q", false)))
        .unwrap();
    assert!(!o.state().is_running());
}

#[test]
fn ctrl_o_ignored_while_help_is_open() {
    let mut o = make_orchestrator();
    o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
        .unwrap();
    // Ctrl+O should not open browser while help is visible
    o.handle_event(AppEvent::InputKey(KeyInput::new("o", true)))
        .unwrap();
    assert!(o.state().help_visible());
}

#[test]
fn help_open_from_messages_then_esc_closes_help_not_pane() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hi")]);
    assert_eq!(o.state().active_pane(), ActivePane::Messages);

    o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("esc", false)))
        .unwrap();

    // Help closed, but still in Messages pane (not back to ChatList)
    assert!(!o.state().help_visible());
    assert_eq!(o.state().active_pane(), ActivePane::Messages);
}

#[test]
fn multiple_help_toggle_cycles() {
    let mut o = make_orchestrator();
    for _ in 0..5 {
        o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
            .unwrap();
        assert!(o.state().help_visible());
        o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
            .unwrap();
        assert!(!o.state().help_visible());
    }
    assert!(o.state().is_running());
}

#[test]
fn tick_events_still_processed_while_help_is_open() {
    let mut o = make_orchestrator();
    o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
        .unwrap();
    // Tick should not error or panic while help is visible
    o.handle_event(AppEvent::Tick).unwrap();
    assert!(o.state().help_visible());
}

#[test]
fn connectivity_events_still_processed_while_help_is_open() {
    let mut o = make_orchestrator();
    o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
        .unwrap();
    o.handle_event(AppEvent::ConnectivityChanged(ConnectivityStatus::Connected))
        .unwrap();
    assert_eq!(
        o.state().connectivity_status(),
        ConnectivityStatus::Connected
    );
    assert!(o.state().help_visible());
}
