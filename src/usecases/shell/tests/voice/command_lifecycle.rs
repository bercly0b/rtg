use super::*;

#[test]
fn command_output_line_event_pushes_to_popup() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
    simulate_voice_recording_started(&mut o, "/tmp/test.oga");

    o.handle_event(AppEvent::CommandOutputLine {
        text: "recording at 48kHz".into(),
        replace_last: false,
    })
    .unwrap();
    o.handle_event(AppEvent::CommandOutputLine {
        text: "size=128kB".into(),
        replace_last: false,
    })
    .unwrap();

    let popup = o.state().command_popup().unwrap();
    assert_eq!(
        popup.visible_lines(20),
        vec!["recording at 48kHz", "size=128kB"]
    );
}

#[test]
fn command_output_line_event_replaces_last_line_when_requested() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
    simulate_voice_recording_started(&mut o, "/tmp/test.oga");

    o.handle_event(AppEvent::CommandOutputLine {
        text: "A: 00:00:01 / 00:00:03".into(),
        replace_last: true,
    })
    .unwrap();
    o.handle_event(AppEvent::CommandOutputLine {
        text: "A: 00:00:02 / 00:00:03".into(),
        replace_last: true,
    })
    .unwrap();

    let popup = o.state().command_popup().unwrap();
    assert_eq!(popup.visible_lines(20), vec!["A: 00:00:02 / 00:00:03"]);
}

#[test]
fn command_output_line_ignored_when_no_popup() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);

    o.handle_event(AppEvent::CommandOutputLine {
        text: "stray line".into(),
        replace_last: false,
    })
    .unwrap();
}

#[test]
fn command_exited_transitions_running_to_awaiting_on_success() {
    use crate::domain::command_popup_state::CommandPhase;

    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
    simulate_voice_recording_with_process(&mut o, "/tmp/test.oga", true);

    o.handle_event(AppEvent::CommandExited { success: true })
        .unwrap();

    let popup = o
        .state()
        .command_popup()
        .expect("popup should still be open");
    assert!(
        matches!(popup.phase(), CommandPhase::AwaitingConfirmation { .. }),
        "expected AwaitingConfirmation but got {:?}",
        popup.phase()
    );
    assert!(o.recording_handle.is_none());
}

#[test]
fn command_exited_does_not_overwrite_awaiting_phase() {
    use crate::domain::command_popup_state::CommandPhase;

    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
    simulate_voice_recording_started(&mut o, "/tmp/test.oga");

    let custom_prompt = "Send recording? (y/n)";
    o.state
        .command_popup_mut()
        .unwrap()
        .set_phase(CommandPhase::AwaitingConfirmation {
            prompt: custom_prompt.into(),
        });

    o.handle_event(AppEvent::CommandExited { success: true })
        .unwrap();

    let popup = o.state().command_popup().unwrap();
    match popup.phase() {
        CommandPhase::AwaitingConfirmation { prompt } => {
            assert_eq!(prompt, custom_prompt);
        }
        _ => panic!("expected AwaitingConfirmation"),
    }
}

#[test]
fn command_exited_ignored_when_no_popup() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);

    o.handle_event(AppEvent::CommandExited { success: false })
        .unwrap();
}

#[test]
fn stopping_phase_transitions_to_awaiting_when_file_exists() {
    use crate::domain::command_popup_state::CommandPhase;

    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);

    let tmp = std::env::temp_dir().join("rtg_test_stopping_ok.oga");
    std::fs::write(&tmp, b"fake audio").unwrap();
    let file_path = tmp.to_str().unwrap().to_owned();

    simulate_voice_recording_started(&mut o, &file_path);

    o.state
        .command_popup_mut()
        .unwrap()
        .set_phase(CommandPhase::Stopping);

    o.handle_event(AppEvent::CommandExited { success: true })
        .unwrap();

    assert!(matches!(
        o.state().command_popup().unwrap().phase(),
        CommandPhase::AwaitingConfirmation { .. }
    ));

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn stopping_phase_transitions_to_failed_when_file_missing() {
    use crate::domain::command_popup_state::CommandPhase;

    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
    simulate_voice_recording_started(&mut o, "/nonexistent/rtg_test.oga");

    o.state
        .command_popup_mut()
        .unwrap()
        .set_phase(CommandPhase::Stopping);

    o.handle_event(AppEvent::CommandExited { success: true })
        .unwrap();

    assert!(matches!(
        o.state().command_popup().unwrap().phase(),
        CommandPhase::Failed { .. }
    ));
    assert!(
        o.recording_file_path.is_none(),
        "failed recording should discard file path"
    );
}

