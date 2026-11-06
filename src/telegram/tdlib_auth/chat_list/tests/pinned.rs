use tdlib_rs::types::{ChatListFolder, ChatPosition};

use super::*;

const FOLDER_ID: i32 = 3;

fn folder_list() -> ChatList {
    ChatList::Folder(ChatListFolder {
        chat_folder_id: FOLDER_ID,
    })
}

fn position(list: ChatList, is_pinned: bool) -> ChatPosition {
    ChatPosition {
        list,
        order: 100,
        is_pinned,
        source: None,
    }
}

/// Chat 1 is pinned only in the folder, chat 2 only in the main list; both
/// belong to both lists.
fn resolver_with_pins_in_different_lists() -> FakeResolver {
    let mut pinned_in_folder = make_test_chat(1, "Pinned in folder");
    pinned_in_folder.positions = vec![
        position(ChatList::Main, false),
        position(folder_list(), true),
    ];
    let mut pinned_in_main = make_test_chat(2, "Pinned in main");
    pinned_in_main.positions = vec![
        position(ChatList::Main, true),
        position(folder_list(), false),
    ];

    FakeResolver::new()
        .with_cached_chat(pinned_in_folder)
        .with_cached_chat(pinned_in_main)
}

fn pinned_ids(list: &ChatList) -> Vec<i64> {
    let resolver = resolver_with_pins_in_different_lists();
    build_summaries_from_ids(&resolver, list, vec![1, 2], NO_FORCE)
        .into_iter()
        .filter(|summary| summary.is_pinned)
        .map(|summary| summary.chat_id)
        .collect()
}

#[test]
fn folder_marks_only_chats_pinned_in_that_folder() {
    assert_eq!(pinned_ids(&folder_list()), vec![1]);
}

#[test]
fn main_list_marks_only_chats_pinned_in_main_list() {
    assert_eq!(pinned_ids(&MAIN_LIST), vec![2]);
}

#[test]
fn folder_ignores_pins_of_another_folder() {
    let other_folder = ChatList::Folder(ChatListFolder {
        chat_folder_id: FOLDER_ID + 1,
    });

    assert!(pinned_ids(&other_folder).is_empty());
}
