use super::*;
use crate::domain::{
    forum_topic_list_state::ForumTopicListUiState, open_chat_state::OpenChatUiState,
    shell_state::ActivePane,
};

#[test]
fn opening_a_forum_chat_installs_loading_topic_list_and_dispatches_load() {
    let mut o = orchestrator_with_chats(vec![forum_chat(100, "Topics")]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    let forum_list = o
        .state()
        .forum_topic_list()
        .expect("forum topic list installed");
    assert_eq!(forum_list.parent_chat_id(), 100);
    assert_eq!(forum_list.parent_chat_title(), "Topics");
    assert_eq!(forum_list.ui_state(), ForumTopicListUiState::Loading);

    assert_eq!(o.dispatcher.forum_topics_dispatch_count(), 1);
    assert_eq!(o.dispatcher.last_forum_topics_chat_id(), Some(100));
    // No messages load — we're in the topic list, not the chat itself.
    assert_eq!(o.dispatcher.messages_dispatch_count(), 0);
    // Active pane stays ChatList (left panel renders topics).
    assert_eq!(o.state().active_pane(), ActivePane::ChatList);
}

#[test]
fn opening_a_forum_chat_does_not_clear_root_chat_list() {
    let chats = vec![forum_chat(100, "Topics"), chat(2, "Alice")];
    let mut o = orchestrator_with_chats(chats.clone());

    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    assert_eq!(o.state().chat_list().chats(), chats.as_slice());
    assert_eq!(
        o.state().chat_list().selected_chat().map(|c| c.chat_id),
        Some(100)
    );
}

#[test]
fn topic_list_navigation_works_in_forum_context() {
    let mut o = orchestrator_with_chats(vec![forum_chat(100, "Topics")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    inject_forum_topics(
        &mut o,
        100,
        vec![
            topic(100, 1, "Alpha", 1000),
            topic(100, 2, "Beta", 500),
            topic(100, 3, "Gamma", 100),
        ],
    );

    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    let list = o.state().forum_topic_list().unwrap();
    assert_eq!(list.selected_topic().map(|t| t.topic_id), Some(2));

    o.handle_event(AppEvent::InputKey(KeyInput::new("k", false)))
        .unwrap();
    let list = o.state().forum_topic_list().unwrap();
    assert_eq!(list.selected_topic().map(|t| t.topic_id), Some(1));
}

#[test]
fn opening_a_topic_switches_to_messages_pane_and_dispatches_topic_load() {
    let mut o = orchestrator_with_chats(vec![forum_chat(100, "Topics")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    inject_forum_topics(&mut o, 100, vec![topic(100, 7, "Backend", 1000)]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    assert_eq!(o.state().active_pane(), ActivePane::Messages);
    assert_eq!(o.state().open_chat().chat_id(), Some(100));
    assert_eq!(o.state().open_chat().topic_id(), Some(7));
    assert_eq!(o.dispatcher.last_load_messages(), Some((100, Some(7))));
}

#[test]
fn h_inside_topic_returns_to_topic_list_without_closing_parent() {
    let mut o = orchestrator_with_chats(vec![forum_chat(100, "Topics")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    inject_forum_topics(&mut o, 100, vec![topic(100, 7, "Backend", 1000)]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    inject_topic_messages(
        &mut o,
        100,
        Some(7),
        vec![message(1, "hello"), message(2, "world")],
    );
    let open_chats_before = o.dispatcher.open_chat_dispatch_count();
    let close_chats_before = o.dispatcher.close_chat_dispatch_count();

    o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
        .unwrap();

    assert_eq!(o.state().active_pane(), ActivePane::ChatList);
    // Forum list still active.
    assert!(o.state().forum_topic_list().is_some());
    // OpenChatState preserved so the topic's messages stay visible until the
    // user opens a different chat or topic — same UX as `h` on a regular chat.
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
    assert_eq!(o.state().open_chat().chat_id(), Some(100));
    assert_eq!(o.state().open_chat().topic_id(), Some(7));
    assert_eq!(o.state().open_chat().messages().len(), 2);
    // Parent forum chat is NOT closed.
    assert_eq!(o.dispatcher.open_chat_dispatch_count(), open_chats_before);
    assert_eq!(o.dispatcher.close_chat_dispatch_count(), close_chats_before);
}

#[test]
fn h_in_topic_list_leaves_forum_and_closes_parent() {
    let mut o = orchestrator_with_chats(vec![forum_chat(100, "Topics"), chat(2, "Alice")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    inject_forum_topics(&mut o, 100, vec![topic(100, 7, "Backend", 1000)]);
    let close_chats_before = o.dispatcher.close_chat_dispatch_count();

    o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
        .unwrap();

    assert!(o.state().forum_topic_list().is_none());
    assert_eq!(o.state().active_pane(), ActivePane::ChatList);
    // Parent forum chat closed in TDLib.
    assert_eq!(
        o.dispatcher.close_chat_dispatch_count(),
        close_chats_before + 1
    );
    // Root chat list preserved.
    assert_eq!(o.state().chat_list().chats().len(), 2);
    assert_eq!(
        o.state().chat_list().selected_chat().map(|c| c.chat_id),
        Some(100)
    );
}

#[test]
fn background_forum_topics_result_populates_list_when_open() {
    let mut o = orchestrator_with_chats(vec![forum_chat(100, "Topics")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    inject_forum_topics(
        &mut o,
        100,
        vec![topic(100, 1, "Alpha", 999), topic(100, 2, "Beta", 100)],
    );

    let list = o.state().forum_topic_list().unwrap();
    assert_eq!(list.ui_state(), ForumTopicListUiState::Ready);
    assert_eq!(list.topics().len(), 2);
    assert_eq!(list.topics()[0].topic_id, 1);
}

#[test]
fn background_forum_topics_result_dropped_for_stale_chat() {
    let mut o = orchestrator_with_chats(vec![forum_chat(100, "Topics")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    // Result for a different chat (e.g. user left the forum before result arrived).
    inject_forum_topics(&mut o, 999, vec![topic(999, 1, "Stale", 1)]);

    let list = o.state().forum_topic_list().unwrap();
    assert_eq!(list.ui_state(), ForumTopicListUiState::Loading);
    assert!(list.topics().is_empty());
}

#[test]
fn sending_a_text_message_inside_a_topic_carries_topic_id() {
    let mut o = orchestrator_with_chats(vec![forum_chat(100, "Topics")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    inject_forum_topics(&mut o, 100, vec![topic(100, 7, "Backend", 1000)]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    // Enter the message input and type a message.
    o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
        .unwrap();
    for c in "hi".chars() {
        o.handle_event(AppEvent::InputKey(KeyInput::new(c.to_string(), false)))
            .unwrap();
    }
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    assert_eq!(
        o.dispatcher.last_send_full(),
        Some((100, Some(7), "hi".to_owned(), None))
    );
}

#[test]
fn mark_as_read_in_a_topic_carries_topic_id() {
    let mut o = orchestrator_with_chats(vec![forum_chat(100, "Topics")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    inject_forum_topics(&mut o, 100, vec![topic(100, 7, "Backend", 1000)]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    // Background load returns messages, which triggers mark-as-read.
    inject_messages(&mut o, 100, vec![message(101, "first")]);

    let last = o
        .dispatcher
        .last_mark_as_read_full()
        .expect("mark dispatched");
    assert_eq!(last.0, 100);
    assert_eq!(last.1, Some(7));
    assert_eq!(last.2, vec![101]);
}

#[test]
fn forum_topic_update_for_open_forum_redispatches_topic_load() {
    use crate::domain::events::ChatUpdate;

    let mut o = orchestrator_with_chats(vec![forum_chat(100, "Topics")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    inject_forum_topics(&mut o, 100, vec![topic(100, 7, "Backend", 1000)]);
    let before = o.dispatcher.forum_topics_dispatch_count();

    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::ForumTopicChanged {
            chat_id: 100,
            topic_id: 7,
        }],
    })
    .unwrap();

    assert_eq!(o.dispatcher.forum_topics_dispatch_count(), before + 1);
}

#[test]
fn forum_topic_update_for_other_forum_does_not_redispatch() {
    use crate::domain::events::ChatUpdate;

    let mut o = orchestrator_with_chats(vec![forum_chat(100, "Topics")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    inject_forum_topics(&mut o, 100, vec![topic(100, 7, "Backend", 1000)]);
    let before = o.dispatcher.forum_topics_dispatch_count();

    // Update for a different chat.
    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::ForumTopicChanged {
            chat_id: 999,
            topic_id: 1,
        }],
    })
    .unwrap();

    assert_eq!(o.dispatcher.forum_topics_dispatch_count(), before);
}

#[test]
fn opening_non_forum_chat_uses_default_flow() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alice")]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    // Default flow: ActivePane::Messages, no forum_topic_list.
    assert_eq!(o.state().active_pane(), ActivePane::Messages);
    assert!(o.state().forum_topic_list().is_none());
    assert_eq!(o.state().open_chat().chat_id(), Some(1));
    assert!(o.state().open_chat().topic_id().is_none());
    assert_eq!(o.dispatcher.last_load_messages(), Some((1, None)));
}

// ── Race-condition regressions: stale background results from a topic the
// user already left must not bleed into the currently displayed topic. All
// three result variants share `chat_id` across all topics of the same forum,
// so the per-topic guard is the only thing keeping them isolated.

#[test]
fn stale_messages_from_previous_topic_are_discarded() {
    let mut o = orchestrator_with_chats(vec![forum_chat(100, "Topics")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    inject_forum_topics(
        &mut o,
        100,
        vec![topic(100, 1, "Alpha", 1000), topic(100, 2, "Beta", 500)],
    );

    // Open topic 1, then return to the topic list and open topic 2.
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    assert_eq!(o.state().open_chat().topic_id(), Some(1));
    o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    assert_eq!(o.state().open_chat().topic_id(), Some(2));

    // Topic 1's load result arrives late, after the user moved to topic 2.
    inject_topic_messages(
        &mut o,
        100,
        Some(1),
        vec![message(11, "stale from topic 1")],
    );

    // Topic 2 hasn't received its own data yet — it must stay Loading rather
    // than render topic 1's stale messages.
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);
}

#[test]
fn stale_older_messages_from_previous_topic_are_discarded() {
    use crate::domain::events::BackgroundTaskResult;

    let mut o = orchestrator_with_chats(vec![forum_chat(100, "Topics")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    inject_forum_topics(
        &mut o,
        100,
        vec![topic(100, 1, "Alpha", 1000), topic(100, 2, "Beta", 500)],
    );

    // Open topic 1, populate it, then move to topic 2 and populate it too.
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    inject_topic_messages(&mut o, 100, Some(1), vec![message(10, "old t1")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    inject_topic_messages(&mut o, 100, Some(2), vec![message(20, "current t2")]);
    let messages_before = o.state().open_chat().messages().to_vec();

    // Topic 1's late OlderMessagesLoaded result arrives.
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::OlderMessagesLoaded {
            chat_id: 100,
            topic_id: Some(1),
            result: Ok(vec![message(1, "stale older from topic 1")]),
        },
    ))
    .unwrap();

    // Topic 2's messages must be untouched.
    assert_eq!(o.state().open_chat().messages().to_vec(), messages_before);
}

#[test]
fn stale_post_send_refresh_from_previous_topic_is_discarded() {
    use crate::domain::events::BackgroundTaskResult;

    let mut o = orchestrator_with_chats(vec![forum_chat(100, "Topics")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    inject_forum_topics(
        &mut o,
        100,
        vec![topic(100, 1, "Alpha", 1000), topic(100, 2, "Beta", 500)],
    );

    // Open topic 1, populate it, then move to topic 2 and populate it.
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    inject_topic_messages(&mut o, 100, Some(1), vec![message(10, "old t1")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    inject_topic_messages(&mut o, 100, Some(2), vec![message(20, "current t2")]);
    let messages_before = o.state().open_chat().messages().to_vec();

    // Refresh-after-send for topic 1 arrives late.
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessageSentRefreshCompleted {
            chat_id: 100,
            topic_id: Some(1),
            result: Ok(vec![message(11, "stale refresh from topic 1")]),
        },
    ))
    .unwrap();

    // Topic 2 must keep its own messages.
    assert_eq!(o.state().open_chat().messages().to_vec(), messages_before);
}

#[test]
fn reload_topics_redispatches_load_from_error_state() {
    let mut o = orchestrator_with_chats(vec![forum_chat(100, "Topics")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    inject_forum_topics_error(&mut o, 100);

    let list = o.state().forum_topic_list().unwrap();
    assert_eq!(list.ui_state(), ForumTopicListUiState::Error);
    let before = o.dispatcher.forum_topics_dispatch_count();

    o.handle_event(AppEvent::InputKey(KeyInput::new("R", false)))
        .unwrap();

    let list = o.state().forum_topic_list().unwrap();
    assert_eq!(list.ui_state(), ForumTopicListUiState::Loading);
    assert_eq!(o.dispatcher.forum_topics_dispatch_count(), before + 1);
}

#[test]
fn chat_scoped_result_is_discarded_when_topic_is_open() {
    use crate::domain::events::BackgroundTaskResult;

    let mut o = orchestrator_with_chats(vec![forum_chat(100, "Topics")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    inject_forum_topics(&mut o, 100, vec![topic(100, 7, "Backend", 1000)]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    inject_topic_messages(&mut o, 100, Some(7), vec![message(70, "topic 7 only")]);
    let messages_before = o.state().open_chat().messages().to_vec();

    // A chat-scoped (topic_id=None) result arrives — could be from a previous
    // non-forum view of the same chat or a stray background path. The handler
    // must reject it.
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessagesLoaded {
            chat_id: 100,
            topic_id: None,
            result: Ok(vec![message(99, "chat-wide stale")]),
        },
    ))
    .unwrap();

    assert_eq!(o.state().open_chat().messages().to_vec(), messages_before);
}

#[test]
fn pressing_i_in_closed_topic_does_not_enter_input_and_notifies() {
    let mut o = orchestrator_with_chats(vec![forum_chat(100, "Topics")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    let mut closed = topic(100, 7, "Backend", 1000);
    closed.is_closed = true;
    inject_forum_topics(&mut o, 100, vec![closed]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    assert_eq!(o.state().active_pane(), ActivePane::Messages);

    o.handle_event(AppEvent::InputKey(KeyInput::new("i", false)))
        .unwrap();

    // Input pane was not entered; notification surfaces the reason.
    assert_eq!(o.state().active_pane(), ActivePane::Messages);
    assert_eq!(o.state().active_notification(), Some("Topic is closed"));
}