#[test]
fn stopping_phase_transitions_to_failed_when_file_empty() {
    use crate::domain::command_popup_state::CommandPhase;

    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);

    let tmp = std::env::temp_dir().join("rtg_test_stopping_empty.oga");
    std::fs::write(&tmp, b"").unwrap();
    let file_path = tmp.to_str().unwrap().to_owned();

    simulate_voice_recording_started(&mut o, &file_path);

    o.state
        .command_popup_mut()
        .unwrap()
        .set_phase(CommandPhase::Stopping);

    o.handle_event(AppEvent::CommandExited { success: true })
        .unwrap();

    assert!(matches!(
        o.state().command_popup().unwrap().phase(),
        CommandPhase::Failed { .. }
    ));

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn stopping_phase_ignores_keys() {
    use crate::domain::command_popup_state::CommandPhase;

    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
    simulate_voice_recording_started(&mut o, "/tmp/test.oga");

    o.state
        .command_popup_mut()
        .unwrap()
        .set_phase(CommandPhase::Stopping);

    for key in ["q", "y", "n", "x", "esc"] {
        o.handle_event(AppEvent::InputKey(KeyInput::new(key, false)))
            .unwrap();
        assert_eq!(
            o.state().command_popup().unwrap().phase(),
            &CommandPhase::Stopping,
            "key '{key}' should not change Stopping phase"
        );
    }
}

#[test]
fn command_exited_with_failure_transitions_to_failed() {
    use crate::domain::command_popup_state::CommandPhase;

    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
    simulate_voice_recording_with_process(&mut o, "/tmp/test.oga", false);

    o.handle_event(AppEvent::CommandExited { success: false })
        .unwrap();

    let popup = o
        .state()
        .command_popup()
        .expect("popup should still be open");
    assert!(
        matches!(popup.phase(), CommandPhase::Failed { .. }),
        "expected Failed phase but got {:?}",
        popup.phase()
    );
    assert!(o.recording_handle.is_none());
    assert!(
        o.recording_file_path.is_none(),
        "failed recording should discard file path"
    );
}

#[test]
fn failed_phase_any_key_closes_popup() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
    simulate_voice_recording_with_process(&mut o, "/tmp/test.oga", false);

    o.handle_event(AppEvent::CommandExited { success: false })
        .unwrap();

    o.handle_event(AppEvent::InputKey(KeyInput::new("x", false)))
        .unwrap();
    assert!(o.state().command_popup().is_none());
}

#[test]
fn failed_phase_message_mentions_config() {
    use crate::domain::command_popup_state::CommandPhase;

    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
    simulate_voice_recording_with_process(&mut o, "/tmp/test.oga", false);

    o.handle_event(AppEvent::CommandExited { success: false })
        .unwrap();

    let popup = o.state().command_popup().unwrap();
    match popup.phase() {
        CommandPhase::Failed { message } => {
            assert!(
                message.contains("config.toml"),
                "message should mention config.toml: {message}"
            );
            assert!(
                message.contains("[voice]"),
                "message should mention [voice] section: {message}"
            );
        }
        other => panic!("expected Failed but got {other:?}"),
    }
}

#[test]
fn take_pending_command_rx_returns_none_by_default() {
    let mut o = make_orchestrator();
    assert!(o.take_pending_command_rx().is_none());
}

#[test]
fn take_pending_command_rx_returns_receiver_once() {
    let mut o = make_orchestrator();
    let (_, rx) = std::sync::mpsc::channel::<crate::domain::events::CommandEvent>();
    o.pending_command_rx = Some(rx);

    assert!(o.take_pending_command_rx().is_some());
    assert!(o.take_pending_command_rx().is_none());
}

#[test]
fn help_popup_not_affected_by_command_popup() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
    simulate_voice_recording_started(&mut o, "/tmp/test.oga");

    assert!(!o.state().help_visible());

    o.handle_event(AppEvent::InputKey(KeyInput::new("?", false)))
        .unwrap();
    assert!(!o.state().help_visible());
}

#[test]
fn quit_requested_during_recording_stops_app() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
    simulate_voice_recording_started(&mut o, "/tmp/test.oga");

    o.handle_event(AppEvent::QuitRequested).unwrap();

    assert!(!o.state().is_running());
}
