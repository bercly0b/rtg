use super::*;

// ── dd (delete message) tests ──

#[test]
fn dd_deletes_selected_message() {
    let mut o = orchestrator_with_open_chat(
        vec![chat(1, "Chat")],
        1,
        vec![message(10, "hello"), message(20, "world")],
    );

    // Select last message (20) — default after open
    assert_eq!(o.state().open_chat().selected_message().unwrap().id, 20);

    // Press d, d
    o.handle_event(AppEvent::InputKey(KeyInput::new("d", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("d", false)))
        .unwrap();

    // Message 20 should be removed from UI
    assert_eq!(o.state().open_chat().messages().len(), 1);
    assert_eq!(o.state().open_chat().messages()[0].id, 10);

    // Dispatch should have been called
    assert_eq!(o.dispatcher.delete_dispatch_count(), 1);
    assert_eq!(o.dispatcher.last_delete(), Some((1, 20)));

    // Notification should be set
    assert_eq!(o.state().active_notification(), Some("Message deleted"));
}

#[test]
fn d_then_other_key_cancels_delete() {
    let mut o = orchestrator_with_open_chat(
        vec![chat(1, "Chat")],
        1,
        vec![message(10, "hello"), message(20, "world")],
    );

    // Press d, then j (navigate) — should cancel delete
    o.handle_event(AppEvent::InputKey(KeyInput::new("d", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();

    // No deletion should have happened
    assert_eq!(o.state().open_chat().messages().len(), 2);
    assert_eq!(o.dispatcher.delete_dispatch_count(), 0);
}

#[test]
fn dd_on_empty_chat_does_nothing() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("d", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("d", false)))
        .unwrap();

    assert_eq!(o.dispatcher.delete_dispatch_count(), 0);
}

#[test]
fn dd_does_not_delete_pending_messages() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hello")]);

    // Switch to message input and send a message (creates pending msg with id=0)
    o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
        .unwrap();
    // Type "test" + enter
    for ch in "test".chars() {
        o.handle_event(AppEvent::InputKey(KeyInput::new(ch.to_string(), false)))
            .unwrap();
    }
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    // Go back to messages pane
    o.handle_event(AppEvent::InputKey(KeyInput::new("esc", false)))
        .unwrap();

    // Select the last message (pending, id=0)
    let selected = o.state().open_chat().selected_message().unwrap();
    assert_eq!(selected.id, 0); // pending

    // dd should not dispatch delete for id=0
    o.handle_event(AppEvent::InputKey(KeyInput::new("d", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("d", false)))
        .unwrap();

    assert_eq!(o.dispatcher.delete_dispatch_count(), 0);
}
// ── o (open link) tests ──

#[test]
fn o_opens_first_url_from_message() {
    let mut o = orchestrator_with_open_chat(
        vec![chat(1, "Chat")],
        1,
        vec![message(10, "Check https://example.com out")],
    );

    o.handle_event(AppEvent::InputKey(KeyInput::new("o", false)))
        .unwrap();

    assert_eq!(o.opener.opened_urls(), vec!["https://example.com"]);
}

#[test]
fn o_does_nothing_when_no_url() {
    let mut o =
        orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "No links here")]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("o", false)))
        .unwrap();

    assert!(o.opener.opened_urls().is_empty());
}

#[test]
fn o_opens_first_url_when_multiple() {
    let mut o = orchestrator_with_open_chat(
        vec![chat(1, "Chat")],
        1,
        vec![message(
            10,
            "Visit https://first.com and https://second.com",
        )],
    );

    o.handle_event(AppEvent::InputKey(KeyInput::new("o", false)))
        .unwrap();

    assert_eq!(o.opener.opened_urls(), vec!["https://first.com"]);
}

#[test]
fn o_on_empty_chat_does_nothing() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("o", false)))
        .unwrap();

    assert!(o.opener.opened_urls().is_empty());
}
// ── reply-to-message tests ──

#[test]
fn r_key_sets_reply_context_and_switches_to_input() {
    let mut o = orchestrator_with_open_chat(
        vec![chat(1, "General")],
        1,
        vec![message(1, "Hello"), message(2, "World")],
    );

    // Select first message and press r
    o.handle_event(AppEvent::InputKey(KeyInput::new("k", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("r", false)))
        .unwrap();

    assert_eq!(o.state().active_pane(), ActivePane::MessageInput);
    let reply = o
        .state()
        .message_input()
        .reply_to()
        .expect("should have reply context");
    assert_eq!(reply.message_id, 1);
    assert_eq!(reply.text, "Hello");
}

#[test]
fn esc_from_input_clears_reply_context() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);

    // Set reply and switch to input
    o.handle_event(AppEvent::InputKey(KeyInput::new("r", false)))
        .unwrap();
    assert!(o.state().message_input().reply_to().is_some());

    // Press esc to go back
    o.handle_event(AppEvent::InputKey(KeyInput::new("esc", false)))
        .unwrap();

    assert_eq!(o.state().active_pane(), ActivePane::Messages);
    assert!(o.state().message_input().reply_to().is_none());
}

#[test]
fn send_message_with_reply_dispatches_reply_to_id() {
    let mut o = orchestrator_with_open_chat(
        vec![chat(1, "General")],
        1,
        vec![message(1, "Hello"), message(2, "World")],
    );

    // Reply to message 2 (selected by default — last message)
    o.handle_event(AppEvent::InputKey(KeyInput::new("r", false)))
        .unwrap();

    // Type text
    o.handle_event(AppEvent::InputKey(KeyInput::new("O", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("K", false)))
        .unwrap();

    // Send
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    assert_eq!(o.dispatcher.send_dispatch_count(), 1);
    let (chat_id, text, reply_to) = o.dispatcher.last_send().unwrap();
    assert_eq!(chat_id, 1);
    assert_eq!(text, "OK");
    assert_eq!(reply_to, Some(2));

    // Reply context should be consumed
    assert!(o.state().message_input().reply_to().is_none());
}

#[test]
fn send_message_without_reply_has_none_reply_to() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);

    // Enter input mode normally (no reply)
    o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("H", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    let (_, _, reply_to) = o.dispatcher.last_send().unwrap();
    assert_eq!(reply_to, None);
}

#[test]
fn r_key_does_nothing_when_no_message_selected() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("r", false)))
        .unwrap();

    // Should stay on Messages pane (not switch to input)
    assert_eq!(o.state().active_pane(), ActivePane::Messages);
    assert!(o.state().message_input().reply_to().is_none());
}

#[test]
fn r_key_ignores_pending_messages() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);

    // Add a pending message (id=0) and select it
    o.state_mut().open_chat_mut().add_pending_message(
        "Pending".to_owned(),
        crate::domain::message::MessageMedia::None,
        None,
    );

    // Try to reply
    o.handle_event(AppEvent::InputKey(KeyInput::new("r", false)))
        .unwrap();

    // Should not set reply context for pending message
    assert_eq!(o.state().active_pane(), ActivePane::Messages);
    assert!(o.state().message_input().reply_to().is_none());
}
