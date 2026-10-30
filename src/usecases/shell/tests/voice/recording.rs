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

    let tmp_dir = tempfile::tempdir().unwrap();
    let tmp = tmp_dir.path().join("rtg_test_send.oga");
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
}

#[test]
fn command_popup_n_discards_voice_and_closes_popup() {
    use crate::domain::command_popup_state::CommandPhase;

    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);

    let tmp_dir = tempfile::tempdir().unwrap();
    let tmp = tmp_dir.path().join("rtg_test_discard.oga");
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

    let tmp_dir = tempfile::tempdir().unwrap();
    let tmp = tmp_dir.path().join("rtg_test_esc.oga");
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

#[test]
fn command_popup_q_stops_live_recording_process() {
    use crate::domain::command_popup_state::CommandPhase;

    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
    o.voice_record_cmd = "sleep 60".to_owned();

    o.handle_event(AppEvent::InputKey(KeyInput::new("v", false)))
        .unwrap();
    assert!(o.recording_handle.is_some(), "recording must have started");
    let rx = o
        .take_pending_command_rx()
        .expect("command events receiver");

    o.handle_event(AppEvent::InputKey(KeyInput::new("q", false)))
        .unwrap();
    assert!(
        o.recording_handle.is_none(),
        "handle must move to the stop thread"
    );
    assert_eq!(
        o.state().command_popup().unwrap().phase(),
        &CommandPhase::Stopping
    );

    let exited = wait_for_command_exit(&rx);
    assert_eq!(exited, AppEvent::CommandExited { success: false });

    o.handle_event(exited).unwrap();
    assert!(
        matches!(
            o.state().command_popup().unwrap().phase(),
            CommandPhase::Failed { .. }
        ),
        "`sleep` records nothing, so the stopped recording must fail"
    );
}

#[test]
fn v_with_missing_record_binary_shows_start_error() {
    use crate::domain::command_popup_state::CommandPhase;

    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
    o.voice_record_cmd = "rtg-test-missing-binary {file_path}".to_owned();

    o.handle_event(AppEvent::InputKey(KeyInput::new("v", false)))
        .unwrap();

    assert!(o.recording_handle.is_none());
    assert!(o.recording_file_path.is_none());
    let popup = o.state().command_popup().expect("failure must be visible");
    assert!(
        matches!(popup.phase(), CommandPhase::Failed { message } if message.contains("[voice] record_cmd")),
        "unexpected phase {:?}",
        popup.phase()
    );
    let lines = popup.visible_lines(20);
    assert!(
        lines
            .iter()
            .any(|line| line.contains("failed to start `rtg-test-missing-binary`")),
        "start error must be shown: {lines:?}"
    );
}
