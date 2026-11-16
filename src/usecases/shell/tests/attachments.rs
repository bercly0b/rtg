use super::*;
use crate::domain::{
    attachment::{AttachmentKind, OutgoingAttachment},
    chat::ChatType,
    file_picker_state::{FileEntry, FileListing},
    message::{MessageMedia, MessageStatus},
    shell_state::ActivePane,
};

fn press(o: &mut TestOrchestrator, key: &str) {
    o.handle_event(AppEvent::InputKey(KeyInput::new(key, false)))
        .unwrap();
}

fn press_ctrl(o: &mut TestOrchestrator, key: &str) {
    o.handle_event(AppEvent::InputKey(KeyInput::new(key, true)))
        .unwrap();
}

fn type_text(o: &mut TestOrchestrator, text: &str) {
    for ch in text.chars() {
        press(o, &ch.to_string());
    }
}

fn entry(path: &str, modified_unix_s: i64) -> FileEntry {
    FileEntry {
        path: path.to_owned(),
        size: 42,
        modified_unix_s,
    }
}

fn complete(o: &mut TestOrchestrator, result: BackgroundTaskResult) {
    o.handle_event(AppEvent::BackgroundTasksCompleted(vec![result]))
        .unwrap();
}

fn list_files(o: &mut TestOrchestrator, listing_id: u64, entries: Vec<FileEntry>) {
    complete(
        o,
        BackgroundTaskResult::FilesListed {
            listing_id,
            entries,
        },
    );
}

fn open_chat() -> TestOrchestrator {
    orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, vec![message(10, "Hello")])
}

/// Chat 1 is open and the file picker lists `entries`.
fn picker_listing(entries: Vec<FileEntry>) -> TestOrchestrator {
    let mut o = open_chat();
    press(&mut o, "a");
    let listing_id = o.dispatcher.last_file_listing().unwrap().0;
    list_files(&mut o, listing_id, entries);
    o
}

fn selected_path(o: &TestOrchestrator) -> Option<String> {
    o.state()
        .file_picker()
        .and_then(|picker| picker.selected())
        .map(|entry| entry.path.clone())
}

fn sent(o: &TestOrchestrator) -> (i64, Option<i32>, OutgoingAttachment) {
    o.dispatcher
        .last_attachment_send()
        .expect("an attachment must have been dispatched")
}

fn fail_send(o: &mut TestOrchestrator, code: &'static str) {
    let (chat_id, topic_id, attachment) = sent(o);
    complete(
        o,
        BackgroundTaskResult::AttachmentSent {
            chat_id,
            topic_id,
            attachment,
            result: Err(BackgroundError::new(code)),
        },
    );
}

// ── opening the picker ──

#[test]
fn a_opens_the_file_picker_and_starts_the_file_list_command() {
    let mut o = DefaultShellOrchestrator::new_with_initial_state(
        StubStorageAdapter::default(),
        RecordingOpener::default(),
        RecordingDispatcher::new(),
        ShellState::default(),
        None,
        OrchestratorConfig {
            min_display_messages: 1,
            file_list_cmd: "fd . {dirs}".to_owned(),
            attach_dirs: vec!["~/Desktop".to_owned()],
            ..OrchestratorConfig::default()
        },
    );
    inject_chat_list(&mut o, vec![chat(1, "Chat")]);
    press(&mut o, "enter");
    inject_messages(&mut o, 1, vec![message(10, "Hello")]);

    press(&mut o, "a");

    let picker = o.state().file_picker().expect("file picker must be open");
    assert_eq!(picker.listing(), &FileListing::Running);
    assert_eq!(
        o.dispatcher.last_file_listing(),
        Some((
            picker.listing_id(),
            "fd . {dirs}".to_owned(),
            vec!["~/Desktop".to_owned()]
        ))
    );
}

