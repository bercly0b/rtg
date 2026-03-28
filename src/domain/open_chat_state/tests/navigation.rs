use super::*;

#[test]
fn select_next_moves_down_in_message_list() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![message(1, "A"), message(2, "B"), message(3, "C")]);

    // Initially at last message (index 2)
    assert_eq!(state.selected_index(), Some(2));

    // Move to beginning for testing
    state.selected_index = Some(0);

    state.select_next();
    assert_eq!(state.selected_index(), Some(1));

    state.select_next();
    assert_eq!(state.selected_index(), Some(2));

    // At the end, should stay at last
    state.select_next();
    assert_eq!(state.selected_index(), Some(2));
}

#[test]
fn select_previous_moves_up_in_message_list() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![message(1, "A"), message(2, "B"), message(3, "C")]);

    // Initially at last message (index 2)
    assert_eq!(state.selected_index(), Some(2));

    state.select_previous();
    assert_eq!(state.selected_index(), Some(1));

    state.select_previous();
    assert_eq!(state.selected_index(), Some(0));

    // At the beginning, should stay at first
    state.select_previous();
    assert_eq!(state.selected_index(), Some(0));
}

#[test]
fn select_next_on_empty_messages_does_nothing() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![]);

    state.select_next();

    assert_eq!(state.selected_index(), None);
}

#[test]
fn select_previous_on_empty_messages_does_nothing() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![]);

    state.select_previous();

    assert_eq!(state.selected_index(), None);
}

#[test]
fn select_next_initializes_to_first_when_no_selection() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![message(1, "A"), message(2, "B")]);
    state.selected_index = None; // Force no selection

    state.select_next();

    assert_eq!(state.selected_index(), Some(0));
}

#[test]
fn select_previous_initializes_to_last_when_no_selection() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![message(1, "A"), message(2, "B")]);
    state.selected_index = None; // Force no selection

    state.select_previous();

    assert_eq!(state.selected_index(), Some(1));
}

#[test]
fn scroll_offset_starts_at_zero() {
    let state = OpenChatState::default();
    assert_eq!(state.scroll_offset(), ScrollOffset::ZERO);
}

#[test]
fn scroll_offset_resets_on_set_loading() {
    let mut state = OpenChatState::default();
    state.scroll_offset = ScrollOffset { item: 5, line: 2 };

    state.set_loading(1, "Chat".to_owned(), ChatType::Private);

    assert_eq!(state.scroll_offset(), ScrollOffset::ZERO);
}

#[test]
fn set_ready_initializes_scroll_offset_to_bottom() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);

    state.set_ready(vec![message(1, "A"), message(2, "B")]);

    assert_eq!(state.scroll_offset(), ScrollOffset::BOTTOM);
}

#[test]
fn set_scroll_offset_persists_value() {
    let mut state = OpenChatState::default();
    let offset = ScrollOffset { item: 3, line: 1 };

    state.set_scroll_offset(offset);

    assert_eq!(state.scroll_offset(), offset);
}

#[test]
fn scroll_margin_constant_is_five() {
    assert_eq!(SCROLL_MARGIN, 5);
}

// ── selected_message tests ──

#[test]
fn selected_message_returns_message_at_selected_index() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![
        message(1, "First"),
        message(2, "Second"),
        message(3, "Third"),
    ]);

    // Initially selected = last (index 2)
    let msg = state.selected_message().unwrap();
    assert_eq!(msg.id, 3);
    assert_eq!(msg.text, "Third");
}

#[test]
fn selected_message_returns_none_when_empty() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![]);

    assert!(state.selected_message().is_none());
}

#[test]
fn selected_message_follows_navigation() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![message(1, "A"), message(2, "B")]);

    state.select_previous();
    let msg = state.selected_message().unwrap();
    assert_eq!(msg.id, 1);
    assert_eq!(msg.text, "A");
}
