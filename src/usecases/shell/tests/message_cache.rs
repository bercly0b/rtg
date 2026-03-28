use super::*;

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

    // TDLib local cache has 1 message < threshold 5 → Loading
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
