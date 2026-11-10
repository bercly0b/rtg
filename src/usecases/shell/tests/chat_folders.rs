use super::*;
use crate::domain::{
    chat_folder::{ChatFolder, ChatFolders, ARCHIVE_CHAT_LIST_TITLE, MAIN_CHAT_LIST_TITLE},
    events::ChatUpdate,
};

fn press(o: &mut TestOrchestrator, key: &str) {
    o.handle_event(AppEvent::InputKey(KeyInput::new(key, false)))
        .unwrap();
}

fn push_folders(o: &mut TestOrchestrator) {
    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::ChatFoldersChanged {
            folders: ChatFolders {
                folders: vec![
                    ChatFolder {
                        id: 3,
                        title: "Work".to_owned(),
                    },
                    ChatFolder {
                        id: 7,
                        title: "Family".to_owned(),
                    },
                ],
                main_list_position: 0,
            },
        }],
    })
    .unwrap();
}

fn inject_folder_chat_list(o: &mut TestOrchestrator, folder_id: i32, chats: Vec<ChatSummary>) {
    o.handle_event(AppEvent::BackgroundTasksCompleted(vec![
        BackgroundTaskResult::ChatListLoaded {
            list: ChatListKind::Folder(folder_id),
            result: Ok(chats),
            all_loaded: true,
        },
    ]))
    .unwrap();
}

fn orchestrator_with_folders() -> TestOrchestrator {
    let mut o = orchestrator_with_chats(vec![chat(1, "General"), chat(2, "Backend")]);
    push_folders(&mut o);
    o
}

/// Opens the picker and picks the "Work" folder (second entry).
fn switch_to_work(o: &mut TestOrchestrator) {
    press(o, "f");
    press(o, "j");
    press(o, "enter");
}

#[test]
fn f_opens_picker_with_cursor_on_active_list() {
    let mut o = orchestrator_with_folders();

    press(&mut o, "f");

    let picker = o.state().chat_folder_picker().expect("picker must be open");
    let titles: Vec<&str> = picker
        .entries()
        .iter()
        .map(|entry| entry.title.as_str())
        .collect();
    assert_eq!(
        titles,
        vec![
            MAIN_CHAT_LIST_TITLE,
            "Work",
            "Family",
            ARCHIVE_CHAT_LIST_TITLE
        ]
    );
    assert_eq!(
        picker.selected().map(|entry| entry.list),
        Some(ChatListKind::Main)
    );
}

#[test]
fn j_and_k_move_picker_cursor_not_chat_selection() {
    let mut o = orchestrator_with_folders();
    press(&mut o, "f");

    press(&mut o, "j");
    press(&mut o, "j");
    press(&mut o, "k");

    let picker = o.state().chat_folder_picker().unwrap();
    assert_eq!(
        picker.selected().map(|entry| entry.list),
        Some(ChatListKind::Folder(3))
    );
    assert_eq!(o.state().chat_list().selected_index(), Some(0));
}

#[test]
fn enter_switches_to_folder_and_loads_its_chats() {
    let mut o = orchestrator_with_folders();
    let dispatches_before = o.dispatcher.chat_list_dispatch_count();

    switch_to_work(&mut o);

    assert!(o.state().chat_folder_picker().is_none());
    assert_eq!(o.state().chat_list().kind(), ChatListKind::Folder(3));
    assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Loading);
    assert!(o.state().chat_list().chats().is_empty());
    assert_eq!(o.state().active_chat_list_title(), Some("Work"));
    assert_eq!(
        o.dispatcher.chat_list_dispatch_count(),
        dispatches_before + 1
    );
    assert_eq!(o.dispatcher.last_chat_list(), Some(ChatListKind::Folder(3)));
}

#[test]
fn folder_result_replaces_chat_list() {
    let mut o = orchestrator_with_folders();
    switch_to_work(&mut o);

    inject_folder_chat_list(&mut o, 3, vec![chat(5, "Team"), chat(6, "Standup")]);

    let chat_list = o.state().chat_list();
    assert_eq!(chat_list.ui_state(), ChatListUiState::Ready);
    let ids: Vec<i64> = chat_list.chats().iter().map(|c| c.chat_id).collect();
    assert_eq!(ids, vec![5, 6]);
    assert_eq!(chat_list.selected_index(), Some(0));
}

#[test]
fn stale_result_for_previous_list_is_dropped_and_folder_reloaded() {
    let mut o = make_orchestrator();
    push_folders(&mut o);
    // The initial Tick puts a main-list request in flight.
    o.handle_event(AppEvent::Tick).unwrap();
    assert_eq!(o.dispatcher.last_chat_list(), Some(ChatListKind::Main));
    let dispatches_before = o.dispatcher.chat_list_dispatch_count();

    switch_to_work(&mut o);
    assert_eq!(
        o.dispatcher.chat_list_dispatch_count(),
        dispatches_before,
        "switch must wait for the in-flight request"
    );

    inject_chat_list(&mut o, vec![chat(1, "General")]);

    assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Loading);
    assert!(o.state().chat_list().chats().is_empty());
    assert_eq!(
        o.dispatcher.chat_list_dispatch_count(),
        dispatches_before + 1
    );
    assert_eq!(o.dispatcher.last_chat_list(), Some(ChatListKind::Folder(3)));
}

