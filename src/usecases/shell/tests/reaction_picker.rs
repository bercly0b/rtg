use super::*;

fn orchestrator_in_messages_pane(
    chats: Vec<ChatSummary>,
    chat_id: i64,
    messages: Vec<Message>,
) -> TestOrchestrator {
    orchestrator_with_open_chat(chats, chat_id, messages)
}

#[test]
fn r_key_opens_reaction_picker_loading() {
    let mut o =
        orchestrator_in_messages_pane(vec![chat(1, "Alice")], 1, vec![message(10, "hello")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("R", false)))
        .unwrap();
    assert!(o.state().reaction_picker().is_some());
    assert_eq!(o.state().reaction_picker().unwrap().ids(), Some((1, 10)));
}

#[test]
fn r_key_does_nothing_when_no_message_selected() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alice")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("R", false)))
        .unwrap();
    assert!(o.state().reaction_picker().is_none());
}

#[test]
fn reaction_picker_closes_on_esc() {
    let mut o =
        orchestrator_in_messages_pane(vec![chat(1, "Alice")], 1, vec![message(10, "hello")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("R", false)))
        .unwrap();
    assert!(o.state().reaction_picker().is_some());

    o.handle_event(AppEvent::InputKey(KeyInput::new("esc", false)))
        .unwrap();
    assert!(o.state().reaction_picker().is_none());
}

#[test]
fn reaction_picker_closes_on_q() {
    let mut o =
        orchestrator_in_messages_pane(vec![chat(1, "Alice")], 1, vec![message(10, "hello")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("R", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("q", false)))
        .unwrap();
    assert!(o.state().reaction_picker().is_none());
}

#[test]
fn reaction_picker_closes_on_second_r() {
    let mut o =
        orchestrator_in_messages_pane(vec![chat(1, "Alice")], 1, vec![message(10, "hello")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("R", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("R", false)))
        .unwrap();
    assert!(o.state().reaction_picker().is_none());
}

#[test]
fn available_reactions_loaded_updates_picker_to_ready() {
    use crate::domain::reaction_picker_state::{AvailableReaction, ReactionPickerState};

    let mut o =
        orchestrator_in_messages_pane(vec![chat(1, "Alice")], 1, vec![message(10, "hello")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("R", false)))
        .unwrap();

    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::AvailableReactionsLoaded {
            chat_id: 1,
            message_id: 10,
            result: Ok(vec![
                AvailableReaction {
                    emoji: "👍".into(),
                    needs_premium: false,
                    is_chosen: false,
                },
                AvailableReaction {
                    emoji: "❤".into(),
                    needs_premium: false,
                    is_chosen: false,
                },
            ]),
        },
    ))
    .unwrap();

    match o.state().reaction_picker().unwrap() {
        ReactionPickerState::Ready(data) => {
            assert_eq!(data.items.len(), 2);
            assert_eq!(data.selected_index, 0);
        }
        _ => panic!("expected Ready state"),
    }
}

#[test]
fn available_reactions_loaded_error_sets_error_state() {
    use crate::domain::reaction_picker_state::ReactionPickerState;

    let mut o =
        orchestrator_in_messages_pane(vec![chat(1, "Alice")], 1, vec![message(10, "hello")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("R", false)))
        .unwrap();

    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::AvailableReactionsLoaded {
            chat_id: 1,
            message_id: 10,
            result: Err(BackgroundError::new("REACTIONS_UNAVAILABLE")),
        },
    ))
    .unwrap();

    assert!(matches!(
        o.state().reaction_picker().unwrap(),
        ReactionPickerState::Error
    ));
}

#[test]
fn empty_reactions_closes_picker_and_shows_notification() {
    let mut o =
        orchestrator_in_messages_pane(vec![chat(1, "Alice")], 1, vec![message(10, "hello")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("R", false)))
        .unwrap();

    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::AvailableReactionsLoaded {
            chat_id: 1,
            message_id: 10,
            result: Ok(vec![]),
        },
    ))
    .unwrap();

    assert!(o.state().reaction_picker().is_none());
    assert_eq!(
        o.state().active_notification(),
        Some("No reactions available")
    );
}

