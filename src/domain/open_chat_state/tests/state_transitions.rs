use super::*;

#[test]
fn default_state_is_empty() {
    let state = OpenChatState::default();

    assert_eq!(state.ui_state(), OpenChatUiState::Empty);
    assert!(!state.is_open());
    assert!(state.messages().is_empty());
    assert_eq!(state.selected_index(), None);
    assert!(!state.is_refreshing());
    assert_eq!(state.message_source(), MessageSource::None);
}

#[test]
fn set_loading_transitions_correctly() {
    let mut state = OpenChatState::default();

    state.set_loading(42, "Test Chat".to_owned(), ChatType::Private);

    assert_eq!(state.chat_id(), Some(42));
    assert_eq!(state.chat_title(), "Test Chat");
    assert_eq!(state.chat_type(), ChatType::Private);
    assert_eq!(state.ui_state(), OpenChatUiState::Loading);
    assert_eq!(state.selected_index(), None);
    assert!(!state.is_refreshing());
    assert_eq!(state.message_source(), MessageSource::None);
}

#[test]
fn set_loading_stores_chat_type() {
    let mut state = OpenChatState::default();

    state.set_loading(1, "Group".to_owned(), ChatType::Group);
    assert_eq!(state.chat_type(), ChatType::Group);

    state.set_loading(2, "Channel".to_owned(), ChatType::Channel);
    assert_eq!(state.chat_type(), ChatType::Channel);
}

#[test]
fn clear_resets_chat_type() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Group".to_owned(), ChatType::Group);

    state.clear();

    assert_eq!(state.chat_type(), ChatType::Private);
}

#[test]
fn set_loading_resets_refreshing_and_source() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![message(1, "A")]);
    state.set_refreshing(true);
    state.set_message_source(MessageSource::Cache);

    state.set_loading(2, "Other Chat".to_owned(), ChatType::Private);

    assert!(!state.is_refreshing());
    assert_eq!(state.message_source(), MessageSource::None);
}

#[test]
fn set_ready_stores_messages_and_selects_last() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);

    state.set_ready(vec![message(1, "Hello"), message(2, "World")]);

    assert_eq!(state.ui_state(), OpenChatUiState::Ready);
    assert_eq!(state.messages().len(), 2);
    assert_eq!(state.selected_index(), Some(1));
}

#[test]
fn set_ready_with_empty_messages_has_no_selection() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);

    state.set_ready(vec![]);

    assert_eq!(state.ui_state(), OpenChatUiState::Ready);
    assert!(state.messages().is_empty());
    assert_eq!(state.selected_index(), None);
    assert_eq!(state.scroll_offset(), ScrollOffset::ZERO);
}

#[test]
fn set_error_transitions_to_error() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);

    state.set_error();

    assert_eq!(state.ui_state(), OpenChatUiState::Error);
}

#[test]
fn set_error_resets_refreshing_and_source() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![message(1, "A")]);
    state.set_refreshing(true);
    state.set_message_source(MessageSource::Cache);

    state.set_error();

    assert!(!state.is_refreshing());
    assert_eq!(state.message_source(), MessageSource::None);
}

#[test]
fn clear_resets_to_empty() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![message(1, "Hi")]);
    state.set_refreshing(true);
    state.set_message_source(MessageSource::Cache);

    state.clear();

    assert_eq!(state.ui_state(), OpenChatUiState::Empty);
    assert!(!state.is_open());
    assert!(state.messages().is_empty());
    assert_eq!(state.selected_index(), None);
    assert!(!state.is_refreshing());
    assert_eq!(state.message_source(), MessageSource::None);
}