#[test]
fn esc_closes_picker_without_switching() {
    let mut o = orchestrator_with_folders();
    let dispatches_before = o.dispatcher.chat_list_dispatch_count();
    press(&mut o, "f");
    press(&mut o, "j");

    press(&mut o, "esc");

    assert!(o.state().chat_folder_picker().is_none());
    assert_eq!(o.state().chat_list().kind(), ChatListKind::Main);
    assert_eq!(o.state().chat_list().chats().len(), 2);
    assert_eq!(o.dispatcher.chat_list_dispatch_count(), dispatches_before);
}

#[test]
fn q_and_f_close_picker_without_quitting() {
    let mut o = orchestrator_with_folders();

    press(&mut o, "f");
    press(&mut o, "q");
    assert!(o.state().chat_folder_picker().is_none());
    assert!(o.state().is_running());

    press(&mut o, "f");
    press(&mut o, "f");
    assert!(o.state().chat_folder_picker().is_none());
}

#[test]
fn picking_active_list_keeps_chats_without_reload() {
    let mut o = orchestrator_with_folders();
    let dispatches_before = o.dispatcher.chat_list_dispatch_count();

    press(&mut o, "f");
    press(&mut o, "enter");

    assert!(o.state().chat_folder_picker().is_none());
    assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Ready);
    assert_eq!(o.state().chat_list().chats().len(), 2);
    assert_eq!(o.dispatcher.chat_list_dispatch_count(), dispatches_before);
}

#[test]
fn picking_main_list_switches_back_from_folder() {
    let mut o = orchestrator_with_folders();
    switch_to_work(&mut o);
    inject_folder_chat_list(&mut o, 3, vec![chat(5, "Team")]);

    press(&mut o, "f");
    press(&mut o, "k");
    press(&mut o, "enter");

    assert_eq!(o.state().chat_list().kind(), ChatListKind::Main);
    assert_eq!(o.state().active_chat_list_title(), None);
    assert_eq!(o.dispatcher.last_chat_list(), Some(ChatListKind::Main));
}

#[test]
fn refresh_reloads_active_folder() {
    let mut o = orchestrator_with_folders();
    switch_to_work(&mut o);
    inject_folder_chat_list(&mut o, 3, vec![chat(5, "Team")]);

    press(&mut o, "R");

    assert_eq!(o.dispatcher.last_chat_list_force(), Some(true));
    assert_eq!(o.dispatcher.last_chat_list(), Some(ChatListKind::Folder(3)));
}

#[test]
fn chat_update_reloads_active_folder() {
    let mut o = orchestrator_with_folders();
    switch_to_work(&mut o);
    inject_folder_chat_list(&mut o, 3, vec![chat(5, "Team")]);

    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::ChatMetadataChanged { chat_id: 5 }],
    })
    .unwrap();

    assert_eq!(o.dispatcher.last_chat_list(), Some(ChatListKind::Folder(3)));
}

#[test]
fn folders_update_does_not_reload_chat_list() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General")]);
    let dispatches_before = o.dispatcher.chat_list_dispatch_count();

    push_folders(&mut o);

    assert_eq!(o.dispatcher.chat_list_dispatch_count(), dispatches_before);
}

fn push_only_family(o: &mut TestOrchestrator) {
    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::ChatFoldersChanged {
            folders: ChatFolders {
                folders: vec![ChatFolder {
                    id: 7,
                    title: "Family".to_owned(),
                }],
                main_list_position: 0,
            },
        }],
    })
    .unwrap();
}

#[test]
fn removing_active_folder_switches_to_main_list_and_reloads_it() {
    let mut o = orchestrator_with_folders();
    switch_to_work(&mut o);
    inject_folder_chat_list(&mut o, 3, vec![chat(5, "Team")]);
    let dispatches_before = o.dispatcher.chat_list_dispatch_count();

    push_only_family(&mut o);

    assert_eq!(o.state().chat_list().kind(), ChatListKind::Main);
    assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Loading);
    assert!(o.state().chat_list().chats().is_empty());
    assert_eq!(
        o.dispatcher.chat_list_dispatch_count(),
        dispatches_before + 1
    );
    assert_eq!(o.dispatcher.last_chat_list(), Some(ChatListKind::Main));
}

