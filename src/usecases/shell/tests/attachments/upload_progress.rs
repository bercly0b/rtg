use super::*;
use crate::domain::message::{DownloadStatus, FileInfo, Message};

const FILE_ID: i32 = 7;

/// A document message as TDLib reports it while its file uploads.
fn document(id: i64, status: MessageStatus) -> Message {
    Message {
        media: MessageMedia::Document,
        status,
        is_outgoing: true,
        file_info: Some(FileInfo {
            file_id: FILE_ID,
            local_path: Some("/d/report.pdf".to_owned()),
            mime_type: "application/pdf".to_owned(),
            size: Some(4000),
            duration: None,
            file_name: Some("report.pdf".to_owned()),
            is_listened: false,
            download_status: DownloadStatus::Completed,
            upload_progress_percent: None,
        }),
        ..message(id, "")
    }
}

fn chat_with(messages: Vec<Message>) -> TestOrchestrator {
    orchestrator_with_open_chat(vec![chat(1, "Chat")], 1, messages)
}

fn file_updated(o: &mut TestOrchestrator, upload_progress_percent: Option<u8>) {
    o.handle_event(AppEvent::ChatUpdateReceived {
        updates: vec![ChatUpdate::FileUpdated {
            file_id: FILE_ID,
            size: 4000,
            downloaded_size: 4000,
            local_path: Some("/d/report.pdf".to_owned()),
            download_status: DownloadStatus::Completed,
            upload_progress_percent,
        }],
    })
    .unwrap();
}

fn upload_progress(messages: &[Message], id: i64) -> Option<u8> {
    messages
        .iter()
        .find(|message| message.id == id)
        .and_then(|message| message.file_info.as_ref())
        .and_then(|file_info| file_info.upload_progress_percent)
}

#[test]
fn upload_progress_reaches_the_message_being_sent() {
    let mut o = chat_with(vec![document(20, MessageStatus::Sending)]);

    file_updated(&mut o, Some(42));

    assert_eq!(
        upload_progress(o.state().open_chat().messages(), 20),
        Some(42)
    );
}

#[test]
fn upload_progress_goes_away_once_the_upload_is_over() {
    let mut o = chat_with(vec![document(20, MessageStatus::Sending)]);
    file_updated(&mut o, Some(42));

    file_updated(&mut o, None);

    assert_eq!(upload_progress(o.state().open_chat().messages(), 20), None);
}

#[test]
fn upload_progress_skips_a_delivered_message_with_the_same_file() {
    let mut o = chat_with(vec![
        document(19, MessageStatus::Delivered),
        document(20, MessageStatus::Sending),
    ]);

    file_updated(&mut o, Some(42));

    assert_eq!(upload_progress(o.state().open_chat().messages(), 19), None);
}

#[test]
fn upload_progress_reaches_the_cached_messages_of_another_chat() {
    let mut o = chat_with(vec![message(10, "Hello")]);
    o.state_mut()
        .message_cache_mut()
        .put(2, vec![document(20, MessageStatus::Sending)], true);

    file_updated(&mut o, Some(42));

    let cached = o.state_mut().message_cache_mut().get(2).unwrap().to_vec();
    assert_eq!(upload_progress(&cached, 20), Some(42));
}

#[test]
fn dd_deletes_a_message_telegram_has_not_sent_yet() {
    let mut o = chat_with(vec![document(20, MessageStatus::Sending)]);

    press(&mut o, "d");
    press(&mut o, "d");

    assert_eq!(o.dispatcher.delete_dispatch_count(), 1);
    assert!(o.state().open_chat().messages().is_empty());
}