#[test]
fn a_does_nothing_without_an_open_chat() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Chat")]);
    o.state_mut().set_active_pane(ActivePane::Messages);

    press(&mut o, "a");

    assert!(o.state().file_picker().is_none());
    assert_eq!(o.dispatcher.last_file_listing(), None);
}

#[test]
fn a_in_a_channel_says_it_is_read_only() {
    let mut o =
        orchestrator_with_open_chat(vec![channel_chat(1, "News")], 1, vec![message(10, "Hello")]);
    assert_eq!(o.state().open_chat().chat_type(), ChatType::Channel);

    press(&mut o, "a");

    assert!(o.state().file_picker().is_none());
    assert_eq!(
        o.state().active_notification(),
        Some("Channel is read-only")
    );
}

#[test]
fn a_in_a_closed_topic_says_it_is_closed() {
    let mut closed = topic(1, 7, "Releases", 1);
    closed.is_closed = true;
    let mut o = orchestrator_with_chats(vec![forum_chat(1, "Forum")]);
    press(&mut o, "enter");
    inject_forum_topics(&mut o, 1, vec![closed]);
    press(&mut o, "enter");
    inject_topic_messages(&mut o, 1, Some(7), vec![message(10, "Hello")]);

    press(&mut o, "a");

    assert!(o.state().file_picker().is_none());
    assert_eq!(o.state().active_notification(), Some("Topic is closed"));
}

// ── listing ──

#[test]
fn listed_files_fill_the_picker_newest_first() {
    let o = picker_listing(vec![entry("/d/old.png", 100), entry("/d/new.png", 200)]);

    let picker = o.state().file_picker().unwrap();
    assert_eq!(picker.visible_len(), 2);
    assert_eq!(selected_path(&o).as_deref(), Some("/d/new.png"));
}

#[test]
fn files_of_a_previous_picker_are_ignored() {
    let mut o = open_chat();
    press(&mut o, "a");
    let stale_id = o.dispatcher.last_file_listing().unwrap().0;
    press(&mut o, "esc");
    press(&mut o, "a");

    list_files(&mut o, stale_id, vec![entry("/d/stale.png", 100)]);
    complete(
        &mut o,
        BackgroundTaskResult::FileListingFinished {
            listing_id: stale_id,
            error: Some("killed".to_owned()),
        },
    );

    let picker = o.state().file_picker().unwrap();
    assert_eq!(picker.visible_len(), 0);
    assert_eq!(picker.listing(), &FileListing::Running);
}

#[test]
fn failed_listing_keeps_the_picker_open_with_the_reason() {
    let mut o = open_chat();
    press(&mut o, "a");
    let listing_id = o.dispatcher.last_file_listing().unwrap().0;

    complete(
        &mut o,
        BackgroundTaskResult::FileListingFinished {
            listing_id,
            error: Some("command not found: fd".to_owned()),
        },
    );

    assert_eq!(
        o.state().file_picker().unwrap().listing(),
        &FileListing::Failed("command not found: fd".to_owned())
    );
}

// ── keys inside the picker ──

#[test]
fn typing_filters_the_files_and_backspace_widens_the_filter() {
    let mut o = picker_listing(vec![entry("/d/shot.png", 200), entry("/d/notes.txt", 100)]);

    type_text(&mut o, "txt");
    assert_eq!(o.state().file_picker().unwrap().query(), "txt");
    assert_eq!(selected_path(&o).as_deref(), Some("/d/notes.txt"));

    press(&mut o, "backspace");
    press(&mut o, "backspace");
    press(&mut o, "backspace");
    assert_eq!(o.state().file_picker().unwrap().visible_len(), 2);
}

#[test]
fn letters_that_are_hotkeys_elsewhere_go_into_the_query() {
    let mut o = picker_listing(vec![entry("/d/shot.png", 200)]);

    type_text(&mut o, "jkq");

    assert_eq!(o.state().file_picker().unwrap().query(), "jkq");
    assert!(o.state().is_running());
}