#[test]
fn removing_active_folder_while_its_load_is_in_flight_reloads_main_list() {
    let mut o = orchestrator_with_folders();
    switch_to_work(&mut o);
    let dispatches_before = o.dispatcher.chat_list_dispatch_count();

    push_only_family(&mut o);
    assert_eq!(
        o.dispatcher.chat_list_dispatch_count(),
        dispatches_before,
        "fallback must wait for the in-flight folder request"
    );

    inject_folder_chat_list(&mut o, 3, vec![chat(5, "Team")]);

    assert!(o.state().chat_list().chats().is_empty());
    assert_eq!(
        o.dispatcher.chat_list_dispatch_count(),
        dispatches_before + 1
    );
    assert_eq!(o.dispatcher.last_chat_list(), Some(ChatListKind::Main));
}

#[test]
fn removing_inactive_folder_keeps_active_folder() {
    let mut o = orchestrator_with_folders();
    let family = 7;
    press(&mut o, "f");
    press(&mut o, "j");
    press(&mut o, "j");
    press(&mut o, "enter");
    inject_folder_chat_list(&mut o, family, vec![chat(5, "Mom")]);
    let dispatches_before = o.dispatcher.chat_list_dispatch_count();

    push_only_family(&mut o);

    assert_eq!(o.state().chat_list().kind(), ChatListKind::Folder(family));
    assert_eq!(o.state().chat_list().chats().len(), 1);
    assert_eq!(o.dispatcher.chat_list_dispatch_count(), dispatches_before);
}

#[test]
fn folders_update_rebuilds_open_picker_without_removed_folder() {
    let mut o = orchestrator_with_folders();
    switch_to_work(&mut o);
    inject_folder_chat_list(&mut o, 3, vec![chat(5, "Team")]);
    press(&mut o, "f");

    push_only_family(&mut o);

    let picker = o
        .state()
        .chat_folder_picker()
        .expect("picker must stay open");
    let lists: Vec<ChatListKind> = picker.entries().iter().map(|entry| entry.list).collect();
    assert_eq!(
        lists,
        vec![
            ChatListKind::Main,
            ChatListKind::Folder(7),
            ChatListKind::Archive
        ]
    );
    assert_eq!(
        picker.selected().map(|entry| entry.list),
        Some(ChatListKind::Main)
    );
}

#[test]
fn folders_update_keeps_picker_cursor_on_surviving_entry() {
    let mut o = orchestrator_with_folders();
    press(&mut o, "f");
    press(&mut o, "j");
    press(&mut o, "j");

    push_only_family(&mut o);

    let picker = o.state().chat_folder_picker().unwrap();
    assert_eq!(
        picker.selected().map(|entry| entry.list),
        Some(ChatListKind::Folder(7))
    );
}

#[test]
fn removing_all_folders_keeps_picker_with_main_list_and_archive() {
    let mut o = orchestrator_with_folders();
    press(&mut o, "f");

    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::ChatFoldersChanged {
            folders: ChatFolders::default(),
        }],
    })
    .unwrap();

    let picker = o
        .state()
        .chat_folder_picker()
        .expect("picker must stay open");
    let lists: Vec<ChatListKind> = picker.entries().iter().map(|entry| entry.list).collect();
    assert_eq!(lists, vec![ChatListKind::Main, ChatListKind::Archive]);
}

#[test]
fn folders_update_keeps_archive_without_reload() {
    let mut o = orchestrator_with_folders();
    press(&mut o, "f");
    for _ in 0..3 {
        press(&mut o, "j");
    }
    press(&mut o, "enter");
    o.handle_event(AppEvent::BackgroundTasksCompleted(vec![
        BackgroundTaskResult::ChatListLoaded {
            list: ChatListKind::Archive,
            result: Ok(vec![chat(5, "Old")]),
            all_loaded: true,
        },
    ]))
    .unwrap();
    let dispatches_before = o.dispatcher.chat_list_dispatch_count();

    push_only_family(&mut o);

    assert_eq!(o.state().chat_list().kind(), ChatListKind::Archive);
    assert_eq!(o.state().chat_list().chats().len(), 1);
    assert_eq!(o.dispatcher.chat_list_dispatch_count(), dispatches_before);
}

#[test]
fn unread_chat_count_update_reaches_an_open_picker() {
    let mut o = make_orchestrator();
    push_folders(&mut o);
    press(&mut o, "f");

    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![
            ChatUpdate::UnreadChatCountChanged {
                list: ChatListKind::Archive,
                unread_count: 3,
            },
            ChatUpdate::UnreadChatCountChanged {
                list: ChatListKind::Folder(7),
                unread_count: 1,
            },
        ],
    })
    .unwrap();

    assert!(o.state().chat_folder_picker().is_some());
    let counts = o.state().unread_chat_counts();
    assert_eq!(counts.get(&ChatListKind::Archive), Some(&3));
    assert_eq!(counts.get(&ChatListKind::Folder(7)), Some(&1));
    assert_eq!(counts.get(&ChatListKind::Main), None);
}
