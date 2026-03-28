use super::*;

// ── Message open / playback tests ──

fn voice_message_downloaded(id: i64, path: &str) -> Message {
    Message {
        id,
        sender_name: "User".to_owned(),
        text: String::new(),
        timestamp_ms: 1000,
        is_outgoing: false,
        media: crate::domain::message::MessageMedia::Voice,
        status: crate::domain::message::MessageStatus::Delivered,
        file_info: Some(crate::domain::message::FileInfo {
            file_id: id as i32,
            local_path: Some(path.to_owned()),
            mime_type: "audio/ogg".to_owned(),
            size: Some(1000),
            duration: Some(3),
            file_name: None,
            is_listened: false,
            download_status: crate::domain::message::DownloadStatus::Completed,
        }),
        call_info: None,
        reply_to: None,
        reaction_count: 0,
        links: Vec::new(),
        is_edited: false,
    }
}

fn voice_message_not_downloaded(id: i64) -> Message {
    Message {
        id,
        sender_name: "User".to_owned(),
        text: String::new(),
        timestamp_ms: 1000,
        is_outgoing: false,
        media: crate::domain::message::MessageMedia::Voice,
        status: crate::domain::message::MessageStatus::Delivered,
        file_info: Some(crate::domain::message::FileInfo {
            file_id: id as i32,
            local_path: None,
            mime_type: "audio/ogg".to_owned(),
            size: Some(1000),
            duration: Some(3),
            file_name: None,
            is_listened: false,
            download_status: crate::domain::message::DownloadStatus::NotStarted,
        }),
        call_info: None,
        reply_to: None,
        reaction_count: 0,
        links: Vec::new(),
        is_edited: false,
    }
}

fn audio_message_downloaded(id: i64, path: &str) -> Message {
    Message {
        id,
        sender_name: "User".to_owned(),
        text: String::new(),
        timestamp_ms: 1000,
        is_outgoing: false,
        media: crate::domain::message::MessageMedia::Audio,
        status: crate::domain::message::MessageStatus::Delivered,
        file_info: Some(crate::domain::message::FileInfo {
            file_id: id as i32,
            local_path: Some(path.to_owned()),
            mime_type: "audio/mpeg".to_owned(),
            size: Some(5000),
            duration: Some(180),
            file_name: None,
            is_listened: false,
            download_status: crate::domain::message::DownloadStatus::Completed,
        }),
        call_info: None,
        reply_to: None,
        reaction_count: 0,
        links: Vec::new(),
        is_edited: false,
    }
}

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
    // Configure a handler that runs `true` (a real command that exits immediately)
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

    // Open first popup
    o.state
        .open_command_popup("Other", CommandPopupKind::Recording);

    // l should be ignored since popup is already open
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

    // Process exits → popup should auto-close
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

    // q → popup should close immediately for playback
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

fn photo_message_downloaded(id: i64, path: &str) -> Message {
    Message {
        id,
        sender_name: "User".to_owned(),
        text: String::new(),
        timestamp_ms: 1000,
        is_outgoing: false,
        media: crate::domain::message::MessageMedia::Photo,
        status: crate::domain::message::MessageStatus::Delivered,
        file_info: Some(crate::domain::message::FileInfo {
            file_id: id as i32,
            local_path: Some(path.to_owned()),
            mime_type: "image/jpeg".to_owned(),
            size: Some(50_000),
            duration: None,
            file_name: None,
            is_listened: false,
            download_status: crate::domain::message::DownloadStatus::Completed,
        }),
        call_info: None,
        reply_to: None,
        reaction_count: 0,
        links: Vec::new(),
        is_edited: false,
    }
}

fn video_message_downloaded(id: i64, path: &str) -> Message {
    Message {
        id,
        sender_name: "User".to_owned(),
        text: String::new(),
        timestamp_ms: 1000,
        is_outgoing: false,
        media: crate::domain::message::MessageMedia::Video,
        status: crate::domain::message::MessageStatus::Delivered,
        file_info: Some(crate::domain::message::FileInfo {
            file_id: id as i32,
            local_path: Some(path.to_owned()),
            mime_type: "video/mp4".to_owned(),
            size: Some(10_000),
            duration: Some(30),
            file_name: None,
            is_listened: false,
            download_status: crate::domain::message::DownloadStatus::Completed,
        }),
        call_info: None,
        reply_to: None,
        reaction_count: 0,
        links: Vec::new(),
        is_edited: false,
    }
}

