use super::*;

#[test]
fn scroll_up_near_top_dispatches_older_messages() {
    let chats = vec![chat(1, "Chat")];
    let messages: Vec<Message> = (1..=20).map(|i| message(i, &format!("msg {i}"))).collect();
    let mut o = orchestrator_with_open_chat(chats, 1, messages);

    // Navigate up until near top (SCROLL_MARGIN = 5)
    for _ in 0..16 {
        o.handle_event(AppEvent::InputKey(KeyInput::new("k", false)))
            .unwrap();
    }

    assert_eq!(o.dispatcher.older_messages_dispatch_count(), 1);
    assert_eq!(o.dispatcher.last_older_messages(), Some((1, 1)));
}

#[test]
fn scroll_up_does_not_dispatch_when_all_loaded() {
    let chats = vec![chat(1, "Chat")];
    let messages: Vec<Message> = (1..=20).map(|i| message(i, &format!("msg {i}"))).collect();
    let mut o = orchestrator_with_open_chat(chats, 1, messages);

    o.state_mut().open_chat_mut().set_all_messages_loaded(true);

    for _ in 0..19 {
        o.handle_event(AppEvent::InputKey(KeyInput::new("k", false)))
            .unwrap();
    }

    assert_eq!(o.dispatcher.older_messages_dispatch_count(), 0);
}

#[test]
fn scroll_up_does_not_dispatch_duplicate_while_in_flight() {
    let chats = vec![chat(1, "Chat")];
    let messages: Vec<Message> = (1..=20).map(|i| message(i, &format!("msg {i}"))).collect();
    let mut o = orchestrator_with_open_chat(chats, 1, messages);

    // Navigate to top — first trigger
    for _ in 0..19 {
        o.handle_event(AppEvent::InputKey(KeyInput::new("k", false)))
            .unwrap();
    }

    // Should only dispatch once despite multiple k presses at top
    assert_eq!(o.dispatcher.older_messages_dispatch_count(), 1);
}

#[test]
fn older_messages_loaded_prepends_to_open_chat() {
    let chats = vec![chat(1, "Chat")];
    let messages = vec![message(10, "ten"), message(20, "twenty")];
    let mut o = orchestrator_with_open_chat(chats, 1, messages);

    let older = vec![message(1, "one"), message(5, "five")];
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::OlderMessagesLoaded {
            chat_id: 1,
            result: Ok(older),
        },
    ))
    .unwrap();

    let msgs = o.state().open_chat().messages();
    assert_eq!(msgs.len(), 4);
    assert_eq!(msgs[0].id, 1);
    assert_eq!(msgs[1].id, 5);
    assert_eq!(msgs[2].id, 10);
    assert_eq!(msgs[3].id, 20);
}

#[test]
fn older_messages_empty_result_sets_all_loaded() {
    let chats = vec![chat(1, "Chat")];
    let messages = vec![message(10, "ten")];
    let mut o = orchestrator_with_open_chat(chats, 1, messages);

    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::OlderMessagesLoaded {
            chat_id: 1,
            result: Ok(vec![]),
        },
    ))
    .unwrap();

    assert!(o.state().open_chat().all_messages_loaded());
}

#[test]
fn older_messages_for_wrong_chat_is_discarded() {
    let chats = vec![chat(1, "Chat")];
    let messages = vec![message(10, "ten")];
    let mut o = orchestrator_with_open_chat(chats, 1, messages);

    let older = vec![message(1, "one")];
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::OlderMessagesLoaded {
            chat_id: 999,
            result: Ok(older),
        },
    ))
    .unwrap();

    assert_eq!(o.state().open_chat().messages().len(), 1);
}

#[test]
fn older_messages_error_does_not_crash() {
    let chats = vec![chat(1, "Chat")];
    let messages = vec![message(10, "ten")];
    let mut o = orchestrator_with_open_chat(chats, 1, messages);

    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::OlderMessagesLoaded {
            chat_id: 1,
            result: Err(BackgroundError::new("TEST_ERROR")),
        },
    ))
    .unwrap();

    assert_eq!(o.state().open_chat().messages().len(), 1);
}

#[test]
fn can_dispatch_again_after_older_messages_complete() {
    let chats = vec![chat(1, "Chat")];
    let messages: Vec<Message> = (1..=20).map(|i| message(i, &format!("msg {i}"))).collect();
    let mut o = orchestrator_with_open_chat(chats, 1, messages);

    // Navigate to top — triggers first dispatch
    for _ in 0..19 {
        o.handle_event(AppEvent::InputKey(KeyInput::new("k", false)))
            .unwrap();
    }
    assert_eq!(o.dispatcher.older_messages_dispatch_count(), 1);

    // Complete the request with only 2 older messages — keeps selected_index near top
    let older = vec![message(-2, "old2"), message(-1, "old1")];
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::OlderMessagesLoaded {
            chat_id: 1,
            result: Ok(older),
        },
    ))
    .unwrap();

    // selected_index shifted by 2, now at index 2 — still within SCROLL_MARGIN (5)
    // Pressing k should trigger another dispatch
    o.handle_event(AppEvent::InputKey(KeyInput::new("k", false)))
        .unwrap();

    assert_eq!(o.dispatcher.older_messages_dispatch_count(), 2);
}
