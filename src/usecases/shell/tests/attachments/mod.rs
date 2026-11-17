mod picker;
mod sending;
mod upload_progress;

use super::*;
use crate::domain::{
    attachment::{AttachmentKind, OutgoingAttachment},
    chat::ChatType,
    file_picker_state::{FileEntry, FileListing},
    message::{MessageMedia, MessageStatus},
    open_defaults::DEFAULT_OPEN,
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
    press(&mut o, "A");
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
