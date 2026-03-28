use super::*;

#[test]
fn l_on_voice_message_opens_playback_popup() {
    use crate::domain::command_popup_state::CommandPopupKind;

    let tmp = std::env::temp_dir().join("rtg_test_playback.ogg");
    std::fs::write(&tmp, b"fake").unwrap();
    let path = tmp.to_str().unwrap();

    let mut o = orchestrator_with_open_chat(
        vec![chat(1, "Chat")],
        1,
        vec![voice_message_downloaded(10, path)],
    );
    o.open_handlers
        .insert("audio/ogg".to_owned(), "true".to_owned());

    o.handle_event(AppEvent::InputKey(KeyInput::new("l", false)))
        .unwrap();

    let popup = o.state().command_popup().expect("popup should be open");
    assert_eq!(popup.title(), "Playing");
    assert_eq!(popup.kind(), CommandPopupKind::Playback);

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn l_on_not_downloaded_voice_does_not_open_popup() {
    let mut o = orchestrator_with_open_chat(
        vec![chat(1, "Chat")],
        1,
        vec![voice_message_not_downloaded(10)],
    );

    o.handle_event(AppEvent::InputKey(KeyInput::new("l", false)))
        .unwrap();

    assert!(o.state().command_popup().is_none());
}

#[test]
fn l_on_text_message_does_nothing() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "hello")]);

    o.handle_event(AppEvent::InputKey(KeyInput::new("l", false)))
        .unwrap();

    assert!(o.state().command_popup().is_none());
}

#[test]
fn l_ignored_when_popup_already_open() {
    use crate::domain::command_popup_state::CommandPopupKind;

    let tmp = std::env::temp_dir().join("rtg_test_playback_dup.ogg");
    std::fs::write(&tmp, b"fake").unwrap();
    let path = tmp.to_str().unwrap();

    let mut o = orchestrator_with_open_chat(
        vec![chat(1, "Chat")],
        1,
        vec![voice_message_downloaded(10, path)],
    );
    o.open_handlers
        .insert("audio/ogg".to_owned(), "true".to_owned());

    o.state
        .open_command_popup("Other", CommandPopupKind::Recording);

    o.handle_event(AppEvent::InputKey(KeyInput::new("l", false)))
        .unwrap();

    assert_eq!(o.state().command_popup().unwrap().title(), "Other");

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn playback_popup_auto_closes_on_process_exit() {
    let tmp = std::env::temp_dir().join("rtg_test_play_autoclose.ogg");
    std::fs::write(&tmp, b"fake").unwrap();
    let path = tmp.to_str().unwrap();

    let mut o = orchestrator_with_open_chat(
        vec![chat(1, "Chat")],
        1,
        vec![voice_message_downloaded(10, path)],
    );
    o.open_handlers
        .insert("audio/ogg".to_owned(), "true".to_owned());

    o.handle_event(AppEvent::InputKey(KeyInput::new("l", false)))
        .unwrap();
    assert!(o.state().command_popup().is_some());

    o.handle_event(AppEvent::CommandExited { success: true })
        .unwrap();
    assert!(o.state().command_popup().is_none());

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn q_closes_playback_popup_immediately() {
    let tmp = std::env::temp_dir().join("rtg_test_play_q.ogg");
    std::fs::write(&tmp, b"fake").unwrap();
    let path = tmp.to_str().unwrap();

    let mut o = orchestrator_with_open_chat(
        vec![chat(1, "Chat")],
        1,
        vec![voice_message_downloaded(10, path)],
    );
    o.open_handlers
        .insert("audio/ogg".to_owned(), "true".to_owned());

    o.handle_event(AppEvent::InputKey(KeyInput::new("l", false)))
        .unwrap();
    assert!(o.state().command_popup().is_some());

    o.handle_event(AppEvent::InputKey(KeyInput::new("q", false)))
        .unwrap();
    assert!(o.state().command_popup().is_none());

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn l_on_audio_message_opens_playback_popup() {
    use crate::domain::command_popup_state::CommandPopupKind;

    let tmp = std::env::temp_dir().join("rtg_test_play_audio.mp3");
    std::fs::write(&tmp, b"fake").unwrap();
    let path = tmp.to_str().unwrap();

    let mut o = orchestrator_with_open_chat(
        vec![chat(1, "Chat")],
        1,
        vec![audio_message_downloaded(10, path)],
    );
    o.open_handlers
        .insert("audio/*".to_owned(), "true".to_owned());

    o.handle_event(AppEvent::InputKey(KeyInput::new("l", false)))
        .unwrap();

    let popup = o.state().command_popup().expect("popup should be open");
    assert_eq!(popup.kind(), CommandPopupKind::Playback);

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn voice_with_wildcard_handler_opens_playback_popup() {
    let tmp = std::env::temp_dir().join("rtg_test_play_wildcard.ogg");
    std::fs::write(&tmp, b"fake").unwrap();
    let path = tmp.to_str().unwrap();

    let mut o = orchestrator_with_open_chat(
        vec![chat(1, "Chat")],
        1,
        vec![voice_message_downloaded(10, path)],
    );
    o.open_handlers
        .insert("audio/*".to_owned(), "true".to_owned());

    o.handle_event(AppEvent::InputKey(KeyInput::new("l", false)))
        .unwrap();
    assert!(o.state().command_popup().is_some());

    let _ = std::fs::remove_file(&tmp);
}
