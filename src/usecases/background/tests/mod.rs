mod error_mapping;

use std::sync::mpsc::{self, Sender};

use crate::{
    domain::events::BackgroundTaskResult,
    usecases::{
        background::TaskDispatcher,
        chat_subtitle::{ChatInfoQuery, ChatSubtitleQuery},
    },
};

/// Stub dispatcher that records dispatched operations and delivers results
/// through a test channel for assertions.
pub struct StubTaskDispatcher {
    result_tx: Sender<BackgroundTaskResult>,
}

impl StubTaskDispatcher {
    pub fn new() -> (Self, mpsc::Receiver<BackgroundTaskResult>) {
        let (tx, rx) = mpsc::channel();
        (Self { result_tx: tx }, rx)
    }

    /// Manually inject a result as if a background task completed.
    pub fn inject_result(&self, result: BackgroundTaskResult) {
        let _ = self.result_tx.send(result);
    }
}

impl TaskDispatcher for StubTaskDispatcher {
    fn dispatch_chat_list(&self, _force: bool) {}

    fn dispatch_load_messages(&self, _chat_id: i64) {}

    fn dispatch_send_message(
        &self,
        _chat_id: i64,
        _text: String,
        _reply_to_message_id: Option<i64>,
    ) {
    }

    fn dispatch_open_chat(&self, _chat_id: i64) {}

    fn dispatch_close_chat(&self, _chat_id: i64) {}

    fn dispatch_mark_as_read(&self, _chat_id: i64, _message_ids: Vec<i64>) {}

    fn dispatch_mark_chat_as_read(&self, _chat_id: i64, _last_message_id: i64) {}

    fn dispatch_prefetch_messages(&self, _chat_id: i64) {}

    fn dispatch_delete_message(&self, _chat_id: i64, _message_id: i64) {}

    fn dispatch_chat_subtitle(&self, _query: ChatSubtitleQuery) {}

    fn dispatch_send_voice(&self, _chat_id: i64, _file_path: String) {}

    fn dispatch_download_file(&self, _file_id: i32) {}

    fn dispatch_chat_info(&self, _query: ChatInfoQuery) {}

    fn dispatch_open_file(&self, _cmd_template: String, _file_path: String) {}

    fn dispatch_save_file(&self, _file_id: i32, _local_path: String, _file_name: Option<String>) {}
}
