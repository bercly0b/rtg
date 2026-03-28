use super::*;

// ── Voice recording / command popup tests ──

/// Simulates the state after `start_voice_recording` succeeds:
/// opens the command popup and sets a recording file path.
/// Does NOT spawn an external process (recording_handle is None).
///
/// Use `simulate_voice_recording_with_process` when testing exit code paths.
fn simulate_voice_recording_started(o: &mut TestOrchestrator, file_path: &str) {
    o.state.open_command_popup(
        "Recording Voice",
        crate::domain::command_popup_state::CommandPopupKind::Recording,
    );
    o.recording_file_path = Some(file_path.to_owned());
}

/// Simulates voice recording with a real process for exit-code tests.
/// `success`: if true, spawns `true` (exit 0); if false, spawns `false` (exit 1).
fn simulate_voice_recording_with_process(o: &mut TestOrchestrator, file_path: &str, success: bool) {
    use std::process::Command;

    let cmd = if success { "true" } else { "false" };
    let child = Command::new(cmd)
        .spawn()
        .expect("failed to spawn test process");
    // Wait briefly for the short-lived process to exit.
    let mut handle = crate::usecases::voice_recording::RecordingHandle::from_child(child);
    std::thread::sleep(std::time::Duration::from_millis(50));
    // Ensure it actually exited so try_exit_success returns Some.
    let _ = handle.try_exit_success();

    o.state.open_command_popup(
        "Recording Voice",
        crate::domain::command_popup_state::CommandPopupKind::Recording,
    );
    o.recording_file_path = Some(file_path.to_owned());
    o.recording_handle = Some(handle);
}

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

    // Second v press should not panic or change state.
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

    // Press a navigation key — should be intercepted, not move selection.
    o.handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
        .unwrap();

    // Popup is still active, key was absorbed.
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

    // Create a real temp file so the file existence check passes.
    let tmp = std::env::temp_dir().join("rtg_test_send.oga");
    std::fs::write(&tmp, b"fake audio").unwrap();
    let file_path = tmp.to_str().unwrap().to_owned();

    simulate_voice_recording_started(&mut o, &file_path);

    // Transition to AwaitingConfirmation (as if q was pressed).
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

    // Clean up.
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

    // Popup should still be open in AwaitingConfirmation.
    let popup = o.state().command_popup().expect("popup still open");
    assert!(matches!(
        popup.phase(),
        CommandPhase::AwaitingConfirmation { .. }
    ));
    assert_eq!(o.dispatcher.voice_send_dispatch_count(), 0);
}

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

    // No popup active — should not panic.
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

    // User already pressed q — already in AwaitingConfirmation.
    let custom_prompt = "Send recording? (y/n)";
    o.state
        .command_popup_mut()
        .unwrap()
        .set_phase(CommandPhase::AwaitingConfirmation {
            prompt: custom_prompt.into(),
        });

    o.handle_event(AppEvent::CommandExited { success: true })
        .unwrap();

    // The prompt should not be overwritten.
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

    // No popup — should not panic.
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

    // Simulate q → Stopping (handle already None since simulate_voice_recording_started
    // doesn't set one, same as after the stop thread takes it).
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

    // All keys should be ignored during Stopping.
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
fn send_voice_skipped_when_no_file_path() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);

    // No recording_file_path set.
    o.send_voice_recording();

    assert_eq!(o.dispatcher.voice_send_dispatch_count(), 0);
}

#[test]
fn send_voice_skipped_when_file_does_not_exist() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
    o.recording_file_path = Some("/nonexistent/path/voice.oga".into());

    o.send_voice_recording();

    assert_eq!(o.dispatcher.voice_send_dispatch_count(), 0);
}

