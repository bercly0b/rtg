use super::*;

// -- Cached startup tests --

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

    assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Ready);

    o.handle_event(AppEvent::Tick).unwrap();
    assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);

    assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Ready);
    assert_eq!(o.state().chat_list().chats().len(), 2);
}

#[test]
fn cached_startup_refresh_only_fires_once() {
    let mut o = make_orchestrator_with_cached_chats(vec![chat(1, "Alpha"), chat(2, "Beta")]);

    o.handle_event(AppEvent::Tick).unwrap();
    assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);

    inject_chat_list(
        &mut o,
        vec![chat(1, "Alpha"), chat(2, "Beta"), chat(3, "Gamma")],
    );

    o.handle_event(AppEvent::Tick).unwrap();
    assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);
}

#[test]
fn cached_startup_background_result_updates_list() {
    let mut o = make_orchestrator_with_cached_chats(vec![chat(1, "Alpha"), chat(2, "Beta")]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    assert_eq!(
        o.state().chat_list().selected_chat().map(|c| c.chat_id),
        Some(2)
    );

    o.handle_event(AppEvent::Tick).unwrap();

    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::ChatListLoaded {
            result: Ok(vec![chat(3, "Gamma"), chat(1, "Alpha"), chat(2, "Beta")]),
        },
    ))
    .unwrap();

    assert_eq!(
        o.state().chat_list().selected_chat().map(|c| c.chat_id),
        Some(2)
    );
    assert_eq!(o.state().chat_list().chats().len(), 3);
}

#[test]
fn non_cached_startup_does_not_set_initial_refresh_flag() {
    let mut o = make_orchestrator();

    assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Loading);

    o.handle_event(AppEvent::Tick).unwrap();
    assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);

    inject_chat_list(&mut o, vec![chat(1, "Alpha")]);

    o.handle_event(AppEvent::Tick).unwrap();
    assert_eq!(o.dispatcher.chat_list_dispatch_count(), 1);
}

// -- Cached messages on chat open tests --

#[test]
fn open_chat_with_cache_shows_ready_instantly() {
    let cache = StubCacheSource::with_messages(vec![(
        1,
        vec![message(10, "Cached A"), message(11, "Cached B")],
    )]);
    let mut o = make_orchestrator_with_cache(vec![chat(1, "Alpha"), chat(2, "Beta")], cache);

    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    assert_eq!(o.state().open_chat().chat_id(), Some(1));
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
    assert_eq!(o.state().open_chat().messages().len(), 2);
    assert_eq!(o.state().open_chat().messages()[0].text, "Cached A");

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

    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
    assert_eq!(o.dispatcher.messages_dispatch_count(), 1);

    o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
        .unwrap();

    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    assert_eq!(o.dispatcher.messages_dispatch_count(), 1);
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
    assert_eq!(o.state().open_chat().chat_id(), Some(1));
}

#[test]
fn reopen_same_chat_still_loading_dispatches_again() {
    let cache = StubCacheSource::empty();
    let mut o = make_orchestrator_with_cache(vec![chat(1, "Alpha")], cache);

    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);
    assert_eq!(o.dispatcher.messages_dispatch_count(), 1);

    o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    assert_eq!(o.dispatcher.messages_dispatch_count(), 2);
}

#[test]
fn background_messages_on_cached_ready_chat_uses_update_messages() {
    let cache = StubCacheSource::with_messages(vec![(
        1,
        vec![message(10, "Cached A"), message(11, "Cached B")],
    )]);
    let mut o = make_orchestrator_with_cache(vec![chat(1, "Alpha")], cache);

    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
    assert_eq!(o.state().open_chat().messages().len(), 2);

    o.handle_event(AppEvent::InputKey(KeyInput::new("k", false)))
        .unwrap();
    assert_eq!(o.state().open_chat().selected_index(), Some(0));

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

    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
    assert_eq!(o.state().open_chat().messages().len(), 5);
    assert_eq!(o.state().open_chat().selected_index(), Some(2));
}

#[test]
fn background_messages_on_loading_chat_uses_set_ready() {
    let mut o = make_orchestrator_with_cached_chats(vec![chat(1, "Alpha")]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Loading);

    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessagesLoaded {
            chat_id: 1,
            result: Ok(vec![message(1, "A"), message(2, "B")]),
        },
    ))
    .unwrap();

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

    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    assert_eq!(o.state().open_chat().chat_id(), Some(1));
    assert_eq!(o.state().open_chat().messages()[0].text, "Chat1 cached");

    o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    assert_eq!(o.state().open_chat().chat_id(), Some(2));
    assert_eq!(o.state().open_chat().ui_state(), OpenChatUiState::Ready);
    assert_eq!(o.state().open_chat().messages()[0].text, "Chat2 cached");
    assert_eq!(o.dispatcher.messages_dispatch_count(), 2);
}
