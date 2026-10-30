use super::*;

#[test]
fn take_unacked_message_ids_returns_all_loaded_ids_first() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![message(10, "A"), message(20, "B")]);

    assert_eq!(state.take_unacked_message_ids(), vec![10, 20]);
}

#[test]
fn take_unacked_message_ids_returns_only_newer_ids_after_reload() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![message(10, "A"), message(20, "B")]);
    state.take_unacked_message_ids();

    state.update_messages(vec![
        message(5, "Older"),
        message(10, "A"),
        message(20, "B"),
        message(30, "C"),
    ]);

    assert_eq!(state.take_unacked_message_ids(), vec![30]);
    assert!(state.take_unacked_message_ids().is_empty());
}

#[test]
fn take_unacked_message_ids_skips_pending_messages() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![message(10, "A")]);
    state.take_unacked_message_ids();

    state.add_pending_message(
        "Sending".to_owned(),
        crate::domain::message::MessageMedia::None,
        None,
    );

    assert!(state.take_unacked_message_ids().is_empty());
}

#[test]
fn switching_chat_via_loading_resets_acked_id() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![message(10, "A"), message(20, "B")]);
    state.take_unacked_message_ids();

    state.set_loading(2, "Other".to_owned(), ChatType::Private);
    state.set_ready(vec![message(10, "C")]);

    assert_eq!(state.take_unacked_message_ids(), vec![10]);
}

#[test]
fn switching_chat_via_identity_resets_acked_id() {
    let mut state = OpenChatState::default();
    state.set_loading(1, "Chat".to_owned(), ChatType::Private);
    state.set_ready(vec![message(10, "A"), message(20, "B")]);
    state.take_unacked_message_ids();

    state.set_chat_identity(2, "Other".to_owned(), ChatType::Private);
    state.set_ready(vec![message(10, "C")]);

    assert_eq!(state.take_unacked_message_ids(), vec![10]);
}

#[test]
fn opening_topic_resets_acked_id() {
    let mut state = OpenChatState::default();
    state.set_loading_with_topic(1, Some(7), "Forum > A".to_owned(), ChatType::Group);
    state.set_ready(vec![message(10, "A"), message(20, "B")]);
    state.take_unacked_message_ids();

    state.set_loading_with_topic(1, Some(8), "Forum > B".to_owned(), ChatType::Group);
    state.set_ready(vec![message(15, "C")]);

    assert_eq!(state.take_unacked_message_ids(), vec![15]);
}
