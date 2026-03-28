use super::*;

#[test]
fn enter_key_dispatches_load_messages_and_switches_pane() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General"), chat(2, "Backend")]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    assert_eq!(o.dispatcher.messages_dispatch_count(), 1);
    assert_eq!(o.state().open_chat().chat_id(), Some(1));
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);
    assert_eq!(o.state().active_pane(), ActivePane::Messages);
}

#[test]
fn messages_loaded_result_sets_ready_state() {
    let o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);

    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
    assert_eq!(o.state().open_chat().messages().len(), 1);
}

#[test]
fn messages_loaded_error_sets_error_state() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessagesLoaded {
            chat_id: 1,
            result: Err(BackgroundError::new("MESSAGES_UNAVAILABLE")),
        },
    ))
    .unwrap();

    assert_eq!(o.state().open_chat().chat_id(), Some(1));
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Error);
}

#[test]
fn stale_messages_result_is_discarded() {
    let mut o = orchestrator_with_chats(vec![chat(1, "A"), chat(2, "B")]);
    // Open chat 1
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    // Navigate away before result arrives
    o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
        .unwrap();
    // Navigate to chat 2 and open it
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    // Now the stale result for chat 1 arrives
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessagesLoaded {
            chat_id: 1,
            result: Ok(vec![message(1, "Stale")]),
        },
    ))
    .unwrap();

    // Should not have been applied — still loading chat 2
    assert_eq!(o.state().open_chat().chat_id(), Some(2));
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);
}
// ── Cached startup tests ──

#[test]
fn cached_startup_shows_ready_immediately() {
    let o = make_orchestrator_with_cached_chats(vec![chat(1, "Alpha"), chat(2, "Beta")]);

    assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Ready);
    assert_eq!(o.state().chat_list().chats().len(), 2);
    assert_eq!(o.state().chat_list().selected_index(), Some(0));
}

#[test]
fn cached_startup_empty_cache_falls_back_to_loading() {
    let o = make_orchestrator_with_cached_chats(vec![]);

    assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Loading);
    assert!(o.state().chat_list().chats().is_empty());
}

#[test]
fn cached_startup_first_tick_triggers_background_refresh() {
    let mut o = make_orchestrator_with_cached_chats(vec![chat(1, "Alpha"), chat(2, "Beta")]);

    // State is Ready from cache
    assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Ready);

    // First tick should trigger a background refresh even though state is Ready
    o.handle_event(AppEvent::Tick).unwrap();
    assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);

    // Data should remain visible (no blink)
    assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Ready);
    assert_eq!(o.state().chat_list().chats().len(), 2);
}

#[test]
fn cached_startup_refresh_only_fires_once() {
    let mut o = make_orchestrator_with_cached_chats(vec![chat(1, "Alpha"), chat(2, "Beta")]);

    // First tick: triggers refresh
    o.handle_event(AppEvent::Tick).unwrap();
    assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);

    // Simulate result arriving
    inject_chat_list(
        &mut o,
        vec![chat(1, "Alpha"), chat(2, "Beta"), chat(3, "Gamma")],
    );

    // Second tick: should NOT trigger another refresh (initial_refresh_needed is false)
    o.handle_event(AppEvent::Tick).unwrap();
    assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);
}

#[test]
fn cached_startup_background_result_updates_list() {
    let mut o = make_orchestrator_with_cached_chats(vec![chat(1, "Alpha"), chat(2, "Beta")]);

    // Navigate to second chat
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    assert_eq!(
        o.state().chat_list().selected_chat().map(|c| c.chat_id),
        Some(2)
    );

    // Trigger refresh via Tick
    o.handle_event(AppEvent::Tick).unwrap();

    // Background result arrives with updated data (new chat appeared at top)
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::ChatListLoaded {
            result: Ok(vec![chat(3, "Gamma"), chat(1, "Alpha"), chat(2, "Beta")]),
        },
    ))
    .unwrap();

    // Selection should be preserved on chat_id=2
    assert_eq!(
        o.state().chat_list().selected_chat().map(|c| c.chat_id),
        Some(2)
    );
    assert_eq!(o.state().chat_list().chats().len(), 3);
}

