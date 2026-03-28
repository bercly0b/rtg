use super::super::*;

// ── Message cache tests ──

#[test]
fn messages_stored_in_cache_after_background_load() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General")]);

    // Open chat 1
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    assert!(!o.state().message_cache().has_messages(1));

    // Background load completes
    inject_messages(&mut o, 1, vec![message(1, "Hello"), message(2, "World")]);

    assert!(o.state().message_cache().has_messages(1));
    assert_eq!(o.state.message_cache_mut().get(1).unwrap().len(), 2);
}

#[test]
fn stale_messages_result_still_cached() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alice"), chat(2, "Bob")]);

    // Open chat 1
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    // Navigate back to chat list before messages arrive
    o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
        .unwrap();

    // Messages arrive for chat 1 (now "stale" since user navigated away)
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessagesLoaded {
            chat_id: 1,
            result: Ok(vec![message(10, "cached even though stale")]),
        },
    ))
    .unwrap();

    // Messages should still be in cache despite the stale discard
    assert!(o.state().message_cache().has_messages(1));
    assert_eq!(
        o.state.message_cache_mut().get(1).unwrap()[0].text,
        "cached even though stale"
    );
}

#[test]
fn cache_hit_on_reopen_shows_messages_instantly() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alice"), chat(2, "Bob")]);

    // Open chat 1 and load messages
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    inject_messages(&mut o, 1, vec![message(1, "Hello"), message(2, "World")]);
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);

    // Navigate back to chat list
    o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
        .unwrap();

    // Move to chat 2 and open it (to make the orchestrator forget chat 1's OpenChatState)
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    inject_messages(&mut o, 2, vec![message(10, "Bob's message")]);

    // Navigate back, move to chat 1
    o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("k", false)))
        .unwrap();

    let msgs_dispatched_before = o.dispatcher.messages_dispatch_count();

    // Re-open chat 1 — should show cached messages instantly
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    // State should be Ready immediately (from cache), not Loading
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
    assert_eq!(o.state().open_chat().messages().len(), 2);
    assert_eq!(o.state().open_chat().messages()[0].text, "Hello");

    // Background refresh should still be dispatched
    assert_eq!(
        o.dispatcher.messages_dispatch_count(),
        msgs_dispatched_before + 1,
        "background refresh should be dispatched even on cache hit"
    );
}

#[test]
fn cache_miss_falls_through_to_tdlib_local_cache() {
    let cache = StubCacheSource::with_messages(vec![(1, vec![message(1, "from tdlib local")])]);
    let mut o = make_orchestrator_with_cache(vec![chat(1, "Alice")], cache);

    // Trigger initial refresh since we use with_initial_chat_list
    o.handle_event(AppEvent::Tick).unwrap();
    inject_chat_list(&mut o, vec![chat(1, "Alice")]);

    // Open chat 1 — app cache is empty, but TDLib local cache has data
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    // Should be Ready from TDLib local cache (StubCacheSource)
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
    assert_eq!(o.state().open_chat().messages()[0].text, "from tdlib local");
}

#[test]
fn cache_updated_on_message_sent_refresh() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "Alice")], 1, vec![message(1, "Hello")]);

    // Simulate sending a message and getting refresh result
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessageSentRefreshCompleted {
            chat_id: 1,
            result: Ok(vec![message(1, "Hello"), message(2, "My new message")]),
        },
    ))
    .unwrap();

    // Cache should contain the updated messages
    assert!(o.state().message_cache().has_messages(1));
    let cached = o.state.message_cache_mut().get(1).unwrap();
    assert_eq!(cached.len(), 2);
    assert_eq!(cached[1].text, "My new message");
}

#[test]
fn cache_not_populated_on_load_error() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General")]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessagesLoaded {
            chat_id: 1,
            result: Err(BackgroundError::new("NETWORK_ERROR")),
        },
    ))
    .unwrap();

    assert!(!o.state().message_cache().has_messages(1));
}
