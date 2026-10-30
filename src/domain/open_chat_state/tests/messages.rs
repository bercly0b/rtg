use super::*;

// ── update_messages tests ──

#[test]
fn update_messages_clears_refreshing_and_sets_live_source() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![message(1, "A")]);
    state.set_refreshing(true);
    state.set_message_source(MessageSource::Cache);

    state.update_messages(vec![message(1, "A"), message(2, "B")]);

    assert!(!state.is_refreshing());
    assert_eq!(state.message_source(), MessageSource::Live);
}

#[test]
fn update_messages_preserves_selection_by_message_id() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![message(1, "A"), message(2, "B"), message(3, "C")]);

    // Select message 2 (index 1)
    state.selected_index = Some(1);
    let saved_offset = ScrollOffset { item: 2, line: 3 };
    state.set_scroll_offset(saved_offset);

    // Update with reordered messages — message 2 is now at index 2
    state.update_messages(vec![message(4, "D"), message(1, "A"), message(2, "B")]);

    assert_eq!(state.selected_index(), Some(2));
    assert_eq!(state.ui_state(), OpenChatUiState::Ready);
    // Scroll offset preserved (not reset)
    assert_eq!(state.scroll_offset(), saved_offset);
}

#[test]
fn update_messages_falls_back_to_last_when_selected_message_disappears() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![message(1, "A"), message(2, "B"), message(3, "C")]);

    // Select message 3 (index 2)
    assert_eq!(state.selected_index(), Some(2));

    // Update without message 3
    state.update_messages(vec![message(1, "A"), message(2, "B")]);

    // Should fall back to last message (index 1)
    assert_eq!(state.selected_index(), Some(1));
    assert_eq!(state.scroll_offset(), ScrollOffset::BOTTOM);
}

#[test]
fn update_messages_handles_empty_replacement() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![message(1, "A"), message(2, "B")]);

    state.update_messages(vec![]);

    assert_eq!(state.selected_index(), None);
    assert_eq!(state.scroll_offset(), ScrollOffset::ZERO);
    assert_eq!(state.ui_state(), OpenChatUiState::Ready);
}

#[test]
fn update_messages_on_empty_state_with_new_messages() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![]);

    state.update_messages(vec![message(1, "A"), message(2, "B")]);

    // No previous selection, so falls back to last message
    assert_eq!(state.selected_index(), Some(1));
    assert_eq!(state.scroll_offset(), ScrollOffset::BOTTOM);
    assert_eq!(state.ui_state(), OpenChatUiState::Ready);
}

#[test]
fn update_messages_preserves_selection_same_position() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![message(1, "A"), message(2, "B")]);

    // Selected last message (index 1, id 2)
    assert_eq!(state.selected_index(), Some(1));
    let saved_offset = ScrollOffset { item: 1, line: 0 };
    state.set_scroll_offset(saved_offset);

    // Update with same messages + one new one at the end
    state.update_messages(vec![message(1, "A"), message(2, "B"), message(3, "C")]);

    // Message 2 is still at index 1
    assert_eq!(state.selected_index(), Some(1));
    // Scroll offset preserved
    assert_eq!(state.scroll_offset(), saved_offset);
}

// ── pending message tests ──

#[test]
fn add_pending_message_appends_and_selects_it() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![message(1, "A"), message(2, "B")]);

    state.add_pending_message(
        "Hello".to_owned(),
        crate::domain::message::MessageMedia::None,
        None,
    );

    assert_eq!(state.messages().len(), 3);
    let pending = &state.messages()[2];
    assert_eq!(pending.text, "Hello");
    assert!(pending.is_outgoing);
    assert_eq!(
        pending.status,
        crate::domain::message::MessageStatus::Sending
    );
    assert_eq!(pending.media, crate::domain::message::MessageMedia::None);
    assert_eq!(state.selected_index(), Some(2));
    assert_eq!(state.scroll_offset(), ScrollOffset::BOTTOM);
}

#[test]
fn add_pending_message_returns_unique_negative_ids() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![message(1, "A")]);

    let text_id = state.add_pending_message(
        "Text".to_owned(),
        crate::domain::message::MessageMedia::None,
        None,
    );
    let voice_id = state.add_pending_message(
        String::new(),
        crate::domain::message::MessageMedia::Voice,
        None,
    );

    assert!(text_id < 0 && voice_id < 0);
    assert_ne!(text_id, voice_id);
    assert_eq!(state.messages()[1].id, text_id);
    assert_eq!(state.messages()[2].id, voice_id);
}