#[test]
fn ctrl_j_and_ctrl_k_move_the_selection() {
    let mut o = picker_listing(vec![entry("/d/new.png", 200), entry("/d/old.png", 100)]);

    press_ctrl(&mut o, "j");
    assert_eq!(selected_path(&o).as_deref(), Some("/d/old.png"));

    press_ctrl(&mut o, "k");
    assert_eq!(selected_path(&o).as_deref(), Some("/d/new.png"));
    assert_eq!(o.state().file_picker().unwrap().query(), "");
}

#[test]
fn arrows_move_the_selection() {
    let mut o = picker_listing(vec![entry("/d/new.png", 200), entry("/d/old.png", 100)]);

    press(&mut o, "down");
    assert_eq!(selected_path(&o).as_deref(), Some("/d/old.png"));

    press(&mut o, "up");
    assert_eq!(selected_path(&o).as_deref(), Some("/d/new.png"));
}

#[test]
fn esc_closes_the_picker_and_stops_the_file_list_command() {
    let mut o = picker_listing(vec![entry("/d/shot.png", 200)]);

    press(&mut o, "esc");

    assert!(o.state().file_picker().is_none());
    assert_eq!(o.dispatcher.stopped_file_listings(), 1);
    assert_eq!(o.dispatcher.attachment_send_count(), 0);
}

// ── sending ──

#[test]
fn enter_sends_the_selected_file_and_shows_it_as_sending() {
    let mut o = picker_listing(vec![entry("/d/report.pdf", 200), entry("/d/old.pdf", 100)]);
    press_ctrl(&mut o, "j");

    press(&mut o, "enter");

    assert!(o.state().file_picker().is_none());
    assert_eq!(o.dispatcher.stopped_file_listings(), 1);

    let pending = o.state().open_chat().messages().last().unwrap().clone();
    assert_eq!(pending.status, MessageStatus::Sending);
    assert_eq!(pending.media, MessageMedia::Document);
    assert_eq!(pending.text, "");
    assert_eq!(
        sent(&o),
        (
            1,
            None,
            OutgoingAttachment {
                pending_id: pending.id,
                path: "/d/old.pdf".to_owned(),
                size_bytes: 42,
                kind: AttachmentKind::Document,
                caption: String::new(),
                reply_to: None,
            }
        )
    );
}

#[test]
fn enter_with_no_file_selected_sends_nothing() {
    let mut o = picker_listing(Vec::new());

    press(&mut o, "enter");

    assert!(o.state().file_picker().is_some());
    assert_eq!(o.dispatcher.attachment_send_count(), 0);
}

#[test]
fn image_within_the_photo_limits_is_sent_as_a_photo() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("shot.png");
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13];
    png.extend(b"IHDR");
    png.extend(640u32.to_be_bytes());
    png.extend(480u32.to_be_bytes());
    png.extend([8, 6, 0, 0, 0, 0, 0, 0, 0]);
    std::fs::write(&path, png).unwrap();
    let mut o = picker_listing(vec![entry(path.to_str().unwrap(), 200)]);

    press(&mut o, "enter");

    assert_eq!(
        sent(&o).2.kind,
        AttachmentKind::Photo {
            width: 640,
            height: 480
        }
    );
    assert_eq!(
        o.state().open_chat().messages().last().unwrap().media,
        MessageMedia::Photo
    );
}

#[test]
fn text_in_the_input_goes_as_the_caption_and_the_input_is_cleared() {
    let mut o = open_chat();
    press(&mut o, "i");
    type_text(&mut o, " Look at this ");
    press(&mut o, "esc");
    press(&mut o, "a");
    let listing_id = o.dispatcher.last_file_listing().unwrap().0;
    list_files(&mut o, listing_id, vec![entry("/d/report.pdf", 200)]);

    press(&mut o, "enter");

    assert_eq!(sent(&o).2.caption, "Look at this");
    assert_eq!(o.state().message_input().text(), "");
    assert_eq!(
        o.state().open_chat().messages().last().unwrap().text,
        "Look at this"
    );
}

