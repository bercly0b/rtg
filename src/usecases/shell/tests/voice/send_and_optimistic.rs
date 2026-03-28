use super::*;

#[test]
fn send_voice_skipped_when_no_file_path() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);

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

    o.discard_voice_recording();

    assert!(o.recording_file_path.is_none());
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

// -- Full e2e flows --

#[test]
fn full_voice_flow_record_stop_send() {
    use crate::domain::command_popup_state::CommandPhase;

    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hi")]);

    let tmp = std::env::temp_dir().join("rtg_test_full_flow.oga");
    std::fs::write(&tmp, b"fake audio").unwrap();
    let file_path = tmp.to_str().unwrap().to_owned();

    simulate_voice_recording_started(&mut o, &file_path);
    assert!(o.state().command_popup().is_some());

    o.handle_event(AppEvent::CommandOutputLine {
        text: "frame=1".into(),
        replace_last: false,
    })
    .unwrap();

    o.handle_event(AppEvent::InputKey(KeyInput::new("q", false)))
        .unwrap();
    assert_eq!(
        o.state().command_popup().unwrap().phase(),
        &CommandPhase::Stopping
    );

    o.handle_event(AppEvent::CommandExited { success: true })
        .unwrap();
    assert!(matches!(
        o.state().command_popup().unwrap().phase(),
        CommandPhase::AwaitingConfirmation { .. }
    ));

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

    o.handle_event(AppEvent::InputKey(KeyInput::new("q", false)))
        .unwrap();
    assert_eq!(
        o.state().command_popup().unwrap().phase(),
        &CommandPhase::Stopping
    );

    o.handle_event(AppEvent::CommandExited { success: true })
        .unwrap();
    assert!(matches!(
        o.state().command_popup().unwrap().phase(),
        CommandPhase::AwaitingConfirmation { .. }
    ));

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

    o.handle_event(AppEvent::CommandExited { success: true })
        .unwrap();
    assert!(matches!(
        o.state().command_popup().unwrap().phase(),
        CommandPhase::AwaitingConfirmation { .. }
    ));

    o.handle_event(AppEvent::InputKey(KeyInput::new("y", false)))
        .unwrap();
    assert!(o.state().command_popup().is_none());
    assert_eq!(o.dispatcher.voice_send_dispatch_count(), 1);

    let _ = std::fs::remove_file(&tmp);
}

// -- Optimistic voice message tests --

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

    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::VoiceSendFailed { chat_id: 1 },
    ))
    .unwrap();

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

    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::VoiceSendFailed { chat_id: 999 },
    ))
    .unwrap();

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

    o.handle_event(AppEvent::InputKey(KeyInput::new("q", false)))
        .unwrap();
    assert_eq!(
        o.state().command_popup().unwrap().phase(),
        &CommandPhase::Stopping
    );

    o.handle_event(AppEvent::CommandExited { success: true })
        .unwrap();

    assert_eq!(o.state().open_chat().messages().len(), 1);

    o.handle_event(AppEvent::InputKey(KeyInput::new("y", false)))
        .unwrap();
    assert!(o.state().command_popup().is_none());
    assert_eq!(o.dispatcher.voice_send_dispatch_count(), 1);

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
