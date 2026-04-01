use super::*;

#[test]
fn tick_dispatches_chat_list_when_loading() {
    let mut o = make_orchestrator();
    assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Loading);

    o.handle_event(AppEvent::Tick).unwrap();
    assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);
}

#[test]
fn tick_does_not_dispatch_when_chat_list_is_ready() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General")]);
    o.handle_event(AppEvent::Tick).unwrap();
    // Only the initial dispatch from inject_chat_list path; tick should not add another
    assert_eq!(o.dispatcher.chat_list_dispatch_count(), 0);
}

#[test]
fn chat_list_loaded_result_sets_ready_state() {
    let mut o = make_orchestrator();
    inject_chat_list(&mut o, vec![chat(1, "General"), chat(2, "Backend")]);

    assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Ready);
    assert_eq!(o.state().chat_list().selected_index(), Some(0));
    assert_eq!(o.state().chat_list().chats().len(), 2);
}

#[test]
fn chat_list_loaded_error_sets_error_state() {
    let mut o = make_orchestrator();
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::ChatListLoaded {
            result: Err(BackgroundError::new("CHAT_LIST_UNAVAILABLE")),
        },
    ))
    .unwrap();

    assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Error);
}

#[test]
fn chat_list_reload_preserves_selection_by_current_chat_id() {
    let mut o =
        orchestrator_with_chats(vec![chat(1, "General"), chat(2, "Backend"), chat(3, "Ops")]);
    // Navigate to "Backend" (index 1)
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    assert_eq!(
        o.state().chat_list().selected_chat().map(|c| c.chat_id),
        Some(2)
    );

    // Simulate a background reload where chat order changed
    // (e.g. chat 3 got a new message and moved to the top)
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::ChatListLoaded {
            result: Ok(vec![chat(3, "Ops"), chat(1, "General"), chat(2, "Backend")]),
        },
    ))
    .unwrap();

    // Selection should follow chat_id 2 ("Backend"), now at index 2
    assert_eq!(o.state().chat_list().selected_index(), Some(2));
    assert_eq!(
        o.state().chat_list().selected_chat().map(|c| c.chat_id),
        Some(2)
    );
}

#[test]
fn chat_list_reload_cursor_follows_current_selection_not_dispatch_time() {
    // Regression test: cursor should not jump when the user navigates
    // with j/k while a background chat list refresh is in flight.
    let mut o = orchestrator_with_chats(vec![
        chat(1, "Alpha"),
        chat(2, "Beta"),
        chat(3, "Gamma"),
        chat(4, "Delta"),
        chat(5, "Epsilon"),
    ]);
    assert_eq!(o.state().chat_list().selected_index(), Some(0));

    // Trigger a background refresh (e.g. from TDLib update)
    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::ChatMetadataChanged { chat_id: 1 }],
    })
    .unwrap();
    assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);

    // User navigates down while refresh is in-flight
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    assert_eq!(
        o.state().chat_list().selected_chat().map(|c| c.chat_id),
        Some(4) // "Delta"
    );

    // Background result arrives with reordered chats
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::ChatListLoaded {
            result: Ok(vec![
                chat(5, "Epsilon"),
                chat(1, "Alpha"),
                chat(4, "Delta"),
                chat(2, "Beta"),
                chat(3, "Gamma"),
            ]),
        },
    ))
    .unwrap();

    // Selection must stay on "Delta" (chat_id=4), now at index 2
    assert_eq!(
        o.state().chat_list().selected_chat().map(|c| c.chat_id),
        Some(4)
    );
    assert_eq!(o.state().chat_list().selected_index(), Some(2));
}

#[test]
fn chat_list_reload_falls_back_when_selected_chat_disappears() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alpha"), chat(2, "Beta"), chat(3, "Gamma")]);
    // Navigate to "Gamma"
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    assert_eq!(
        o.state().chat_list().selected_chat().map(|c| c.chat_id),
        Some(3)
    );

    // Background refresh arrives without chat 3
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::ChatListLoaded {
            result: Ok(vec![chat(1, "Alpha"), chat(2, "Beta")]),
        },
    ))
    .unwrap();

    // Should fall back to first chat
    assert_eq!(o.state().chat_list().selected_index(), Some(0));
    assert_eq!(
        o.state().chat_list().selected_chat().map(|c| c.chat_id),
        Some(1)
    );
}

