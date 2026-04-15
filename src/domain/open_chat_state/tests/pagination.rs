use super::*;

fn ready_state_with_messages(messages: Vec<Message>) -> OpenChatState {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Test".to_owned(), ChatType::Private);
    state.set_ready(messages);
    state
}

#[test]
fn needs_more_messages_false_when_empty() {
    let state = OpenChatState::default();
    assert!(!state.needs_more_messages());
}

#[test]
fn needs_more_messages_false_when_all_loaded() {
    let msgs: Vec<Message> = (1..=10).map(|i| message(i, "msg")).collect();
    let mut state = ready_state_with_messages(msgs);
    state.set_all_messages_loaded(true);
    state.selected_index = Some(0);
    assert!(!state.needs_more_messages());
}

#[test]
fn needs_more_messages_true_when_near_top() {
    let msgs: Vec<Message> = (1..=20).map(|i| message(i, "msg")).collect();
    let mut state = ready_state_with_messages(msgs);
    state.selected_index = Some(3);
    assert!(state.needs_more_messages());
}

#[test]
fn needs_more_messages_false_when_far_from_top() {
    let msgs: Vec<Message> = (1..=20).map(|i| message(i, "msg")).collect();
    let mut state = ready_state_with_messages(msgs);
    state.selected_index = Some(10);
    assert!(!state.needs_more_messages());
}

#[test]
fn needs_more_messages_true_at_index_zero() {
    let msgs: Vec<Message> = (1..=10).map(|i| message(i, "msg")).collect();
    let mut state = ready_state_with_messages(msgs);
    state.selected_index = Some(0);
    assert!(state.needs_more_messages());
}

#[test]
fn oldest_message_id_returns_first() {
    let msgs = vec![message(100, "old"), message(200, "new")];
    let state = ready_state_with_messages(msgs);
    assert_eq!(state.oldest_message_id(), Some(100));
}

#[test]
fn oldest_message_id_none_when_empty() {
    let state = OpenChatState::default();
    assert_eq!(state.oldest_message_id(), None);
}

#[test]
fn prepend_older_messages_adds_before_existing() {
    let msgs = vec![message(10, "ten"), message(20, "twenty")];
    let mut state = ready_state_with_messages(msgs);
    state.selected_index = Some(1);
    state.scroll_offset = ScrollOffset { item: 0, line: 0 };

    let older = vec![message(1, "one"), message(5, "five")];
    state.prepend_older_messages(older);

    assert_eq!(state.messages().len(), 4);
    assert_eq!(state.messages()[0].id, 1);
    assert_eq!(state.messages()[1].id, 5);
    assert_eq!(state.messages()[2].id, 10);
    assert_eq!(state.messages()[3].id, 20);
    assert_eq!(state.selected_index(), Some(3));
    assert_eq!(state.scroll_offset().item, 2);
}

#[test]
fn prepend_empty_sets_all_loaded() {
    let msgs = vec![message(1, "first")];
    let mut state = ready_state_with_messages(msgs);
    assert!(!state.all_messages_loaded());

    state.prepend_older_messages(vec![]);

    assert!(state.all_messages_loaded());
    assert_eq!(state.messages().len(), 1);
}

#[test]
fn set_loading_resets_all_messages_loaded() {
    let msgs = vec![message(1, "msg")];
    let mut state = ready_state_with_messages(msgs);
    state.set_all_messages_loaded(true);
    assert!(state.all_messages_loaded());

    state.set_loading(2, "New".to_owned(), ChatType::Private);
    assert!(!state.all_messages_loaded());
}
