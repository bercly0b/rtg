use super::*;
use crate::domain::{
    chat_folder::{ChatFolder, ChatFolders, ARCHIVE_CHAT_LIST_TITLE},
    events::ChatUpdate,
};

fn press(o: &mut TestOrchestrator, key: &str) {
    o.handle_event(AppEvent::InputKey(KeyInput::new(key, false)))
        .unwrap();
}

fn archived_chat(chat_id: i64, title: &str) -> ChatSummary {
    ChatSummary {
        is_archived: true,
        ..chat(chat_id, title)
    }
}

fn inject_list(o: &mut TestOrchestrator, list: ChatListKind, chats: Vec<ChatSummary>) {
    o.handle_event(AppEvent::BackgroundTasksCompleted(vec![
        BackgroundTaskResult::ChatListLoaded {
            list,
            result: Ok(chats),
            all_loaded: true,
        },
    ]))
    .unwrap();
}

fn inject_add_chat_to_list_failed(o: &mut TestOrchestrator, chat_id: i64, list: ChatListKind) {
    o.handle_event(AppEvent::BackgroundTasksCompleted(vec![
        BackgroundTaskResult::AddChatToListFailed { chat_id, list },
    ]))
    .unwrap();
}

fn chat_ids(o: &TestOrchestrator) -> Vec<i64> {
    o.state()
        .chat_list()
        .chats()
        .iter()
        .map(|c| c.chat_id)
        .collect()
}

fn selected_chat_id(o: &TestOrchestrator) -> Option<i64> {
    o.state().chat_list().selected_chat().map(|c| c.chat_id)
}

/// Without folders the picker holds "All chats" and "Archive".
fn switch_to_archive(o: &mut TestOrchestrator, chats: Vec<ChatSummary>) {
    press(o, "f");
    press(o, "j");
    press(o, "enter");
    inject_list(o, ChatListKind::Archive, chats);
}

fn switch_to_work_folder(o: &mut TestOrchestrator, chats: Vec<ChatSummary>) {
    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::ChatFoldersChanged {
            folders: ChatFolders {
                folders: vec![ChatFolder {
                    id: 3,
                    title: "Work".to_owned(),
                }],
                main_list_position: 0,
            },
        }],
    })
    .unwrap();
    press(o, "f");
    press(o, "j");
    press(o, "enter");
    inject_list(o, ChatListKind::Folder(3), chats);
}

// ── Entering the archive ──

#[test]
fn f_without_folders_opens_picker_with_main_list_and_archive() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General")]);

    press(&mut o, "f");

    let picker = o.state().chat_folder_picker().expect("picker must be open");
    let lists: Vec<ChatListKind> = picker.entries().iter().map(|e| e.list).collect();
    assert_eq!(lists, vec![ChatListKind::Main, ChatListKind::Archive]);
    assert_eq!(o.state().active_notification(), None);
}

#[test]
fn picking_archive_loads_archive_list_with_archive_title() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General")]);

    press(&mut o, "f");
    press(&mut o, "j");
    press(&mut o, "enter");

    assert_eq!(o.state().chat_list().kind(), ChatListKind::Archive);
    assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Loading);
    assert_eq!(o.dispatcher.last_chat_list(), Some(ChatListKind::Archive));
    assert_eq!(
        o.state().active_chat_list_title(),
        Some(ARCHIVE_CHAT_LIST_TITLE)
    );

    inject_list(&mut o, ChatListKind::Archive, vec![archived_chat(9, "Old")]);

    assert_eq!(chat_ids(&o), vec![9]);
}

#[test]
fn empty_archive_opens_as_empty_list() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General")]);

    switch_to_archive(&mut o, vec![]);

    assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Empty);
}

#[test]
fn picking_main_list_leaves_archive() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General")]);
    switch_to_archive(&mut o, vec![archived_chat(9, "Old")]);

    press(&mut o, "f");
    press(&mut o, "k");
    press(&mut o, "enter");

    assert_eq!(o.state().chat_list().kind(), ChatListKind::Main);
    assert_eq!(o.dispatcher.last_chat_list(), Some(ChatListKind::Main));
}

// ── Archiving and unarchiving ──