#[test]
fn key_contract_navigates_chat_list_with_vim_keys() {
    let mut o =
        orchestrator_with_chats(vec![chat(1, "General"), chat(2, "Backend"), chat(3, "Ops")]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    assert_eq!(o.state().chat_list().selected_index(), Some(1));

    o.handle_event(AppEvent::InputKey(KeyInput::new("k", false)))
        .unwrap();
    assert_eq!(o.state().chat_list().selected_index(), Some(0));
}
#[test]
fn refresh_key_dispatches_chat_list() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("R", false)))
        .unwrap();
    assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);
}

#[test]
fn chat_list_update_event_dispatches_refresh() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General")]);
    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::ChatMetadataChanged { chat_id: 1 }],
    })
    .unwrap();
    assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);
}

#[test]
fn duplicate_chat_list_dispatch_is_guarded_but_sets_pending() {
    let mut o = make_orchestrator();
    // First tick dispatches
    o.handle_event(AppEvent::Tick).unwrap();
    assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);
    assert!(o.chat_list_in_flight);
    assert!(!o.chat_list_refresh_pending);

    // Second tick should not dispatch again, but sets pending flag
    o.handle_event(AppEvent::Tick).unwrap();
    assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);
    assert!(o.chat_list_refresh_pending);
}

#[test]
fn in_flight_guard_resets_after_result() {
    let mut o = make_orchestrator();
    o.handle_event(AppEvent::Tick).unwrap();
    assert!(o.chat_list_in_flight);

    inject_chat_list(&mut o, vec![chat(1, "General")]);
    assert!(!o.chat_list_in_flight);

    // Now another dispatch should work
    o.handle_event(AppEvent::InputKey(KeyInput::new("R", false)))
        .unwrap();
    assert_eq!(o.dispatcher.chat_list_dispatch_count(), 2);
}

#[test]
fn user_refresh_shows_notification_on_success() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General")]);

    // User presses R to refresh — immediate "Refreshing..." feedback
    o.handle_event(AppEvent::InputKey(KeyInput::new("R", false)))
        .unwrap();
    assert_eq!(
        o.state().active_notification(),
        Some("Refreshing chat list...")
    );

    // Background result arrives — notification updates to "refreshed"
    inject_chat_list(&mut o, vec![chat(1, "General"), chat(2, "Backend")]);
    assert_eq!(o.state().active_notification(), Some("Chat list refreshed"));
}

#[test]
fn user_refresh_shows_notification_on_failure() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General")]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("R", false)))
        .unwrap();
    assert_eq!(
        o.state().active_notification(),
        Some("Refreshing chat list...")
    );

    // Inject a failure
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::ChatListLoaded {
            result: Err(BackgroundError::new("CHAT_LIST_UNAVAILABLE")),
        },
    ))
    .unwrap();
    assert_eq!(
        o.state().active_notification(),
        Some("Chat list refresh failed")
    );
}

#[test]
fn automatic_refresh_does_not_show_notification() {
    let mut o = make_orchestrator();
    // Initial tick triggers automatic refresh
    o.handle_event(AppEvent::Tick).unwrap();

    inject_chat_list(&mut o, vec![chat(1, "General")]);
    assert!(o.state().active_notification().is_none());
}

#[test]
fn refresh_from_ready_keeps_data_visible() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General"), chat(2, "Backend")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();

    // Trigger refresh via "R" key
    o.handle_event(AppEvent::InputKey(KeyInput::new("R", false)))
        .unwrap();

    // State must stay Ready with existing data — no blink
    assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Ready);
    assert_eq!(o.state().chat_list().chats().len(), 2);
    assert_eq!(o.state().chat_list().selected_index(), Some(1));
    // But a background dispatch was issued
    assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);
}

#[test]
fn chat_list_update_event_keeps_data_visible() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General")]);

    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::ChatMetadataChanged { chat_id: 1 }],
    })
    .unwrap();

    // Must not blink — state stays Ready while background fetch runs
    assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Ready);
    assert_eq!(o.state().chat_list().chats().len(), 1);
    assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);
}
// ── Mark chat as read from chat list (r key) ──