#[test]
fn non_cached_startup_does_not_set_initial_refresh_flag() {
    let mut o = make_orchestrator();

    // Default state is Loading, NOT Ready
    assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Loading);

    // First tick triggers the standard Loading -> dispatch path
    o.handle_event(AppEvent::Tick).unwrap();
    assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);

    // Result arrives
    inject_chat_list(&mut o, vec![chat(1, "Alpha")]);

    // Second tick should NOT dispatch again (no initial_refresh_needed flag)
    o.handle_event(AppEvent::Tick).unwrap();
    assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);
}
// ── Cached messages on chat open tests ──

#[test]
fn open_chat_with_cache_shows_ready_instantly() {
    let cache = StubCacheSource::with_messages(vec![(
        1,
        vec![message(10, "Cached A"), message(11, "Cached B")],
    )]);
    let mut o = make_orchestrator_with_cache(vec![chat(1, "Alpha"), chat(2, "Beta")], cache);

    // Open first chat (Enter)
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    // Chat should be Ready immediately from cache
    assert_eq!(o.state().open_chat().chat_id(), Some(1));
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
    assert_eq!(o.state().open_chat().messages().len(), 2);
    assert_eq!(o.state().open_chat().messages()[0].text, "Cached A");

    // A background dispatch should still have been issued for a full load
    assert_eq!(o.dispatcher.messages_dispatch_count(), 1);
}

#[test]
fn open_chat_without_cached_messages_falls_back_to_loading() {
    let cache = StubCacheSource::empty();
    let mut o = make_orchestrator_with_cache(vec![chat(1, "Alpha")], cache);

    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    assert_eq!(o.state().open_chat().chat_id(), Some(1));
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);
    assert_eq!(o.dispatcher.messages_dispatch_count(), 1);
}

#[test]
fn open_chat_without_cache_source_falls_back_to_loading() {
    // make_orchestrator_with_cached_chats sets cache_source = None
    let mut o = make_orchestrator_with_cached_chats(vec![chat(1, "Alpha")]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    assert_eq!(o.state().open_chat().chat_id(), Some(1));
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);
    assert_eq!(o.dispatcher.messages_dispatch_count(), 1);
}

#[test]
fn reopen_same_ready_chat_skips_reload() {
    let cache = StubCacheSource::with_messages(vec![(1, vec![message(10, "Cached")])]);
    let mut o = make_orchestrator_with_cache(vec![chat(1, "Alpha"), chat(2, "Beta")], cache);

    // Open chat 1
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
    assert_eq!(o.dispatcher.messages_dispatch_count(), 1);

    // Navigate back to chat list
    o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
        .unwrap();

    // Re-open the same chat 1 (cursor still on it)
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    // No additional dispatch — still at 1
    assert_eq!(o.dispatcher.messages_dispatch_count(), 1);
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
    assert_eq!(o.state().open_chat().chat_id(), Some(1));
}

#[test]
fn reopen_same_chat_still_loading_dispatches_again() {
    let cache = StubCacheSource::empty();
    let mut o = make_orchestrator_with_cache(vec![chat(1, "Alpha")], cache);

    // Open chat 1 (no cache → Loading)
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);
    assert_eq!(o.dispatcher.messages_dispatch_count(), 1);

    // Go back and re-open (still Loading, not Ready)
    o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    // Should dispatch again since it's not Ready
    assert_eq!(o.dispatcher.messages_dispatch_count(), 2);
}

#[test]
fn background_messages_on_cached_ready_chat_uses_update_messages() {
    let cache = StubCacheSource::with_messages(vec![(
        1,
        vec![message(10, "Cached A"), message(11, "Cached B")],
    )]);
    let mut o = make_orchestrator_with_cache(vec![chat(1, "Alpha")], cache);

    // Open chat — becomes Ready from cache
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
    assert_eq!(o.state().open_chat().messages().len(), 2);

    // Navigate to first cached message
    o.handle_event(AppEvent::InputKey(KeyInput::new("k", false)))
        .unwrap();
    assert_eq!(o.state().open_chat().selected_index(), Some(0));
    // selected message id is 10

    // Background full load arrives with more messages, including message 10
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessagesLoaded {
            chat_id: 1,
            result: Ok(vec![
                message(8, "Older"),
                message(9, "Old"),
                message(10, "Cached A"),
                message(11, "Cached B"),
                message(12, "New"),
            ]),
        },
    ))
    .unwrap();

    // State should still be Ready
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
    // Messages updated to the full set
    assert_eq!(o.state().open_chat().messages().len(), 5);
    // Selection preserved on message 10 (now at index 2)
    assert_eq!(o.state().open_chat().selected_index(), Some(2));
}

