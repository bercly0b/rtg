use crate::telegram::tdlib_mappers::map_chat_to_summary;

#[test]
fn chat_summary_uses_deleted_placeholder_when_title_empty() {
    let td_chat = crate::telegram::tdlib_cache::tests::make_test_chat(1, "");

    let summary = map_chat_to_summary(&td_chat, None, None, false, false);
    assert_eq!(summary.title, "Deleted");
}

#[test]
fn chat_summary_maps_unread_reaction_count() {
    let mut td_chat = crate::telegram::tdlib_cache::tests::make_test_chat(1, "Test");
    td_chat.unread_reaction_count = 5;

    let summary = map_chat_to_summary(&td_chat, None, None, false, false);
    assert_eq!(summary.unread_reaction_count, 5);
}

#[test]
fn chat_summary_maps_zero_unread_reaction_count() {
    let td_chat = crate::telegram::tdlib_cache::tests::make_test_chat(1, "Test");

    let summary = map_chat_to_summary(&td_chat, None, None, false, false);
    assert_eq!(summary.unread_reaction_count, 0);
}

#[test]
fn chat_summary_carries_is_forum_flag() {
    let td_chat = crate::telegram::tdlib_cache::tests::make_test_chat(1, "Topics");

    let with_topics = map_chat_to_summary(&td_chat, None, None, false, true);
    let without_topics = map_chat_to_summary(&td_chat, None, None, false, false);

    assert!(with_topics.is_forum);
    assert!(!without_topics.is_forum);
}
