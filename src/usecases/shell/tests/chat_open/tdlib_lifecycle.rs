use super::*;

#[test]
fn open_chat_dispatches_tdlib_open_chat() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General")]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    assert_eq!(o.dispatcher.open_chat_dispatch_count(), 1);
    assert_eq!(o.tdlib_opened_chat_id, Some(1));
}

#[test]
fn navigate_away_from_chat_dispatches_tdlib_close_chat() {
    let o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);

    assert_eq!(o.tdlib_opened_chat_id, Some(1));

    let mut o = o;
    o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
        .unwrap();

    assert_eq!(o.dispatcher.close_chat_dispatch_count(), 1);
    assert_eq!(o.tdlib_opened_chat_id, None);
}

#[test]
fn esc_from_messages_dispatches_tdlib_close_chat() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("esc", false)))
        .unwrap();

    assert_eq!(o.dispatcher.close_chat_dispatch_count(), 1);
    assert_eq!(o.tdlib_opened_chat_id, None);
}

#[test]
fn switching_chats_closes_previous_and_opens_new() {
    let mut o = orchestrator_with_chats(vec![chat(1, "A"), chat(2, "B")]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    assert_eq!(o.dispatcher.open_chat_dispatch_count(), 1);
    assert_eq!(o.tdlib_opened_chat_id, Some(1));

    o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
        .unwrap();
    assert_eq!(o.dispatcher.close_chat_dispatch_count(), 1);

    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    assert_eq!(o.dispatcher.open_chat_dispatch_count(), 2);
    assert_eq!(o.tdlib_opened_chat_id, Some(2));
}

#[test]
fn messages_loaded_dispatches_mark_as_read() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General")]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessagesLoaded {
            chat_id: 1,
            result: Ok(vec![message(10, "A"), message(20, "B"), message(30, "C")]),
        },
    ))
    .unwrap();

    assert_eq!(o.dispatcher.mark_as_read_dispatch_count(), 1);
    let (mark_chat_id, mark_ids) = o.dispatcher.last_mark_as_read().unwrap();
    assert_eq!(mark_chat_id, 1);
    assert_eq!(mark_ids, vec![10, 20, 30]);
}

#[test]
fn messages_loaded_does_not_mark_as_read_when_empty() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General")]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessagesLoaded {
            chat_id: 1,
            result: Ok(vec![]),
        },
    ))
    .unwrap();

    assert_eq!(o.dispatcher.mark_as_read_dispatch_count(), 0);
}

#[test]
fn message_sent_refresh_dispatches_mark_as_read() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);

    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessageSentRefreshCompleted {
            chat_id: 1,
            result: Ok(vec![message(1, "Hello"), message(2, "My reply")]),
        },
    ))
    .unwrap();

    assert_eq!(o.dispatcher.mark_as_read_dispatch_count(), 2);
    let (mark_chat_id, mark_ids) = o.dispatcher.last_mark_as_read().unwrap();
    assert_eq!(mark_chat_id, 1);
    assert_eq!(mark_ids, vec![1, 2]);
}

#[test]
fn reopen_same_ready_chat_does_not_dispatch_open_chat_again() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("l", false)))
        .unwrap();

    assert_eq!(o.dispatcher.open_chat_dispatch_count(), 2);
}

#[test]
fn quit_while_chat_open_dispatches_close_chat() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
    assert_eq!(o.tdlib_opened_chat_id, Some(1));

    o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("l", false)))
        .unwrap();
    assert_eq!(o.tdlib_opened_chat_id, Some(1));

    o.handle_event(AppEvent::QuitRequested).unwrap();

    assert!(!o.state().is_running());
    assert_eq!(o.tdlib_opened_chat_id, None);
    assert_eq!(o.dispatcher.close_chat_dispatch_count(), 2);
}

#[test]
fn stale_messages_do_not_dispatch_mark_as_read() {
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
            result: Ok(vec![message(10, "Stale")]),
        },
    ))
    .unwrap();

    assert_eq!(o.dispatcher.mark_as_read_dispatch_count(), 0);
}

#[test]
fn messages_loaded_does_not_mark_as_read_when_focus_on_chat_list() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General")]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    // Navigate back to chat list while messages are still loading.
    // Note: pressing h closes TDLib chat but does NOT clear open_chat state,
    // so the MessagesLoaded result still passes the stale-chat check and
    // reaches the focus guard.
    o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
        .unwrap();
    assert_eq!(o.state().active_pane(), ActivePane::ChatList);

    // Messages arrive while focus is on ChatList — should NOT mark as read
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessagesLoaded {
            chat_id: 1,
            result: Ok(vec![message(10, "A"), message(20, "B")]),
        },
    ))
    .unwrap();

    assert_eq!(o.dispatcher.mark_as_read_dispatch_count(), 0);
}

#[test]
fn messages_loaded_marks_as_read_when_focus_on_messages() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General")]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    assert_eq!(o.state().active_pane(), ActivePane::Messages);

    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessagesLoaded {
            chat_id: 1,
            result: Ok(vec![message(10, "A"), message(20, "B")]),
        },
    ))
    .unwrap();

    assert_eq!(o.dispatcher.mark_as_read_dispatch_count(), 1);
}

#[test]
fn messages_loaded_marks_as_read_when_focus_on_message_input() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);

    // Switch to message input
    o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
        .unwrap();
    assert_eq!(o.state().active_pane(), ActivePane::MessageInput);

    let prev_count = o.dispatcher.mark_as_read_dispatch_count();

    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessagesLoaded {
            chat_id: 1,
            result: Ok(vec![message(1, "Hello"), message(2, "New")]),
        },
    ))
    .unwrap();

    assert_eq!(o.dispatcher.mark_as_read_dispatch_count(), prev_count + 1);
}

#[test]
fn reopen_chat_marks_as_read_after_returning_focus() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);

    // Navigate away (h closes TDLib chat)
    o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
        .unwrap();
    assert_eq!(o.state().active_pane(), ActivePane::ChatList);

    let prev_count = o.dispatcher.mark_as_read_dispatch_count();

    // Reopen: enter sets focus to Messages first, then mark_open_chat_messages_as_read
    // fires inside the TDLib-reopen branch of open_selected_chat
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    assert_eq!(o.dispatcher.mark_as_read_dispatch_count(), prev_count + 1);
}

#[test]
fn prefetched_messages_do_not_mark_as_read_when_focus_on_chat_list() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General")]);

    // Open the chat (sets focus to Messages, dispatches load_messages)
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    // Navigate back to chat list while chat is still Loading
    o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
        .unwrap();
    assert_eq!(o.state().active_pane(), ActivePane::ChatList);

    // Prefetched messages arrive while focus is on ChatList — should NOT mark as read
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessagesPrefetched {
            chat_id: 1,
            result: Ok(vec![message(10, "A"), message(20, "B")]),
        },
    ))
    .unwrap();

    assert_eq!(o.dispatcher.mark_as_read_dispatch_count(), 0);
}