fn chat_with_unread(
    chat_id: i64,
    title: &str,
    unread: u32,
    last_msg_id: Option<i64>,
) -> ChatSummary {
    use crate::domain::chat::{ChatType, OutgoingReadStatus};
    ChatSummary {
        chat_id,
        title: title.to_owned(),
        unread_count: unread,
        last_message_preview: Some("text".to_owned()),
        last_message_unix_ms: None,
        is_pinned: false,
        chat_type: ChatType::Private,
        last_message_sender: None,
        is_online: None,
        is_bot: false,
        outgoing_status: OutgoingReadStatus::default(),
        last_message_id: last_msg_id,
        unread_reaction_count: 0,
    }
}

#[test]
fn r_key_marks_selected_chat_as_read() {
    let mut o = orchestrator_with_chats(vec![chat_with_unread(1, "General", 5, Some(100))]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("r", false)))
        .unwrap();

    assert_eq!(o.dispatcher.mark_chat_as_read_dispatch_count(), 1);
    assert_eq!(o.dispatcher.last_mark_chat_as_read(), Some((1, 100)));
}

#[test]
fn r_key_does_nothing_when_chat_has_no_unread() {
    let mut o = orchestrator_with_chats(vec![chat_with_unread(1, "General", 0, Some(100))]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("r", false)))
        .unwrap();

    assert_eq!(o.dispatcher.mark_chat_as_read_dispatch_count(), 0);
}

#[test]
fn r_key_does_nothing_when_no_last_message_id() {
    let mut o = orchestrator_with_chats(vec![chat_with_unread(1, "General", 3, None)]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("r", false)))
        .unwrap();

    assert_eq!(o.dispatcher.mark_chat_as_read_dispatch_count(), 0);
}

#[test]
fn r_key_uses_mark_as_read_when_chat_already_opened_in_tdlib() {
    let mut o = orchestrator_with_chats(vec![chat_with_unread(1, "General", 5, Some(100))]);

    // Open the chat first (which sets tdlib_opened_chat_id)
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    // Go back to chat list
    o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
        .unwrap();

    // Now press r — should use dispatch_mark_as_read (not dispatch_mark_chat_as_read)
    // because the chat is still open in TDLib (closeChat was called when pressing h)
    // Actually, pressing h calls close_tdlib_chat, so the chat is closed.
    // This means dispatch_mark_chat_as_read will be used.
    o.handle_event(AppEvent::InputKey(KeyInput::new("r", false)))
        .unwrap();

    assert_eq!(o.dispatcher.mark_chat_as_read_dispatch_count(), 1);
}

#[test]
fn r_key_optimistically_clears_unread_counter() {
    let mut o = orchestrator_with_chats(vec![
        chat_with_unread(1, "General", 5, Some(100)),
        chat_with_unread(2, "Backend", 3, Some(200)),
    ]);

    // Select second chat and press r
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("r", false)))
        .unwrap();

    // Unread counter should be cleared immediately (optimistic)
    let chats = o.state().chat_list().chats();
    assert_eq!(chats[0].unread_count, 5); // first chat unchanged
    assert_eq!(chats[1].unread_count, 0); // second chat cleared
}

// ── Dirty flag: pending refresh after in-flight completion ──

#[test]
fn pending_flag_triggers_re_dispatch_after_chat_list_loaded() {
    let mut o = make_orchestrator();

    // First tick dispatches chat list load
    o.handle_event(AppEvent::Tick).unwrap();
    assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);
    assert!(o.chat_list_in_flight);

    // TDLib update arrives while in-flight — sets pending flag
    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::ChatMetadataChanged { chat_id: 1 }],
    })
    .unwrap();
    assert_eq!(
        o.dispatcher.chat_list_dispatch_count(),
        1,
        "should not dispatch while in-flight"
    );
    assert!(o.chat_list_refresh_pending);

    // Background result arrives — triggers auto re-dispatch
    inject_chat_list(&mut o, vec![chat(1, "General")]);
    assert_eq!(
        o.dispatcher.chat_list_dispatch_count(),
        2,
        "pending flag should trigger another dispatch"
    );
    assert!(
        !o.chat_list_refresh_pending,
        "pending flag should be cleared"
    );
}

#[test]
fn no_extra_dispatch_when_pending_flag_is_not_set() {
    let mut o = make_orchestrator();

    // First tick dispatches
    o.handle_event(AppEvent::Tick).unwrap();
    assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);
    assert!(!o.chat_list_refresh_pending);

    // Background result arrives without any pending updates
    inject_chat_list(&mut o, vec![chat(1, "General")]);
    assert_eq!(
        o.dispatcher.chat_list_dispatch_count(),
        1,
        "no extra dispatch without pending flag"
    );
}

