use super::super::*;

// ── Prefetch on j/k navigation tests (Phase 3) ──

#[test]
fn jk_navigation_dispatches_prefetch_for_uncached_chat() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alpha"), chat(2, "Beta"), chat(3, "Gamma")]);

    // Navigate down to chat 2 (no cache)
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();

    assert_eq!(o.dispatcher.prefetch_dispatch_count(), 1);
    assert_eq!(o.dispatcher.last_prefetch_chat_id(), Some(2));
    assert_eq!(o.prefetch_in_flight, Some(2));
}

#[test]
fn jk_navigation_skips_prefetch_when_cache_hit() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alpha"), chat(2, "Beta")]);

    // Populate cache for chat 2
    o.state
        .message_cache_mut()
        .put(2, vec![message(10, "cached")], true);

    // Navigate down to chat 2
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();

    assert_eq!(
        o.dispatcher.prefetch_dispatch_count(),
        0,
        "should not prefetch when cache already has data"
    );
}

#[test]
fn jk_rapid_navigation_debounces_prefetch() {
    let mut o = orchestrator_with_chats(vec![
        chat(1, "Alpha"),
        chat(2, "Beta"),
        chat(3, "Gamma"),
        chat(4, "Delta"),
    ]);

    // First j dispatches prefetch for chat 2
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    assert_eq!(o.dispatcher.prefetch_dispatch_count(), 1);
    assert_eq!(o.dispatcher.last_prefetch_chat_id(), Some(2));

    // Second j should NOT dispatch (prefetch for chat 2 still in-flight)
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    assert_eq!(
        o.dispatcher.prefetch_dispatch_count(),
        1,
        "second j should be debounced by in-flight guard"
    );
}

#[test]
fn prefetch_result_populates_cache_only() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alpha"), chat(2, "Beta")]);

    // Navigate down (triggers prefetch for chat 2)
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    assert_eq!(o.prefetch_in_flight, Some(2));

    // Prefetch result arrives
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessagesPrefetched {
            chat_id: 2,
            result: Ok(vec![
                message(10, "Prefetched A"),
                message(11, "Prefetched B"),
            ]),
        },
    ))
    .unwrap();

    // Cache should have the data
    assert!(o.state().message_cache().has_messages(2));
    assert_eq!(o.state.message_cache_mut().get(2).unwrap().len(), 2);

    // OpenChatState should NOT be affected (no chat is open)
    assert!(!o.state().open_chat().is_open());

    // In-flight guard should be cleared
    assert_eq!(o.prefetch_in_flight, None);
}

#[test]
fn prefetch_result_updates_open_chat_if_loading() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alpha"), chat(2, "Beta")]);

    // Navigate down (triggers prefetch for chat 2)
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();

    // User opens chat 2 while prefetch is in-flight
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    assert_eq!(o.state().open_chat().chat_id(), Some(2));
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);

    // Prefetch result arrives
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessagesPrefetched {
            chat_id: 2,
            result: Ok(vec![message(10, "Prefetched")]),
        },
    ))
    .unwrap();

    // OpenChatState should be updated from cache
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
    assert_eq!(o.state().open_chat().messages().len(), 1);
    assert_eq!(o.state().open_chat().messages()[0].text, "Prefetched");
}

#[test]
fn prefetch_error_clears_in_flight_without_cache() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alpha"), chat(2, "Beta")]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    assert_eq!(o.prefetch_in_flight, Some(2));

    // Prefetch fails
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessagesPrefetched {
            chat_id: 2,
            result: Err(BackgroundError::new("MESSAGES_UNAVAILABLE")),
        },
    ))
    .unwrap();

    assert_eq!(o.prefetch_in_flight, None);
    assert!(!o.state().message_cache().has_messages(2));
}

#[test]
fn open_selected_chat_clears_prefetch_in_flight() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alpha"), chat(2, "Beta")]);

    // Trigger prefetch
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    assert_eq!(o.prefetch_in_flight, Some(2));

    // Open the chat
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    assert_eq!(
        o.prefetch_in_flight, None,
        "opening a chat should clear prefetch guard"
    );
}

