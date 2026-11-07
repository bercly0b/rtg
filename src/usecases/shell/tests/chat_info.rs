use super::*;

// ── Chat info popup tests ──

#[test]
fn i_key_opens_chat_info_popup_when_chat_selected() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alice")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("I", false)))
        .unwrap();
    assert!(o.state().chat_info_popup().is_some());
    assert_eq!(o.state().chat_info_popup().unwrap().title(), "Alice");
}

#[test]
fn i_key_dispatches_chat_info_query_with_chat_folders() {
    use crate::domain::chat_folder::{ChatFolder, ChatFolders};

    let folders = ChatFolders {
        folders: vec![ChatFolder {
            id: 3,
            title: "Work".to_owned(),
        }],
        main_list_position: 0,
    };
    let mut o = orchestrator_with_chats(vec![chat(1, "Alice")]);
    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![crate::domain::events::ChatUpdate::ChatFoldersChanged {
            folders: folders.clone(),
        }],
    })
    .unwrap();

    o.handle_event(AppEvent::InputKey(KeyInput::new("I", false)))
        .unwrap();

    let query = o.dispatcher.last_chat_info_query().unwrap();
    assert_eq!(query.chat_id, 1);
    assert_eq!(query.title, "Alice");
    assert_eq!(query.folders, folders);
}

#[test]
fn i_key_does_nothing_when_no_chat_selected() {
    let mut o = make_orchestrator();
    o.handle_event(AppEvent::InputKey(KeyInput::new("I", false)))
        .unwrap();
    assert!(o.state().chat_info_popup().is_none());
}

#[test]
fn chat_info_popup_closes_on_esc() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alice")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("I", false)))
        .unwrap();
    assert!(o.state().chat_info_popup().is_some());

    o.handle_event(AppEvent::InputKey(KeyInput::new("esc", false)))
        .unwrap();
    assert!(o.state().chat_info_popup().is_none());
}

#[test]
fn chat_info_popup_closes_on_q() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alice")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("I", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("q", false)))
        .unwrap();
    assert!(o.state().chat_info_popup().is_none());
}

#[test]
fn chat_info_popup_closes_on_second_i() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alice")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("I", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("I", false)))
        .unwrap();
    assert!(o.state().chat_info_popup().is_none());
}

#[test]
fn chat_info_popup_ignores_other_keys() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alice")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("I", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    // Popup is still open — j key was ignored
    assert!(o.state().chat_info_popup().is_some());
}

#[test]
fn chat_info_loaded_updates_popup_state() {
    use crate::domain::chat_info_state::{tests::sample_chat_info, ChatInfoPopupState};

    let mut o = orchestrator_with_chats(vec![chat(1, "Alice")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("I", false)))
        .unwrap();

    // Simulate background task completion
    o.handle_event(AppEvent::BackgroundTasksCompleted(vec![
        BackgroundTaskResult::ChatInfoLoaded {
            chat_id: 1,
            result: Ok(sample_chat_info("Alice")),
        },
    ]))
    .unwrap();

    match o.state().chat_info_popup().unwrap() {
        ChatInfoPopupState::Loaded(info) => assert_eq!(**info, sample_chat_info("Alice")),
        _ => panic!("expected Loaded state"),
    }
}

#[test]
fn chat_info_loaded_error_sets_error_state() {
    use crate::domain::chat_info_state::ChatInfoPopupState;

    let mut o = orchestrator_with_chats(vec![chat(1, "Alice")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("I", false)))
        .unwrap();

    o.handle_event(AppEvent::BackgroundTasksCompleted(vec![
        BackgroundTaskResult::ChatInfoLoaded {
            chat_id: 1,
            result: Err(BackgroundError::new("CHAT_INFO_UNAVAILABLE")),
        },
    ]))
    .unwrap();

    match o.state().chat_info_popup().unwrap() {
        ChatInfoPopupState::Error { title } => {
            assert_eq!(title, "Alice");
        }
        _ => panic!("expected Error state"),
    }
}

#[test]
fn chat_info_loaded_ignored_when_popup_closed() {
    use crate::domain::chat_info_state::tests::sample_chat_info;

    let mut o = orchestrator_with_chats(vec![chat(1, "Alice")]);
    // No popup open — result should be silently ignored
    o.handle_event(AppEvent::BackgroundTasksCompleted(vec![
        BackgroundTaskResult::ChatInfoLoaded {
            chat_id: 1,
            result: Ok(sample_chat_info("Alice")),
        },
    ]))
    .unwrap();

    assert!(o.state().chat_info_popup().is_none());
}

#[test]
fn chat_info_loaded_stale_result_ignored() {
    use crate::domain::chat_info_state::{tests::sample_chat_info, ChatInfoPopupState};

    let mut o = orchestrator_with_chats(vec![chat(1, "Alice"), chat(2, "Bob")]);

    // Open popup for Alice (chat_id=1)
    o.handle_event(AppEvent::InputKey(KeyInput::new("I", false)))
        .unwrap();
    assert_eq!(o.state().chat_info_popup().unwrap().title(), "Alice");

    // Close popup and re-open for Bob (chat_id=2)
    o.handle_event(AppEvent::InputKey(KeyInput::new("esc", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("I", false)))
        .unwrap();
    assert_eq!(o.state().chat_info_popup().unwrap().title(), "Bob");

    // Stale result for Alice arrives — should be ignored
    o.handle_event(AppEvent::BackgroundTasksCompleted(vec![
        BackgroundTaskResult::ChatInfoLoaded {
            chat_id: 1,
            result: Ok(sample_chat_info("Alice")),
        },
    ]))
    .unwrap();

    // Popup should still show Bob (Loading state), not Alice's data
    match o.state().chat_info_popup().unwrap() {
        ChatInfoPopupState::Loading { title, .. } => assert_eq!(title, "Bob"),
        _ => panic!("expected Loading state for Bob, not stale Alice data"),
    }
}
