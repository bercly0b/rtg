use super::*;

fn orchestrator_in_messages_pane(
    chats: Vec<ChatSummary>,
    chat_id: i64,
    messages: Vec<Message>,
) -> TestOrchestrator {
    orchestrator_with_open_chat(chats, chat_id, messages)
}

#[test]
fn i_key_opens_message_info_popup_when_message_selected() {
    let mut o =
        orchestrator_in_messages_pane(vec![chat(1, "Alice")], 1, vec![message(10, "hello")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("I", false)))
        .unwrap();
    assert!(o.state().message_info_popup().is_some());
    assert_eq!(o.state().message_info_popup().unwrap().ids(), Some((1, 10)));
}

#[test]
fn i_key_does_nothing_when_no_message_selected() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alice")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    // Chat is open but no messages loaded yet — I should be no-op
    o.handle_event(AppEvent::InputKey(KeyInput::new("I", false)))
        .unwrap();
    assert!(o.state().message_info_popup().is_none());
}

#[test]
fn message_info_popup_closes_on_esc() {
    let mut o =
        orchestrator_in_messages_pane(vec![chat(1, "Alice")], 1, vec![message(10, "hello")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("I", false)))
        .unwrap();
    assert!(o.state().message_info_popup().is_some());

    o.handle_event(AppEvent::InputKey(KeyInput::new("esc", false)))
        .unwrap();
    assert!(o.state().message_info_popup().is_none());
}

#[test]
fn message_info_popup_closes_on_q() {
    let mut o =
        orchestrator_in_messages_pane(vec![chat(1, "Alice")], 1, vec![message(10, "hello")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("I", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("q", false)))
        .unwrap();
    assert!(o.state().message_info_popup().is_none());
}

#[test]
fn message_info_popup_closes_on_second_i() {
    let mut o =
        orchestrator_in_messages_pane(vec![chat(1, "Alice")], 1, vec![message(10, "hello")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("I", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("I", false)))
        .unwrap();
    assert!(o.state().message_info_popup().is_none());
}

#[test]
fn message_info_popup_ignores_other_keys() {
    let mut o =
        orchestrator_in_messages_pane(vec![chat(1, "Alice")], 1, vec![message(10, "hello")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("I", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    assert!(o.state().message_info_popup().is_some());
}

#[test]
fn message_info_loaded_updates_popup_state() {
    use crate::domain::message_info_state::{MessageInfo, MessageInfoPopupState, ReactionDetail};

    let mut o =
        orchestrator_in_messages_pane(vec![chat(1, "Alice")], 1, vec![message(10, "hello")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("I", false)))
        .unwrap();

    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessageInfoLoaded {
            chat_id: 1,
            message_id: 10,
            result: Ok(MessageInfo {
                reactions: vec![ReactionDetail {
                    emoji: "👍".to_owned(),
                    sender_name: "Bob".to_owned(),
                }],
                viewers: vec![],
                read_date: None,
                edit_date: Some(1700000000),
            }),
        },
    ))
    .unwrap();

    match o.state().message_info_popup().unwrap() {
        MessageInfoPopupState::Loaded(info) => {
            assert_eq!(info.reactions.len(), 1);
            assert_eq!(info.reactions[0].emoji, "👍");
            assert_eq!(info.reactions[0].sender_name, "Bob");
            assert_eq!(info.edit_date, Some(1700000000));
        }
        _ => panic!("expected Loaded state"),
    }
}

#[test]
fn message_info_loaded_error_sets_error_state() {
    use crate::domain::message_info_state::MessageInfoPopupState;

    let mut o =
        orchestrator_in_messages_pane(vec![chat(1, "Alice")], 1, vec![message(10, "hello")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("I", false)))
        .unwrap();

    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessageInfoLoaded {
            chat_id: 1,
            message_id: 10,
            result: Err(BackgroundError::new("MESSAGE_INFO_UNAVAILABLE")),
        },
    ))
    .unwrap();

    assert!(matches!(
        o.state().message_info_popup().unwrap(),
        MessageInfoPopupState::Error
    ));
}

#[test]
fn message_info_loaded_ignored_when_popup_closed() {
    use crate::domain::message_info_state::MessageInfo;

    let mut o =
        orchestrator_in_messages_pane(vec![chat(1, "Alice")], 1, vec![message(10, "hello")]);

    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessageInfoLoaded {
            chat_id: 1,
            message_id: 10,
            result: Ok(MessageInfo {
                reactions: vec![],
                viewers: vec![],
                read_date: None,
                edit_date: None,
            }),
        },
    ))
    .unwrap();

    assert!(o.state().message_info_popup().is_none());
}

#[test]
fn message_info_loaded_stale_result_ignored() {
    use crate::domain::message_info_state::{MessageInfo, MessageInfoPopupState};

    let mut o = orchestrator_in_messages_pane(
        vec![chat(1, "Alice")],
        1,
        vec![message(10, "hello"), message(20, "world")],
    );

    o.handle_event(AppEvent::InputKey(KeyInput::new("I", false)))
        .unwrap();
    assert_eq!(o.state().message_info_popup().unwrap().ids(), Some((1, 20)));

    // Stale result for message 10 arrives while popup is open for message 20
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessageInfoLoaded {
            chat_id: 1,
            message_id: 10,
            result: Ok(MessageInfo {
                reactions: vec![],
                viewers: vec![],
                read_date: None,
                edit_date: None,
            }),
        },
    ))
    .unwrap();

    // Popup should still be in Loading state for message 20
    match o.state().message_info_popup().unwrap() {
        MessageInfoPopupState::Loading {
            chat_id,
            message_id,
        } => {
            assert_eq!(*chat_id, 1);
            assert_eq!(*message_id, 20);
        }
        _ => panic!("expected Loading state for message 20"),
    }
}