#[test]
fn pending_ids_stay_unique_across_chat_switch() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![message(1, "A")]);
    let first_chat_id = state.add_pending_message(
        "In chat 1".to_owned(),
        crate::domain::message::MessageMedia::None,
        None,
    );

    state.set_loading(2, "Other".to_owned(), ChatType::Private);
    state.set_ready(vec![message(5, "B")]);
    let second_chat_id = state.add_pending_message(
        "In chat 2".to_owned(),
        crate::domain::message::MessageMedia::None,
        None,
    );

    assert_ne!(first_chat_id, second_chat_id);
    state.remove_message(first_chat_id);
    assert_eq!(state.messages().len(), 2);
}

#[test]
fn removing_pending_message_keeps_other_pending_and_delivered() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![message(1, "A"), message(2, "B")]);

    let failed_id = state.add_pending_message(
        "Failed".to_owned(),
        crate::domain::message::MessageMedia::None,
        None,
    );
    state.add_pending_message(
        String::new(),
        crate::domain::message::MessageMedia::Voice,
        None,
    );

    state.remove_message(failed_id);

    let texts: Vec<_> = state.messages().iter().map(|m| m.text.as_str()).collect();
    assert_eq!(texts, vec!["A", "B", ""]);
    assert_eq!(
        state.messages()[2].media,
        crate::domain::message::MessageMedia::Voice
    );
}

#[test]
fn removing_pending_message_fixes_selection() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![message(1, "A")]);

    let pending_id = state.add_pending_message(
        "Pending".to_owned(),
        crate::domain::message::MessageMedia::None,
        None,
    );
    assert_eq!(state.selected_index(), Some(1)); // pending message selected

    state.remove_message(pending_id);

    // Selection should clamp to last remaining message
    assert_eq!(state.selected_index(), Some(0));
}

#[test]
fn set_ready_replaces_pending_messages() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![message(1, "A")]);

    state.add_pending_message(
        "Pending".to_owned(),
        crate::domain::message::MessageMedia::None,
        None,
    );
    assert_eq!(state.messages().len(), 2);

    // Server refresh replaces everything including pending
    state.set_ready(vec![message(1, "A"), message(3, "Pending delivered")]);

    assert_eq!(state.messages().len(), 2);
    assert_eq!(state.messages()[1].text, "Pending delivered");
    assert_eq!(
        state.messages()[1].status,
        crate::domain::message::MessageStatus::Delivered
    );
}

#[test]
fn add_pending_message_with_voice_media() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![message(1, "A")]);

    state.add_pending_message(
        String::new(),
        crate::domain::message::MessageMedia::Voice,
        None,
    );

    assert_eq!(state.messages().len(), 2);
    let pending = &state.messages()[1];
    assert_eq!(pending.text, "");
    assert_eq!(pending.media, crate::domain::message::MessageMedia::Voice);
    assert_eq!(
        pending.status,
        crate::domain::message::MessageStatus::Sending
    );
    assert!(pending.is_outgoing);
    assert!(pending.is_pending());
    assert_eq!(state.selected_index(), Some(1));
    assert_eq!(state.scroll_offset(), ScrollOffset::BOTTOM);
}

// ── remove_message tests ──

#[test]
fn remove_message_removes_by_id() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![message(1, "A"), message(2, "B"), message(3, "C")]);

    state.remove_message(2);

    assert_eq!(state.messages().len(), 2);
    assert_eq!(state.messages()[0].id, 1);
    assert_eq!(state.messages()[1].id, 3);
}

#[test]
fn remove_message_adjusts_selection_when_last_removed() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![message(1, "A"), message(2, "B")]);
    // Selection defaults to last (index 1, id=2)

    state.remove_message(2);

    // Selection should clamp to new last (index 0)
    assert_eq!(state.selected_message().unwrap().id, 1);
}

#[test]
fn remove_message_clears_selection_when_empty() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![message(1, "A")]);

    state.remove_message(1);

    assert!(state.messages().is_empty());
    assert!(state.selected_message().is_none());
}

#[test]
fn remove_message_ignores_unknown_id() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![message(1, "A"), message(2, "B")]);

    state.remove_message(999);

    assert_eq!(state.messages().len(), 2);
}

// ── Reaction count update tests ──

#[test]
fn update_message_reaction_count_modifies_existing() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![message(1, "A"), message(2, "B")]);

    state.update_message_reaction_count(2, 7);

    assert_eq!(state.messages()[0].reaction_count, 0);
    assert_eq!(state.messages()[1].reaction_count, 7);
}

#[test]
fn update_message_reaction_count_ignores_unknown_id() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![message(1, "A")]);

    state.update_message_reaction_count(999, 3);

    assert_eq!(state.messages()[0].reaction_count, 0);
}
