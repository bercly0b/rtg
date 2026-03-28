use super::*;

// ── Chat update → open chat message refresh tests ──

#[test]
fn chat_update_for_open_chat_dispatches_message_refresh() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
    let before = o.dispatcher.messages_dispatch_count();

    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::ChatMetadataChanged { chat_id: 1 }],
    })
    .unwrap();

    assert_eq!(o.dispatcher.messages_dispatch_count(), before + 1);
}

#[test]
fn chat_update_for_unrelated_chat_does_not_dispatch_message_refresh() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
    let before = o.dispatcher.messages_dispatch_count();

    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::ChatMetadataChanged { chat_id: 999 }],
    })
    .unwrap();

    assert_eq!(o.dispatcher.messages_dispatch_count(), before);
}

#[test]
fn chat_update_debounces_while_messages_refresh_in_flight() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
    let before = o.dispatcher.messages_dispatch_count();

    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::ChatMetadataChanged { chat_id: 1 }],
    })
    .unwrap();
    assert_eq!(o.dispatcher.messages_dispatch_count(), before + 1);

    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::ChatMetadataChanged { chat_id: 1 }],
    })
    .unwrap();
    assert_eq!(
        o.dispatcher.messages_dispatch_count(),
        before + 1,
        "second update while in-flight should be skipped"
    );
}

#[test]
fn messages_refresh_in_flight_resets_after_messages_loaded() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hello")]);
    let before = o.dispatcher.messages_dispatch_count();

    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::ChatMetadataChanged { chat_id: 1 }],
    })
    .unwrap();
    assert_eq!(o.dispatcher.messages_dispatch_count(), before + 1);

    inject_messages(&mut o, 1, vec![message(1, "Hello"), message(2, "World")]);

    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::ChatMetadataChanged { chat_id: 1 }],
    })
    .unwrap();
    assert_eq!(
        o.dispatcher.messages_dispatch_count(),
        before + 2,
        "after MessagesLoaded, new update should dispatch again"
    );
}

#[test]
fn chat_update_with_no_open_chat_only_refreshes_chat_list() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General")]);

    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::ChatMetadataChanged { chat_id: 1 }],
    })
    .unwrap();

    assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);
    assert_eq!(o.dispatcher.messages_dispatch_count(), 0);
}

#[test]
fn messages_load_error_does_not_dispatch_mark_as_read() {
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

    assert_eq!(o.dispatcher.mark_as_read_dispatch_count(), 0);
}
// ── Push-based cache warming tests (Phase 2) ──

#[test]
fn push_new_message_warms_cache_for_non_open_chat() {
    let mut o = orchestrator_with_open_chat(
        vec![chat(1, "Alice"), chat(2, "Bob")],
        1,
        vec![message(1, "Hello")],
    );

    // Push a new message for chat 2 (not currently open)
    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::NewMessage {
            chat_id: 2,
            message: Box::new(message(10, "Hey from Bob")),
        }],
    })
    .unwrap();

    // Chat 2 should now have a cached message
    assert!(o.state().message_cache().has_messages(2));
    assert_eq!(
        o.state.message_cache_mut().get(2).unwrap()[0].text,
        "Hey from Bob"
    );
}

#[test]
fn push_new_message_appends_to_existing_cache() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alice")]);

    // Open and load chat 1
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    inject_messages(&mut o, 1, vec![message(1, "First")]);

    // Push a new message via update
    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::NewMessage {
            chat_id: 1,
            message: Box::new(message(2, "Second")),
        }],
    })
    .unwrap();

    let cached = o.state.message_cache_mut().get(1).unwrap();
    assert_eq!(cached.len(), 2);
    assert_eq!(cached[0].text, "First");
    assert_eq!(cached[1].text, "Second");
}