#[test]
fn pending_flag_cleared_on_error_result_and_re_dispatches() {
    let mut o = make_orchestrator();

    // First tick dispatches
    o.handle_event(AppEvent::Tick).unwrap();

    // Set pending via update while in-flight
    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::ChatMetadataChanged { chat_id: 1 }],
    })
    .unwrap();
    assert!(o.chat_list_refresh_pending);

    // Error result still triggers re-dispatch
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::ChatListLoaded {
            result: Err(BackgroundError::new("FAIL")),
        },
    ))
    .unwrap();
    assert_eq!(o.dispatcher.chat_list_dispatch_count(), 2);
    assert!(!o.chat_list_refresh_pending);
}

// ── Force refresh: R key dispatches with force=true ──

#[test]
fn r_key_dispatches_with_force_true() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General")]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("R", false)))
        .unwrap();

    assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);
    assert_eq!(
        o.dispatcher.last_chat_list_force(),
        Some(true),
        "R key should dispatch with force=true"
    );
}

#[test]
fn tick_dispatches_with_force_false() {
    let mut o = make_orchestrator();

    o.handle_event(AppEvent::Tick).unwrap();

    assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);
    assert_eq!(
        o.dispatcher.last_chat_list_force(),
        Some(false),
        "tick should dispatch with force=false"
    );
}

#[test]
fn chat_update_dispatches_with_force_false() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General")]);

    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::ChatMetadataChanged { chat_id: 1 }],
    })
    .unwrap();

    assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);
    assert_eq!(
        o.dispatcher.last_chat_list_force(),
        Some(false),
        "auto-refresh from update should use force=false"
    );
}

#[test]
fn r_during_inflight_preserves_force_on_pending_redispatch() {
    let mut o = make_orchestrator();

    // Initial auto-refresh from Tick
    o.handle_event(AppEvent::Tick).unwrap();
    assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);
    assert_eq!(o.dispatcher.last_chat_list_force(), Some(false));

    // User presses R while auto-refresh is in-flight
    o.handle_event(AppEvent::InputKey(KeyInput::new("R", false)))
        .unwrap();
    assert_eq!(
        o.dispatcher.chat_list_dispatch_count(),
        1,
        "should not dispatch while in-flight"
    );
    assert!(o.chat_list_refresh_pending);
    assert!(o.chat_list_pending_force, "force=true should be preserved");

    // First result arrives — triggers pending re-dispatch with force
    inject_chat_list(&mut o, vec![chat(1, "General")]);
    assert_eq!(o.dispatcher.chat_list_dispatch_count(), 2);
    assert_eq!(
        o.dispatcher.last_chat_list_force(),
        Some(true),
        "pending re-dispatch should use force=true from R key"
    );
}

#[test]
fn r_during_inflight_defers_notification_to_redispatch() {
    let mut o = make_orchestrator();

    // Initial auto-refresh
    o.handle_event(AppEvent::Tick).unwrap();

    // User presses R while in-flight
    o.handle_event(AppEvent::InputKey(KeyInput::new("R", false)))
        .unwrap();

    // First result arrives — should NOT show "Chat list refreshed" yet
    inject_chat_list(&mut o, vec![chat(1, "General")]);
    assert_eq!(
        o.state().active_notification(),
        Some("Refreshing chat list..."),
        "notification from R key press should persist until re-dispatch completes"
    );

    // Second result (the force-refresh) arrives — NOW show notification
    inject_chat_list(&mut o, vec![chat(1, "General"), chat(2, "Backend")]);
    assert_eq!(
        o.state().active_notification(),
        Some("Chat list refreshed"),
        "notification should appear after the forced re-dispatch completes"
    );
}

#[test]
fn auto_update_during_inflight_redispatches_without_force() {
    let mut o = make_orchestrator();

    // Initial auto-refresh
    o.handle_event(AppEvent::Tick).unwrap();

    // Auto-update arrives while in-flight (no force)
    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::ChatMetadataChanged { chat_id: 1 }],
    })
    .unwrap();
    assert!(o.chat_list_refresh_pending);
    assert!(
        !o.chat_list_pending_force,
        "auto-update should not set force"
    );

    // First result arrives — re-dispatch without force
    inject_chat_list(&mut o, vec![chat(1, "General")]);
    assert_eq!(
        o.dispatcher.last_chat_list_force(),
        Some(false),
        "auto-update re-dispatch should use force=false"
    );
}
