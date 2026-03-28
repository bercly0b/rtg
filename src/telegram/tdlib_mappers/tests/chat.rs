use crate::telegram::tdlib_mappers::map_chat_to_summary;

#[test]
fn chat_summary_maps_unread_reaction_count() {
    let mut td_chat = crate::telegram::tdlib_cache::tests::make_test_chat(1, "Test");
    td_chat.unread_reaction_count = 5;

    let summary = map_chat_to_summary(&td_chat, None, None, false);
    assert_eq!(summary.unread_reaction_count, 5);
}

#[test]
fn chat_summary_maps_zero_unread_reaction_count() {
    let td_chat = crate::telegram::tdlib_cache::tests::make_test_chat(1, "Test");

    let summary = map_chat_to_summary(&td_chat, None, None, false);
    assert_eq!(summary.unread_reaction_count, 0);
}
