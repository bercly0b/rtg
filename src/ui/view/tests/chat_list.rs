use super::{super::chat_list, chat, chat_with_pinned};

const TEST_WIDTH: usize = 50;

#[test]
fn build_chat_list_items_creates_all_chats_section_for_regular_chats() {
    let chats = vec![chat(1, "General", 0, Some("Hello"))];
    let items = chat_list::build_chat_list_items(&chats, TEST_WIDTH);

    assert_eq!(items.len(), 2);
}

#[test]
fn build_chat_list_items_creates_both_sections_when_pinned_exists() {
    let chats = vec![
        chat_with_pinned(1, "Pinned Chat", 0, Some("Hi"), true),
        chat(2, "Regular Chat", 0, Some("Hello")),
    ];
    let items = chat_list::build_chat_list_items(&chats, TEST_WIDTH);

    assert_eq!(items.len(), 4);
}

#[test]
fn compute_visual_index_accounts_for_headers() {
    let chats = vec![
        chat_with_pinned(1, "Pinned", 0, None, true),
        chat(2, "Regular", 0, None),
    ];

    assert_eq!(chat_list::compute_visual_index(&chats, 0), 1);
    assert_eq!(chat_list::compute_visual_index(&chats, 1), 3);
}

#[test]
fn compute_visual_index_with_no_pinned() {
    let chats = vec![chat(1, "Chat1", 0, None), chat(2, "Chat2", 0, None)];

    assert_eq!(chat_list::compute_visual_index(&chats, 0), 1);
    assert_eq!(chat_list::compute_visual_index(&chats, 1), 2);
}

#[test]
fn compute_visual_index_with_all_pinned() {
    let chats = vec![
        chat_with_pinned(1, "Pinned1", 0, None, true),
        chat_with_pinned(2, "Pinned2", 0, None, true),
    ];

    assert_eq!(chat_list::compute_visual_index(&chats, 0), 1);
    assert_eq!(chat_list::compute_visual_index(&chats, 1), 2);
}

#[test]
fn build_chat_list_items_shows_all_chats_header_when_all_pinned() {
    let chats = vec![
        chat_with_pinned(1, "Pinned1", 0, None, true),
        chat_with_pinned(2, "Pinned2", 0, None, true),
    ];
    let items = chat_list::build_chat_list_items(&chats, TEST_WIDTH);

    assert_eq!(items.len(), 3);
}