#[test]
fn push_delete_messages_removes_from_cache() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alice")]);

    // Open and load chat 1
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    inject_messages(&mut o, 1, vec![message(1, "Keep"), message(2, "Delete me")]);

    // Push a delete update
    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::MessagesDeleted {
            chat_id: 1,
            message_ids: vec![2],
        }],
    })
    .unwrap();

    let cached = o.state.message_cache_mut().get(1).unwrap();
    assert_eq!(cached.len(), 1);
    assert_eq!(cached[0].text, "Keep");
}

#[test]
fn push_new_message_for_open_chat_dispatches_refresh() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "Alice")], 1, vec![message(1, "Hello")]);
    let before = o.dispatcher.messages_dispatch_count();

    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::NewMessage {
            chat_id: 1,
            message: Box::new(message(2, "New message")),
        }],
    })
    .unwrap();

    // Should dispatch a message refresh for the open chat
    assert_eq!(o.dispatcher.messages_dispatch_count(), before + 1);
}

#[test]
fn push_metadata_update_does_not_warm_cache() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alice")]);

    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::ChatMetadataChanged { chat_id: 1 }],
    })
    .unwrap();

    // Metadata updates should not create cache entries
    assert!(!o.state().message_cache().has_messages(1));
}

#[test]
fn push_cache_warm_then_open_is_instant() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alice"), chat(2, "Bob")]);

    // Push messages for chat 2 (not open)
    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![
            ChatUpdate::NewMessage {
                chat_id: 2,
                message: Box::new(message(10, "Bob msg 1")),
            },
            ChatUpdate::NewMessage {
                chat_id: 2,
                message: Box::new(message(11, "Bob msg 2")),
            },
        ],
    })
    .unwrap();

    // Navigate to chat 2 and open it
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    // Should be Ready instantly from push-warmed cache
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
    assert_eq!(o.state().open_chat().messages().len(), 2);
    assert_eq!(o.state().open_chat().messages()[0].text, "Bob msg 1");
}
// ── UserStatusChanged tests ──

fn group_chat(chat_id: i64, title: &str) -> ChatSummary {
    use crate::domain::chat::{ChatType, OutgoingReadStatus};
    ChatSummary {
        chat_id,
        title: title.to_owned(),
        unread_count: 0,
        last_message_preview: None,
        last_message_unix_ms: None,
        is_pinned: false,
        chat_type: ChatType::Group,
        last_message_sender: None,
        is_online: None,
        is_bot: false,
        outgoing_status: OutgoingReadStatus::default(),
        last_message_id: None,
        unread_reaction_count: 0,
    }
}

#[test]
fn user_status_changed_dispatches_subtitle_for_open_private_chat() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "Alice")], 1, vec![message(1, "Hello")]);

    let before = o.dispatcher.subtitle_dispatch_count();

    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::UserStatusChanged { user_id: 1 }],
    })
    .unwrap();

    assert_eq!(o.dispatcher.subtitle_dispatch_count(), before + 1);
    let query = o.dispatcher.last_subtitle_query().unwrap();
    assert_eq!(query.chat_id, 1);
    assert_eq!(query.chat_type, ChatType::Private);
}

#[test]
fn user_status_changed_skips_subtitle_for_group_chat() {
    let mut o =
        orchestrator_with_open_chat(vec![group_chat(1, "Devs")], 1, vec![message(1, "Hello")]);

    let before = o.dispatcher.subtitle_dispatch_count();

    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::UserStatusChanged { user_id: 42 }],
    })
    .unwrap();

    assert_eq!(o.dispatcher.subtitle_dispatch_count(), before);
}

#[test]
fn user_status_changed_skips_subtitle_when_no_chat_open() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alice")]);

    let before = o.dispatcher.subtitle_dispatch_count();

    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::UserStatusChanged { user_id: 1 }],
    })
    .unwrap();

    assert_eq!(o.dispatcher.subtitle_dispatch_count(), before);
}

#[test]
fn user_status_changed_refreshes_chat_list() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alice")]);

    let before = o.dispatcher.chat_list_dispatch_count();

    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::UserStatusChanged { user_id: 1 }],
    })
    .unwrap();

    assert_eq!(o.dispatcher.chat_list_dispatch_count(), before + 1);
}