#[test]
fn send_voice_skipped_when_no_chat_open() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Chat")]);

    let tmp = std::env::temp_dir().join("rtg_test_nochat.oga");
    std::fs::write(&tmp, b"fake audio").unwrap();
    o.recording_file_path = Some(tmp.to_str().unwrap().into());

    o.send_voice_recording();

    assert_eq!(o.dispatcher.voice_send_dispatch_count(), 0);
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn send_voice_dispatches_with_correct_chat_id_and_path() {
    let mut o = orchestrator_with_open_chat(vec![chat(42, "Chat")], 42, vec![message(10, "hi")]);

    let tmp = std::env::temp_dir().join("rtg_test_dispatch.oga");
    std::fs::write(&tmp, b"fake audio").unwrap();
    let file_path = tmp.to_str().unwrap().to_owned();

    o.recording_file_path = Some(file_path.clone());
    o.send_voice_recording();

    assert_eq!(o.dispatcher.voice_send_dispatch_count(), 1);
    let (sent_chat_id, sent_path) = o.dispatcher.last_voice_send().unwrap();
    assert_eq!(sent_chat_id, 42);
    assert_eq!(sent_path, file_path);

    // Optimistic pending voice message should be visible
    let messages = o.state().open_chat().messages();
    let pending = messages.last().unwrap();
    assert_eq!(pending.media, crate::domain::message::MessageMedia::Voice);
    assert_eq!(
        pending.status,
        crate::domain::message::MessageStatus::Sending
    );

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn discard_voice_removes_file() {
    let mut o = make_orchestrator();

    let tmp = std::env::temp_dir().join("rtg_test_discard_file.oga");
    std::fs::write(&tmp, b"fake audio").unwrap();
    o.recording_file_path = Some(tmp.to_str().unwrap().into());

    o.discard_voice_recording();

    assert!(!tmp.exists());
    assert!(o.recording_file_path.is_none());
}

#[test]
fn discard_voice_no_op_when_no_file_path() {
    let mut o = make_orchestrator();

    // Should not panic when there's nothing to discard.
    o.discard_voice_recording();

    assert!(o.recording_file_path.is_none());
}

#[test]
fn full_voice_flow_record_stop_send() {
    use crate::domain::command_popup_state::CommandPhase;

    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);

    let tmp = std::env::temp_dir().join("rtg_test_full_flow.oga");
    std::fs::write(&tmp, b"fake audio").unwrap();
    let file_path = tmp.to_str().unwrap().to_owned();

    // Step 1: Simulate recording started.
    simulate_voice_recording_started(&mut o, &file_path);
    assert!(o.state().command_popup().is_some());

    // Step 2: Output lines arrive.
    o.handle_event(AppEvent::CommandOutputLine {
        text: "frame=1".into(),
        replace_last: false,
    })
    .unwrap();

    // Step 3: User presses q to stop → transitions to Stopping.
    o.handle_event(AppEvent::InputKey(KeyInput::new("q", false)))
        .unwrap();
    assert_eq!(
        o.state().command_popup().unwrap().phase(),
        &CommandPhase::Stopping
    );

    // Step 4: Process exits → transitions to AwaitingConfirmation.
    o.handle_event(AppEvent::CommandExited { success: true })
        .unwrap();
    assert!(matches!(
        o.state().command_popup().unwrap().phase(),
        CommandPhase::AwaitingConfirmation { .. }
    ));

    // Step 5: User presses y to send.
    o.handle_event(AppEvent::InputKey(KeyInput::new("y", false)))
        .unwrap();
    assert!(o.state().command_popup().is_none());
    assert_eq!(o.dispatcher.voice_send_dispatch_count(), 1);

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn full_voice_flow_record_stop_discard() {
    use crate::domain::command_popup_state::CommandPhase;

    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);

    let tmp = std::env::temp_dir().join("rtg_test_full_discard.oga");
    std::fs::write(&tmp, b"fake audio").unwrap();
    let file_path = tmp.to_str().unwrap().to_owned();

    simulate_voice_recording_started(&mut o, &file_path);

    // q to stop → Stopping.
    o.handle_event(AppEvent::InputKey(KeyInput::new("q", false)))
        .unwrap();
    assert_eq!(
        o.state().command_popup().unwrap().phase(),
        &CommandPhase::Stopping
    );

    // Process exits → AwaitingConfirmation.
    o.handle_event(AppEvent::CommandExited { success: true })
        .unwrap();
    assert!(matches!(
        o.state().command_popup().unwrap().phase(),
        CommandPhase::AwaitingConfirmation { .. }
    ));

    // n to discard.
    o.handle_event(AppEvent::InputKey(KeyInput::new("n", false)))
        .unwrap();
    assert!(o.state().command_popup().is_none());
    assert_eq!(o.dispatcher.voice_send_dispatch_count(), 0);
    assert!(!tmp.exists());
}

#[test]
fn full_voice_flow_command_exits_then_send() {
    use crate::domain::command_popup_state::CommandPhase;

    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);

    let tmp = std::env::temp_dir().join("rtg_test_exit_send.oga");
    std::fs::write(&tmp, b"fake audio").unwrap();
    let file_path = tmp.to_str().unwrap().to_owned();

    simulate_voice_recording_with_process(&mut o, &file_path, true);

    // Process exits on its own (success).
    o.handle_event(AppEvent::CommandExited { success: true })
        .unwrap();
    assert!(matches!(
        o.state().command_popup().unwrap().phase(),
        CommandPhase::AwaitingConfirmation { .. }
    ));

    // User confirms send.
    o.handle_event(AppEvent::InputKey(KeyInput::new("y", false)))
        .unwrap();
    assert!(o.state().command_popup().is_none());
    assert_eq!(o.dispatcher.voice_send_dispatch_count(), 1);

    let _ = std::fs::remove_file(&tmp);
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

    // Help should not be visible when command popup is active.
    assert!(!o.state().help_visible());

    // ? should be intercepted by command popup, not open help.
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

    // Any key should close the Failed popup.
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
fn send_voice_recording_is_idempotent() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);

    let tmp = std::env::temp_dir().join("rtg_test_idempotent.oga");
    std::fs::write(&tmp, b"fake audio").unwrap();
    o.recording_file_path = Some(tmp.to_str().unwrap().into());

    o.send_voice_recording();
    o.send_voice_recording();

    assert_eq!(o.dispatcher.voice_send_dispatch_count(), 1);
    let _ = std::fs::remove_file(&tmp);
}

// ── Optimistic voice message tests ──