#[test]
fn l_on_photo_with_custom_handler_opens_playback_popup() {
    let tmp = std::env::temp_dir().join("rtg_test_photo_custom.jpg");
    std::fs::write(&tmp, b"fake").unwrap();
    let path = tmp.to_str().unwrap();

    let mut o = orchestrator_with_open_chat(
        vec![chat(1, "Chat")],
        1,
        vec![photo_message_downloaded(10, path)],
    );
    o.open_handlers
        .insert("image/*".to_owned(), "true {file_path}".to_owned());

    o.handle_event(AppEvent::InputKey(KeyInput::new("l", false)))
        .unwrap();

    let popup = o
        .state()
        .command_popup()
        .expect("playback popup should open for photo with custom handler");
    assert_eq!(popup.title(), "Playing");
    assert_eq!(
        popup.kind(),
        crate::domain::command_popup_state::CommandPopupKind::Playback
    );

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn l_on_photo_without_handler_dispatches_open_no_popup() {
    let tmp = std::env::temp_dir().join("rtg_test_photo_default.jpg");
    std::fs::write(&tmp, b"fake").unwrap();
    let path = tmp.to_str().unwrap();

    let mut o = orchestrator_with_open_chat(
        vec![chat(1, "Chat")],
        1,
        vec![photo_message_downloaded(10, path)],
    );
    // No handlers — falls back to platform default (open/xdg-open).
    // dispatch_open_file is called (no-op in RecordingDispatcher), no popup.

    o.handle_event(AppEvent::InputKey(KeyInput::new("l", false)))
        .unwrap();
    assert!(o.state().command_popup().is_none());

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn l_on_video_without_handler_dispatches_open_no_popup() {
    let tmp = std::env::temp_dir().join("rtg_test_video_default.mp4");
    std::fs::write(&tmp, b"fake").unwrap();
    let path = tmp.to_str().unwrap();

    let mut o = orchestrator_with_open_chat(
        vec![chat(1, "Chat")],
        1,
        vec![video_message_downloaded(10, path)],
    );
    // No handlers — falls back to platform default, no popup.

    o.handle_event(AppEvent::InputKey(KeyInput::new("l", false)))
        .unwrap();
    assert!(o.state().command_popup().is_none());

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn l_on_not_downloaded_file_shows_notification() {
    let mut o = orchestrator_with_open_chat(
        vec![chat(1, "Chat")],
        1,
        vec![voice_message_not_downloaded(10)],
    );

    o.handle_event(AppEvent::InputKey(KeyInput::new("l", false)))
        .unwrap();
    assert!(o.state().command_popup().is_none());
    assert!(o.state().active_notification().is_some());
}

#[test]
fn open_file_failed_shows_notification() {
    let mut o = make_orchestrator();

    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::OpenFileFailed {
            stderr: "No application knows how to open this file".to_owned(),
        },
    ))
    .unwrap();

    let notification = o
        .state()
        .active_notification()
        .expect("notification should be set");
    assert!(notification.contains("Open failed"));
}

#[test]
fn open_file_failed_empty_stderr_shows_config_hint() {
    let mut o = make_orchestrator();

    o.handle_event(AppEvent::BackgroundTaskCompleted(
        BackgroundTaskResult::OpenFileFailed {
            stderr: String::new(),
        },
    ))
    .unwrap();

    let notification = o
        .state()
        .active_notification()
        .expect("notification should be set");
    assert!(notification.contains("config.toml"));
}

#[test]
fn l_on_video_with_custom_handler_opens_playback_popup() {
    let tmp = std::env::temp_dir().join("rtg_test_video_mpv.mp4");
    std::fs::write(&tmp, b"fake").unwrap();
    let path = tmp.to_str().unwrap();

    let mut o = orchestrator_with_open_chat(
        vec![chat(1, "Chat")],
        1,
        vec![video_message_downloaded(10, path)],
    );
    o.open_handlers
        .insert("video/*".to_owned(), "true {file_path}".to_owned());

    o.handle_event(AppEvent::InputKey(KeyInput::new("l", false)))
        .unwrap();

    let popup = o
        .state()
        .command_popup()
        .expect("playback popup should open for video with custom handler");
    assert_eq!(popup.title(), "Playing");
    assert_eq!(
        popup.kind(),
        crate::domain::command_popup_state::CommandPopupKind::Playback
    );

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn l_on_document_with_custom_handler_opens_playback_popup() {
    let tmp = std::env::temp_dir().join("rtg_test_doc.pdf");
    std::fs::write(&tmp, b"fake").unwrap();
    let path = tmp.to_str().unwrap();

    let mut o = orchestrator_with_open_chat(
        vec![chat(1, "Chat")],
        1,
        vec![Message {
            id: 10,
            sender_name: "User".to_owned(),
            text: String::new(),
            timestamp_ms: 1000,
            is_outgoing: false,
            media: crate::domain::message::MessageMedia::Document,
            status: crate::domain::message::MessageStatus::Delivered,
            file_info: Some(crate::domain::message::FileInfo {
                file_id: 10,
                local_path: Some(path.to_owned()),
                mime_type: "application/pdf".to_owned(),
                size: Some(20_000),
                duration: None,
                file_name: Some("doc.pdf".to_owned()),
                is_listened: false,
                download_status: crate::domain::message::DownloadStatus::Completed,
            }),
            call_info: None,
            reply_to: None,
            reaction_count: 0,
            links: Vec::new(),
            is_edited: false,
        }],
    );
    o.open_handlers
        .insert("application/pdf".to_owned(), "true {file_path}".to_owned());

    o.handle_event(AppEvent::InputKey(KeyInput::new("l", false)))
        .unwrap();

    let popup = o
        .state()
        .command_popup()
        .expect("playback popup should open for document with custom handler");
    assert_eq!(popup.title(), "Playing");

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn l_on_sticker_is_ignored() {
    let mut o = orchestrator_with_open_chat(
        vec![chat(1, "Chat")],
        1,
        vec![Message {
            id: 10,
            sender_name: "User".to_owned(),
            text: String::new(),
            timestamp_ms: 1000,
            is_outgoing: false,
            media: crate::domain::message::MessageMedia::Sticker,
            status: crate::domain::message::MessageStatus::Delivered,
            file_info: Some(crate::domain::message::FileInfo {
                file_id: 10,
                local_path: Some("/tmp/sticker.webp".to_owned()),
                mime_type: "image/webp".to_owned(),
                size: Some(5000),
                duration: None,
                file_name: None,
                is_listened: false,
                download_status: crate::domain::message::DownloadStatus::Completed,
            }),
            call_info: None,
            reply_to: None,
            reaction_count: 0,
            links: Vec::new(),
            is_edited: false,
        }],
    );

    o.handle_event(AppEvent::InputKey(KeyInput::new("l", false)))
        .unwrap();
    assert!(o.state().command_popup().is_none());
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
