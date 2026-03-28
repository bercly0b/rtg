use super::super::*;

// ── Phase 5: UX polish tests ──

#[test]
fn cache_below_threshold_stays_in_loading() {
    let mut o = make_orchestrator_with_threshold(vec![chat(1, "Alice")], 5);

    // Pre-populate cache with fewer messages than threshold
    o.state
        .message_cache_mut()
        .put(1, vec![message(1, "single msg")], true);

    // Open the chat
    inject_chat_list(&mut o, vec![chat(1, "Alice")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    // Should remain in Loading because cache has 1 < 5 messages
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);
}

#[test]
fn cache_at_threshold_shows_ready() {
    let mut o = make_orchestrator_with_threshold(vec![chat(1, "Alice")], 3);

    // Pre-populate cache with exactly threshold messages
    o.state.message_cache_mut().put(
        1,
        vec![message(1, "A"), message(2, "B"), message(3, "C")],
        true,
    );

    inject_chat_list(&mut o, vec![chat(1, "Alice")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
    assert_eq!(o.state().open_chat().messages().len(), 3);
}

#[test]
fn cache_hit_sets_refreshing_and_cached_source() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alice")]);

    // Pre-populate cache
    o.state
        .message_cache_mut()
        .put(1, vec![message(1, "A"), message(2, "B")], true);

    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
    assert!(o.state().open_chat().is_refreshing());
    assert_eq!(o.state().open_chat().message_source(), MessageSource::Cache);
}

#[test]
fn background_load_clears_refreshing_and_sets_live_source() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alice")]);

    // Pre-populate cache for instant display
    o.state
        .message_cache_mut()
        .put(1, vec![message(1, "cached")], true);

    // Open chat — sets Ready + refreshing + Cache source
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    assert!(o.state().open_chat().is_refreshing());

    // Background load completes
    inject_messages(
        &mut o,
        1,
        vec![message(1, "fresh A"), message(2, "fresh B")],
    );

    assert!(!o.state().open_chat().is_refreshing());
    assert_eq!(o.state().open_chat().message_source(), MessageSource::Live);
    assert_eq!(o.state().open_chat().messages().len(), 2);
}

#[test]
fn loading_state_has_no_refreshing_or_source() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alice")]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);
    assert!(!o.state().open_chat().is_refreshing());
    assert_eq!(o.state().open_chat().message_source(), MessageSource::None);
}

#[test]
fn tdlib_local_cache_below_threshold_stays_in_loading() {
    let cache = StubCacheSource::with_messages(vec![(1, vec![message(1, "sparse")])]);
    let mut o = make_orchestrator_with_cache_and_threshold(vec![chat(1, "Alice")], cache, 5);

    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    // TDLib local cache has 1 message < threshold 5 -> Loading
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);
}

#[test]
fn tdlib_local_cache_at_threshold_shows_ready_with_cache_source() {
    let msgs: Vec<Message> = (1..=5).map(|i| message(i, &format!("msg {i}"))).collect();
    let cache = StubCacheSource::with_messages(vec![(1, msgs)]);
    let mut o = make_orchestrator_with_cache_and_threshold(vec![chat(1, "Alice")], cache, 5);

    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
    assert_eq!(o.state().open_chat().messages().len(), 5);
    assert!(o.state().open_chat().is_refreshing());
    assert_eq!(o.state().open_chat().message_source(), MessageSource::Cache);
}

#[test]
fn message_sent_refresh_clears_refreshing_and_sets_live() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "Alice")], 1, vec![message(1, "Hello")]);

    // Simulate: cache hit sets refreshing
    o.state.open_chat_mut().set_refreshing(true);
    o.state
        .open_chat_mut()
        .set_message_source(MessageSource::Cache);

    // Message sent refresh arrives
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessageSentRefreshCompleted {
            chat_id: 1,
            result: Ok(vec![message(1, "Hello"), message(2, "New msg")]),
        },
    ))
    .unwrap();

    assert!(!o.state().open_chat().is_refreshing());
    assert_eq!(o.state().open_chat().message_source(), MessageSource::Live);
}

#[test]
fn threshold_zero_is_clamped_to_one() {
    let mut o = make_orchestrator_with_threshold(vec![chat(1, "Alice")], 0);

    o.state
        .message_cache_mut()
        .put(1, vec![message(1, "single")], true);

    inject_chat_list(&mut o, vec![chat(1, "Alice")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    // With threshold clamped to 1, a single message is sufficient
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
}

#[test]
fn background_load_error_clears_refreshing() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alice")]);

    // Pre-populate cache for instant display
    o.state
        .message_cache_mut()
        .put(1, vec![message(1, "cached")], true);

    // Open chat — sets Ready + refreshing
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    assert!(o.state().open_chat().is_refreshing());

    // Background load fails
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessagesLoaded {
            chat_id: 1,
            result: Err(BackgroundError::new("MESSAGES_UNAVAILABLE")),
        },
    ))
    .unwrap();

    // Error state should have refreshing cleared
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Error);
    assert!(!o.state().open_chat().is_refreshing());
}

#[test]
fn prefetch_below_threshold_does_not_populate_open_chat() {
    let mut o = make_orchestrator_with_threshold(vec![chat(1, "Alpha"), chat(2, "Beta")], 5);

    inject_chat_list(&mut o, vec![chat(1, "Alpha"), chat(2, "Beta")]);

    // Navigate to chat 2 (triggers prefetch)
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();

    // Open chat 2
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);

    // Prefetch result with too few messages
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessagesPrefetched {
            chat_id: 2,
            result: Ok(vec![message(10, "sparse")]),
        },
    ))
    .unwrap();

    // Should still be Loading (1 < 5 threshold)
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);
    // But cache should have data
    assert!(o.state().message_cache().has_messages(2));
}