#[test]
fn prefetch_then_open_is_instant_from_cache() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alpha"), chat(2, "Beta"), chat(3, "Gamma")]);

    // Navigate to chat 2 (triggers prefetch)
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();

    // Prefetch completes
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessagesPrefetched {
            chat_id: 2,
            result: Ok(vec![message(10, "Prefetched msg")]),
        },
    ))
    .unwrap();

    // Open chat 2 — should be instant from cache
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    assert_eq!(o.state().open_chat().chat_id(), Some(2));
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
    assert_eq!(o.state().open_chat().messages()[0].text, "Prefetched msg");

    // Background refresh still dispatched
    assert!(o.dispatcher.messages_dispatch_count() > 0);
}

#[test]
fn k_navigation_also_triggers_prefetch() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alpha"), chat(2, "Beta"), chat(3, "Gamma")]);

    // Navigate to the bottom first
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();

    // Clear the in-flight by injecting the result
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessagesPrefetched {
            chat_id: 2,
            result: Ok(vec![message(10, "Beta msg")]),
        },
    ))
    .unwrap();

    // Navigate up with k to chat 2 (already cached)
    o.handle_event(AppEvent::InputKey(KeyInput::new("k", false)))
        .unwrap();
    let prefetch_count_after_k_to_cached = o.dispatcher.prefetch_dispatch_count();

    // Chat 2 is cached, so no new prefetch
    assert_eq!(prefetch_count_after_k_to_cached, 1);

    // Navigate further up to chat 1 (not cached)
    // But prefetch for chat 3 might still be in-flight...
    // Actually chat 3 prefetch was never dispatched because chat 2 prefetch was in-flight.
    // So after receiving the result above, prefetch_in_flight is None.
    o.handle_event(AppEvent::InputKey(KeyInput::new("k", false)))
        .unwrap();
    assert_eq!(o.dispatcher.prefetch_dispatch_count(), 2);
    assert_eq!(o.dispatcher.last_prefetch_chat_id(), Some(1));
}

#[test]
fn prefetch_guard_allows_new_dispatch_after_result() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alpha"), chat(2, "Beta"), chat(3, "Gamma")]);

    // Navigate to chat 2 (prefetch dispatched)
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    assert_eq!(o.dispatcher.prefetch_dispatch_count(), 1);

    // Prefetch completes
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessagesPrefetched {
            chat_id: 2,
            result: Ok(vec![message(10, "msg")]),
        },
    ))
    .unwrap();

    // Navigate to chat 3 (should dispatch new prefetch)
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    assert_eq!(o.dispatcher.prefetch_dispatch_count(), 2);
    assert_eq!(o.dispatcher.last_prefetch_chat_id(), Some(3));
}

#[test]
fn prefetch_result_for_different_chat_does_not_affect_open_chat() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alpha"), chat(2, "Beta"), chat(3, "Gamma")]);

    // Prefetch dispatched for chat 2
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();

    // User opens chat 3 instead
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    assert_eq!(o.state().open_chat().chat_id(), Some(3));
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);

    // Stale prefetch for chat 2 arrives
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessagesPrefetched {
            chat_id: 2,
            result: Ok(vec![message(10, "Prefetched for 2")]),
        },
    ))
    .unwrap();

    // Chat 3 should still be Loading
    assert_eq!(o.state().open_chat().chat_id(), Some(3));
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);
    // Chat 2 should be in cache
    assert!(o.state().message_cache().has_messages(2));
}

#[test]
fn prefetch_empty_result_does_not_populate_cache() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alpha"), chat(2, "Beta")]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();

    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessagesPrefetched {
            chat_id: 2,
            result: Ok(vec![]),
        },
    ))
    .unwrap();

    assert!(!o.state().message_cache().has_messages(2));
}