#[test]
fn reply_target_is_passed_on_with_the_attachment() {
    let mut o = open_chat();
    let reply = ReplyContext {
        message_id: 10,
        sender_name: "User".to_owned(),
        text: "Hello".to_owned(),
    };
    o.state_mut()
        .message_input_mut()
        .set_reply_to(reply.clone());
    press(&mut o, "a");
    let listing_id = o.dispatcher.last_file_listing().unwrap().0;
    list_files(&mut o, listing_id, vec![entry("/d/report.pdf", 200)]);

    press(&mut o, "enter");

    assert_eq!(sent(&o).2.reply_to, Some(reply));
    assert!(o.state().message_input().reply_to().is_none());
    let pending = o.state().open_chat().messages().last().unwrap();
    assert_eq!(pending.reply_to.as_ref().unwrap().text, "Hello");
}

#[test]
fn attachment_goes_into_the_open_forum_topic() {
    let mut o = orchestrator_with_chats(vec![forum_chat(1, "Forum")]);
    press(&mut o, "enter");
    inject_forum_topics(&mut o, 1, vec![topic(1, 7, "Releases", 1)]);
    press(&mut o, "enter");
    inject_topic_messages(&mut o, 1, Some(7), vec![message(10, "Hello")]);
    press(&mut o, "a");
    let listing_id = o.dispatcher.last_file_listing().unwrap().0;
    list_files(&mut o, listing_id, vec![entry("/d/report.pdf", 200)]);

    press(&mut o, "enter");

    let (chat_id, topic_id, _) = sent(&o);
    assert_eq!((chat_id, topic_id), (1, Some(7)));
}

// ── failed send ──

#[test]
fn failed_send_removes_the_pending_message_and_brings_the_caption_back() {
    let mut o = open_chat();
    press(&mut o, "i");
    type_text(&mut o, "Look");
    press(&mut o, "esc");
    press(&mut o, "a");
    let listing_id = o.dispatcher.last_file_listing().unwrap().0;
    list_files(&mut o, listing_id, vec![entry("/d/report.pdf", 200)]);
    press(&mut o, "enter");
    let pending_id = sent(&o).2.pending_id;

    fail_send(&mut o, "SEND_ATTACHMENT_FAILED");

    assert!(o
        .state()
        .open_chat()
        .messages()
        .iter()
        .all(|message| message.id != pending_id));
    assert_eq!(o.state().message_input().text(), "Look");
    assert_eq!(
        o.state().active_notification(),
        Some("Failed to send the file")
    );
}

#[test]
fn file_over_the_upload_limit_is_reported_as_too_large() {
    let mut o = picker_listing(vec![entry("/d/huge.iso", 200)]);
    press(&mut o, "enter");

    fail_send(&mut o, crate::usecases::background::ATTACHMENT_TOO_LARGE);

    assert_eq!(o.state().open_chat().messages().len(), 1);
    assert_eq!(
        o.state().active_notification(),
        Some("File is too large to send")
    );
}

#[test]
fn failed_send_without_a_caption_leaves_a_newer_draft_alone() {
    let mut o = picker_listing(vec![entry("/d/report.pdf", 200)]);
    press(&mut o, "enter");
    press(&mut o, "i");
    type_text(&mut o, "new draft");

    fail_send(&mut o, "SEND_ATTACHMENT_FAILED");

    assert_eq!(o.state().message_input().text(), "new draft");
}

#[test]
fn successful_send_keeps_the_pending_message_until_the_refresh() {
    let mut o = picker_listing(vec![entry("/d/report.pdf", 200)]);
    press(&mut o, "enter");
    let (chat_id, topic_id, attachment) = sent(&o);

    complete(
        &mut o,
        BackgroundTaskResult::AttachmentSent {
            chat_id,
            topic_id,
            attachment,
            result: Ok(()),
        },
    );

    assert_eq!(o.state().open_chat().messages().len(), 2);
    assert_eq!(o.state().active_notification(), None);
}