#[test]
fn voice_send_creates_pending_message_with_voice_media() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);

    let tmp = std::env::temp_dir().join("rtg_test_voice_pending.oga");
    std::fs::write(&tmp, b"fake audio").unwrap();
    o.recording_file_path = Some(tmp.to_str().unwrap().into());

    o.send_voice_recording();

    let messages = o.state().open_chat().messages();
    assert_eq!(messages.len(), 2);
    let pending = &messages[1];
    assert_eq!(pending.text, "");
    assert_eq!(pending.media, crate::domain::message::MessageMedia::Voice);
    assert_eq!(
        pending.status,
        crate::domain::message::MessageStatus::Sending
    );
    assert!(pending.is_outgoing);
    assert_eq!(pending.id, 0);
    assert_eq!(
        o.state().open_chat().scroll_offset(),
        crate::domain::open_chat_state::ScrollOffset::BOTTOM
    );

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn voice_send_failed_removes_pending_message() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);

    let tmp = std::env::temp_dir().join("rtg_test_voice_fail.oga");
    std::fs::write(&tmp, b"fake audio").unwrap();
    o.recording_file_path = Some(tmp.to_str().unwrap().into());

    o.send_voice_recording();
    assert_eq!(o.state().open_chat().messages().len(), 2);

    // Simulate voice send failure
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::VoiceSendFailed { chat_id: 1 },
    ))
    .unwrap();

    // Pending message should be rolled back
    assert_eq!(o.state().open_chat().messages().len(), 1);
    assert_eq!(o.state().open_chat().messages()[0].text, "hi");
}

#[test]
fn voice_send_failed_ignored_for_different_chat() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);

    let tmp = std::env::temp_dir().join("rtg_test_voice_fail_other.oga");
    std::fs::write(&tmp, b"fake audio").unwrap();
    o.recording_file_path = Some(tmp.to_str().unwrap().into());

    o.send_voice_recording();
    assert_eq!(o.state().open_chat().messages().len(), 2);

    // Failure for a different chat should not affect current chat
    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::VoiceSendFailed { chat_id: 999 },
    ))
    .unwrap();

    // Pending message should remain
    assert_eq!(o.state().open_chat().messages().len(), 2);

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn voice_send_success_replaces_pending_with_real_message() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);

    let tmp = std::env::temp_dir().join("rtg_test_voice_replace.oga");
    std::fs::write(&tmp, b"fake audio").unwrap();
    o.recording_file_path = Some(tmp.to_str().unwrap().into());

    o.send_voice_recording();
    assert_eq!(o.state().open_chat().messages().len(), 2);

    // Simulate MessageSentRefreshCompleted with real message from server
    let mut voice_msg = message(99, "");
    voice_msg.media = crate::domain::message::MessageMedia::Voice;
    voice_msg.is_outgoing = true;

    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::MessageSentRefreshCompleted {
            chat_id: 1,
            result: Ok(vec![message(10, "hi"), voice_msg]),
        },
    ))
    .unwrap();

    let messages = o.state().open_chat().messages();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[1].id, 99);
    assert_eq!(
        messages[1].status,
        crate::domain::message::MessageStatus::Delivered
    );
    assert_eq!(
        messages[1].media,
        crate::domain::message::MessageMedia::Voice
    );

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn voice_send_no_pending_when_file_missing() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);
    o.recording_file_path = Some("/nonexistent/rtg_test_voice.oga".into());

    o.send_voice_recording();

    // File doesn't exist so no dispatch and no pending message
    assert_eq!(o.dispatcher.voice_send_dispatch_count(), 0);
    assert_eq!(o.state().open_chat().messages().len(), 1);
}

#[test]
fn full_voice_flow_creates_optimistic_message_on_confirm() {
    use crate::domain::command_popup_state::CommandPhase;

    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);

    let tmp = std::env::temp_dir().join("rtg_test_full_optimistic.oga");
    std::fs::write(&tmp, b"fake audio").unwrap();
    let file_path = tmp.to_str().unwrap().to_owned();

    simulate_voice_recording_started(&mut o, &file_path);

    // q → Stopping
    o.handle_event(AppEvent::InputKey(KeyInput::new("q", false)))
        .unwrap();
    assert_eq!(
        o.state().command_popup().unwrap().phase(),
        &CommandPhase::Stopping
    );

    // Process exits → AwaitingConfirmation
    o.handle_event(AppEvent::CommandExited { success: true })
        .unwrap();

    // No pending message yet
    assert_eq!(o.state().open_chat().messages().len(), 1);

    // y → send with optimistic message
    o.handle_event(AppEvent::InputKey(KeyInput::new("y", false)))
        .unwrap();
    assert!(o.state().command_popup().is_none());
    assert_eq!(o.dispatcher.voice_send_dispatch_count(), 1);

    // Pending voice message should be visible
    let messages = o.state().open_chat().messages();
    assert_eq!(messages.len(), 2);
    let pending = &messages[1];
    assert_eq!(pending.media, crate::domain::message::MessageMedia::Voice);
    assert_eq!(
        pending.status,
        crate::domain::message::MessageStatus::Sending
    );

    let _ = std::fs::remove_file(&tmp);
}