#[test]
fn shift_a_in_main_list_archives_chat_and_drops_its_row() {
    let mut o =
        orchestrator_with_chats(vec![chat(1, "General"), chat(2, "Backend"), chat(3, "Ops")]);
    press(&mut o, "j");
    let chat_list_dispatches = o.dispatcher.chat_list_dispatch_count();

    press(&mut o, "A");

    assert_eq!(
        o.dispatcher.add_chat_to_list_dispatches(),
        vec![(2, ChatListKind::Archive)]
    );
    assert_eq!(chat_ids(&o), vec![1, 3]);
    assert_eq!(selected_chat_id(&o), Some(3));
    assert_eq!(o.state().active_notification(), Some("Chat archived"));
    assert_eq!(
        o.dispatcher.chat_list_dispatch_count(),
        chat_list_dispatches,
        "TDLib's position updates drive the reload, not the toggle"
    );
}

#[test]
fn archiving_the_only_chat_leaves_main_list_empty() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General")]);

    press(&mut o, "A");

    assert_eq!(o.state().chat_list().ui_state(), ChatListUiState::Empty);
    assert_eq!(selected_chat_id(&o), None);
}

#[test]
fn archiving_prefetches_the_chat_the_cursor_lands_on() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General"), chat(2, "Backend")]);

    press(&mut o, "A");

    assert_eq!(o.dispatcher.last_prefetch_chat_id(), Some(2));
}

#[test]
fn shift_a_in_archive_unarchives_chat_and_drops_its_row() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General")]);
    switch_to_archive(
        &mut o,
        vec![archived_chat(8, "Old"), archived_chat(9, "Older")],
    );

    press(&mut o, "A");

    assert_eq!(
        o.dispatcher.add_chat_to_list_dispatches(),
        vec![(8, ChatListKind::Main)]
    );
    assert_eq!(chat_ids(&o), vec![9]);
    assert_eq!(selected_chat_id(&o), Some(9));
    assert_eq!(o.state().active_notification(), Some("Chat unarchived"));
}

#[test]
fn shift_a_in_folder_follows_chat_state_and_keeps_row() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General")]);
    switch_to_work_folder(&mut o, vec![chat(5, "Team"), archived_chat(6, "Old team")]);

    press(&mut o, "A");
    press(&mut o, "j");
    press(&mut o, "A");

    assert_eq!(
        o.dispatcher.add_chat_to_list_dispatches(),
        vec![(5, ChatListKind::Archive), (6, ChatListKind::Main)]
    );
    assert_eq!(chat_ids(&o), vec![5, 6]);
    assert_eq!(o.state().active_notification(), Some("Chat unarchived"));
}

#[test]
fn repeated_shift_a_in_folder_undoes_the_toggle() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General")]);
    switch_to_work_folder(&mut o, vec![chat(5, "Team")]);

    press(&mut o, "A");
    press(&mut o, "A");

    assert_eq!(
        o.dispatcher.add_chat_to_list_dispatches(),
        vec![(5, ChatListKind::Archive), (5, ChatListKind::Main)]
    );
    assert!(!o.state().chat_list().chats()[0].is_archived);
}

#[test]
fn shift_a_without_chats_does_nothing() {
    let mut o = orchestrator_with_chats(vec![]);

    press(&mut o, "A");

    assert!(o.dispatcher.add_chat_to_list_dispatches().is_empty());
    assert_eq!(o.state().active_notification(), None);
}

#[test]
fn failed_archive_notifies_and_reloads_chat_list() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General"), chat(2, "Backend")]);
    press(&mut o, "A");
    let chat_list_dispatches = o.dispatcher.chat_list_dispatch_count();

    inject_add_chat_to_list_failed(&mut o, 1, ChatListKind::Archive);

    assert_eq!(
        o.state().active_notification(),
        Some("Failed to archive chat")
    );
    assert_eq!(
        o.dispatcher.chat_list_dispatch_count(),
        chat_list_dispatches + 1
    );
    assert_eq!(o.dispatcher.last_chat_list(), Some(ChatListKind::Main));

    inject_chat_list(&mut o, vec![chat(1, "General"), chat(2, "Backend")]);
    assert_eq!(chat_ids(&o), vec![1, 2]);
}

#[test]
fn failed_unarchive_notifies_and_reloads_archive() {
    let mut o = orchestrator_with_chats(vec![chat(1, "General")]);
    switch_to_archive(&mut o, vec![archived_chat(8, "Old")]);
    press(&mut o, "A");

    inject_add_chat_to_list_failed(&mut o, 8, ChatListKind::Main);

    assert_eq!(
        o.state().active_notification(),
        Some("Failed to unarchive chat")
    );
    assert_eq!(o.dispatcher.last_chat_list(), Some(ChatListKind::Archive));
}
