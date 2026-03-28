use super::*;

#[test]
fn v_ignored_when_no_chat_open() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Chat")]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("v", false)))
        .unwrap();

    assert!(o.state().command_popup().is_none());
}

#[test]
fn v_ignored_when_popup_already_active() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
    simulate_voice_recording_started(&mut o, "/tmp/test.oga");

    o.handle_event(AppEvent::InputKey(KeyInput::new("v", false)))
        .unwrap();

    assert!(o.state().command_popup().is_some());
    assert_eq!(
        o.state().command_popup().unwrap().title(),
        "Recording Voice"
    );
}

#[test]
fn command_popup_intercepts_all_keys() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
    simulate_voice_recording_started(&mut o, "/tmp/test.oga");

    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();

    assert!(o.state().command_popup().is_some());
}

#[test]
fn command_popup_q_transitions_to_stopping() {
    use crate::domain::command_popup_state::CommandPhase;

    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
    simulate_voice_recording_started(&mut o, "/tmp/test.oga");

    o.handle_event(AppEvent::InputKey(KeyInput::new("q", false)))
        .unwrap();

    let popup = o
        .state()
        .command_popup()
        .expect("popup should still be open");
    assert_eq!(popup.phase(), &CommandPhase::Stopping);
}

#[test]
fn command_popup_random_key_during_running_is_ignored() {
    use crate::domain::command_popup_state::CommandPhase;

    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
    simulate_voice_recording_started(&mut o, "/tmp/test.oga");

    o.handle_event(AppEvent::InputKey(KeyInput::new("x", false)))
        .unwrap();

    let popup = o
        .state()
        .command_popup()
        .expect("popup should still be open");
    assert_eq!(popup.phase(), &CommandPhase::Running);
}

#[test]
fn command_popup_y_sends_voice_and_closes_popup() {
    use crate::domain::command_popup_state::CommandPhase;

    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);

    let tmp = std::env::temp_dir().join("rtg_test_send.oga");
    std::fs::write(&tmp, b"fake audio").unwrap();
    let file_path = tmp.to_str().unwrap().to_owned();

    simulate_voice_recording_started(&mut o, &file_path);

    o.state
        .command_popup_mut()
        .unwrap()
        .set_phase(CommandPhase::AwaitingConfirmation {
            prompt: "Send recording? (y/n)".into(),
        });

    o.handle_event(AppEvent::InputKey(KeyInput::new("y", false)))
        .unwrap();

    assert!(o.state().command_popup().is_none());
    assert_eq!(o.dispatcher.voice_send_dispatch_count(), 1);
    assert_eq!(o.dispatcher.last_voice_send().unwrap().0, 1);

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn command_popup_n_discards_voice_and_closes_popup() {
    use crate::domain::command_popup_state::CommandPhase;

    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);

    let tmp = std::env::temp_dir().join("rtg_test_discard.oga");
    std::fs::write(&tmp, b"fake audio").unwrap();
    let file_path = tmp.to_str().unwrap().to_owned();

    simulate_voice_recording_started(&mut o, &file_path);

    o.state
        .command_popup_mut()
        .unwrap()
        .set_phase(CommandPhase::AwaitingConfirmation {
            prompt: "Send recording? (y/n)".into(),
        });

    o.handle_event(AppEvent::InputKey(KeyInput::new("n", false)))
        .unwrap();

    assert!(o.state().command_popup().is_none());
    assert_eq!(o.dispatcher.voice_send_dispatch_count(), 0);
    assert!(!tmp.exists(), "file should be deleted on discard");
}

#[test]
fn command_popup_esc_discards_voice_and_closes_popup() {
    use crate::domain::command_popup_state::CommandPhase;

    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);

    let tmp = std::env::temp_dir().join("rtg_test_esc.oga");
    std::fs::write(&tmp, b"fake audio").unwrap();
    let file_path = tmp.to_str().unwrap().to_owned();

    simulate_voice_recording_started(&mut o, &file_path);

    o.state
        .command_popup_mut()
        .unwrap()
        .set_phase(CommandPhase::AwaitingConfirmation {
            prompt: "Send recording? (y/n)".into(),
        });

    o.handle_event(AppEvent::InputKey(KeyInput::new("esc", false)))
        .unwrap();

    assert!(o.state().command_popup().is_none());
    assert_eq!(o.dispatcher.voice_send_dispatch_count(), 0);
    assert!(!tmp.exists(), "file should be deleted on esc");
}

#[test]
fn command_popup_random_key_during_awaiting_is_ignored() {
    use crate::domain::command_popup_state::CommandPhase;

    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
    simulate_voice_recording_started(&mut o, "/tmp/test.oga");

    o.state
        .command_popup_mut()
        .unwrap()
        .set_phase(CommandPhase::AwaitingConfirmation {
            prompt: "Send? (y/n)".into(),
        });

    o.handle_event(AppEvent::InputKey(KeyInput::new("x", false)))
        .unwrap();

    let popup = o.state().command_popup().expect("popup still open");
    assert!(matches!(
        popup.phase(),
        CommandPhase::AwaitingConfirmation { .. }
    ));
    assert_eq!(o.dispatcher.voice_send_dispatch_count(), 0);
}