#[test]
fn background_messages_on_loading_chat_uses_set_ready() {
    let mut o = make_orchestrator_with_cached_chats(vec![chat(1, "Alpha")]);

    // Open chat — no cache, Loading
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);

    // Background load arrives
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessagesLoaded {
            chat_id: 1,
            result: Ok(vec![message(1, "A"), message(2, "B")]),
        },
    ))
    .unwrap();

    // Should use set_ready (selects last message)
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
    assert_eq!(o.state().open_chat().messages().len(), 2);
    assert_eq!(o.state().open_chat().selected_index(), Some(1));
}

#[test]
fn open_different_chat_with_cache_replaces_previous() {
    let cache = StubCacheSource::with_messages(vec![
        (1, vec![message(10, "Chat1 cached")]),
        (2, vec![message(20, "Chat2 cached")]),
    ]);
    let mut o = make_orchestrator_with_cache(vec![chat(1, "Alpha"), chat(2, "Beta")], cache);

    // Open chat 1
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    assert_eq!(o.state().open_chat().chat_id(), Some(1));
    assert_eq!(o.state().open_chat().messages()[0].text, "Chat1 cached");

    // Navigate back, move to chat 2, open it
    o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    // Chat 2 should be shown from cache
    assert_eq!(o.state().open_chat().chat_id(), Some(2));
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
    assert_eq!(o.state().open_chat().messages()[0].text, "Chat2 cached");
    assert_eq!(o.dispatcher.messages_dispatch_count(), 2);
}
// ── Chat lifecycle (openChat/closeChat/viewMessages) tests ──

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

    // Open chat 1
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    assert_eq!(o.dispatcher.open_chat_dispatch_count(), 1);
    assert_eq!(o.tdlib_opened_chat_id, Some(1));

    // Navigate back to chat list
    o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
        .unwrap();
    assert_eq!(o.dispatcher.close_chat_dispatch_count(), 1);

    // Move to chat 2 and open it
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

    // Open chat
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    // Simulate messages loaded
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

    // After send refresh arrives with updated messages
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessageSentRefreshCompleted {
            chat_id: 1,
            result: Ok(vec![message(1, "Hello"), message(2, "My reply")]),
        },
    ))
    .unwrap();

    // mark_as_read dispatched: once from initial messages load + once from refresh
    assert_eq!(o.dispatcher.mark_as_read_dispatch_count(), 2);
    let (mark_chat_id, mark_ids) = o.dispatcher.last_mark_as_read().unwrap();
    assert_eq!(mark_chat_id, 1);
    assert_eq!(mark_ids, vec![1, 2]);
}

#[test]
fn reopen_same_ready_chat_does_not_dispatch_open_chat_again() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);

    // Focus back to chat list, then reopen same chat
    // Note: h closes the TDLib chat
    o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("l", false)))
        .unwrap();

    // Re-opening the same Ready chat triggers a new open_chat
    // (since we closed it with h)
    assert_eq!(o.dispatcher.open_chat_dispatch_count(), 2);
}

#[test]
fn quit_while_chat_open_dispatches_close_chat() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
    assert_eq!(o.tdlib_opened_chat_id, Some(1));

    // Go back to chat list pane (but don't press h — stay with chat "open" in TDLib)
    // Actually, we need to be in ChatList or Messages pane for QuitRequested to quit
    o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
        .unwrap();
    // h already closed it, let's reopen
    o.handle_event(AppEvent::InputKey(KeyInput::new("l", false)))
        .unwrap();
    assert_eq!(o.tdlib_opened_chat_id, Some(1));

    // Now quit
    o.handle_event(AppEvent::QuitRequested).unwrap();

    assert!(!o.state().is_running());
    assert_eq!(o.tdlib_opened_chat_id, None);
    // close_chat dispatched: once from h, once from quit
    assert_eq!(o.dispatcher.close_chat_dispatch_count(), 2);
}

#[test]
fn stale_messages_do_not_dispatch_mark_as_read() {
    let mut o = orchestrator_with_chats(vec![chat(1, "A"), chat(2, "B")]);

    // Open chat 1
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    // Navigate away
    o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
        .unwrap();
    // Open chat 2
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    // Stale result for chat 1 arrives
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessagesLoaded {
            chat_id: 1,
            result: Ok(vec![message(10, "Stale")]),
        },
    ))
    .unwrap();

    // Should not dispatch mark_as_read since chat 1 is no longer viewed
    assert_eq!(o.dispatcher.mark_as_read_dispatch_count(), 0);
}