#[test]
fn j_k_navigates_in_ready_picker() {
    use crate::domain::reaction_picker_state::{AvailableReaction, ReactionPickerState};

    let mut o =
        orchestrator_in_messages_pane(vec![chat(1, "Alice")], 1, vec![message(10, "hello")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("R", false)))
        .unwrap();

    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::AvailableReactionsLoaded {
            chat_id: 1,
            message_id: 10,
            result: Ok(vec![
                AvailableReaction {
                    emoji: "👍".into(),
                    needs_premium: false,
                    is_chosen: false,
                },
                AvailableReaction {
                    emoji: "❤".into(),
                    needs_premium: false,
                    is_chosen: false,
                },
                AvailableReaction {
                    emoji: "🔥".into(),
                    needs_premium: false,
                    is_chosen: false,
                },
            ]),
        },
    ))
    .unwrap();

    // Navigate down
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    match o.state().reaction_picker().unwrap() {
        ReactionPickerState::Ready(data) => assert_eq!(data.selected_index, 1),
        _ => panic!("expected Ready"),
    }

    // Navigate down again
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    match o.state().reaction_picker().unwrap() {
        ReactionPickerState::Ready(data) => assert_eq!(data.selected_index, 2),
        _ => panic!("expected Ready"),
    }

    // Should stop at last
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    match o.state().reaction_picker().unwrap() {
        ReactionPickerState::Ready(data) => assert_eq!(data.selected_index, 2),
        _ => panic!("expected Ready"),
    }

    // Navigate up
    o.handle_event(AppEvent::InputKey(KeyInput::new("k", false)))
        .unwrap();
    match o.state().reaction_picker().unwrap() {
        ReactionPickerState::Ready(data) => assert_eq!(data.selected_index, 1),
        _ => panic!("expected Ready"),
    }
}

#[test]
fn enter_on_unchosen_reaction_dispatches_add_and_closes_picker() {
    use crate::domain::reaction_picker_state::AvailableReaction;

    let mut o =
        orchestrator_in_messages_pane(vec![chat(1, "Alice")], 1, vec![message(10, "hello")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("R", false)))
        .unwrap();

    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::AvailableReactionsLoaded {
            chat_id: 1,
            message_id: 10,
            result: Ok(vec![
                AvailableReaction {
                    emoji: "👍".into(),
                    needs_premium: false,
                    is_chosen: false,
                },
                AvailableReaction {
                    emoji: "❤".into(),
                    needs_premium: false,
                    is_chosen: false,
                },
            ]),
        },
    ))
    .unwrap();

    // Select the second reaction
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    assert!(o.state().reaction_picker().is_none());
    assert_eq!(
        o.dispatcher.last_add_reaction(),
        Some((1, 10, "❤".to_owned()))
    );
    assert_eq!(o.dispatcher.last_remove_reaction(), None);
}

#[test]
fn enter_on_chosen_reaction_dispatches_remove_and_closes_picker() {
    use crate::domain::reaction_picker_state::AvailableReaction;

    let mut o =
        orchestrator_in_messages_pane(vec![chat(1, "Alice")], 1, vec![message(10, "hello")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("R", false)))
        .unwrap();

    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::AvailableReactionsLoaded {
            chat_id: 1,
            message_id: 10,
            result: Ok(vec![
                AvailableReaction {
                    emoji: "🔥".into(),
                    needs_premium: false,
                    is_chosen: true,
                },
                AvailableReaction {
                    emoji: "❤".into(),
                    needs_premium: false,
                    is_chosen: false,
                },
            ]),
        },
    ))
    .unwrap();

    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();

    assert!(o.state().reaction_picker().is_none());
    assert_eq!(
        o.dispatcher.last_remove_reaction(),
        Some((1, 10, "🔥".to_owned()))
    );
    assert_eq!(o.dispatcher.last_add_reaction(), None);
}

#[test]
fn stale_reactions_result_ignored() {
    use crate::domain::reaction_picker_state::{AvailableReaction, ReactionPickerState};

    let mut o =
        orchestrator_in_messages_pane(vec![chat(1, "Alice")], 1, vec![message(10, "hello")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("R", false)))
        .unwrap();

    // Stale result for a different message
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::AvailableReactionsLoaded {
            chat_id: 1,
            message_id: 99,
            result: Ok(vec![AvailableReaction {
                emoji: "👍".into(),
                needs_premium: false,
                is_chosen: false,
            }]),
        },
    ))
    .unwrap();

    // Should still be in loading state for message 10
    match o.state().reaction_picker().unwrap() {
        ReactionPickerState::Loading {
            chat_id,
            message_id,
        } => {
            assert_eq!(*chat_id, 1);
            assert_eq!(*message_id, 10);
        }
        _ => panic!("expected Loading state"),
    }
}

#[test]
fn picker_blocks_other_message_keys() {
    let mut o =
        orchestrator_in_messages_pane(vec![chat(1, "Alice")], 1, vec![message(10, "hello")]);
    o.handle_event(AppEvent::InputKey(KeyInput::new("R", false)))
        .unwrap();

    // Press 'h' (back to chat list) while picker is open — should be ignored
    o.handle_event(AppEvent::InputKey(KeyInput::new("h", false)))
        .unwrap();
    assert!(o.state().reaction_picker().is_some());
    assert_eq!(o.state().active_pane(), ActivePane::Messages);
}
