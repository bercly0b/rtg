//! Background task dispatcher for non-blocking API operations.
//!
//! Provides the [`TaskDispatcher`] trait and a thread-based implementation
//! that moves blocking Telegram API calls off the UI thread.

mod error_mapping;
mod file_ops;
mod lifecycle;
mod messaging;

use std::sync::{mpsc::Sender, Arc};

use crate::domain::events::BackgroundTaskResult;

use super::{
    chat_lifecycle::{ChatLifecycle, ChatReadMarker, FileDownloader, MessageDeleter},
    chat_subtitle::{ChatInfoQuery, ChatSubtitleQuery, ChatSubtitleSource},
    edit_message::MessageEditor,
    list_chats::ListChatsSource,
    load_messages::MessagesSource,
    message_info::{MessageInfoQuery, MessageInfoSource},
    send_message::MessageSender,
    send_voice::VoiceNoteSender,
};

/// Contract for dispatching background work from the orchestrator.
///
/// Implementations must be non-blocking: they enqueue work and return immediately.
/// Results are delivered asynchronously via the background result channel.
///
/// Lifecycle operations (`dispatch_open_chat`, `dispatch_close_chat`,
/// `dispatch_mark_as_read`) are fire-and-forget: errors are logged
/// but do not produce `BackgroundTaskResult`.
pub trait TaskDispatcher {
    fn dispatch_chat_list(&self, force: bool);
    fn dispatch_load_messages(&self, chat_id: i64);
    fn dispatch_send_message(&self, chat_id: i64, text: String, reply_to_message_id: Option<i64>);
    fn dispatch_edit_message(&self, chat_id: i64, message_id: i64, text: String);

    /// Informs TDLib that the user has opened a chat (fire-and-forget).
    fn dispatch_open_chat(&self, chat_id: i64);

    /// Informs TDLib that the user has closed a chat (fire-and-forget).
    fn dispatch_close_chat(&self, chat_id: i64);

    /// Marks messages as read in a chat (fire-and-forget).
    fn dispatch_mark_as_read(&self, chat_id: i64, message_ids: Vec<i64>);

    /// Marks a chat as read from the chat list (fire-and-forget).
    ///
    /// Performs openChat -> viewMessages(force_read) -> closeChat sequence
    /// to mark the chat as read without loading its messages.
    fn dispatch_mark_chat_as_read(&self, chat_id: i64, last_message_id: i64);

    /// Prefetches messages for a chat the user is hovering in the chat list.
    /// Results go into `MessageCache` only (not `OpenChatState`).
    fn dispatch_prefetch_messages(&self, chat_id: i64);

    /// Deletes a message from a chat (fire-and-forget).
    ///
    /// Tries `revoke=true` first (delete for everyone), falls back to
    /// `revoke=false` (delete for self only) if that fails.
    fn dispatch_delete_message(&self, chat_id: i64, message_id: i64);

    /// Resolves the chat subtitle (user status, member count, etc.) in the background.
    fn dispatch_chat_subtitle(&self, query: ChatSubtitleQuery);

    /// Sends a recorded voice note to a chat (fire-and-forget for now).
    ///
    /// Extracts audio duration via ffprobe, generates a waveform stub,
    /// and calls the Telegram API.
    fn dispatch_send_voice(&self, chat_id: i64, file_path: String);

    /// Triggers a file download in TDLib (fire-and-forget).
    ///
    /// Progress is delivered asynchronously via `updateFile` events.
    fn dispatch_download_file(&self, file_id: i32);

    /// Resolves full chat info (title, status, description) in the background.
    fn dispatch_chat_info(&self, query: ChatInfoQuery);

    /// Opens a file with the platform default opener in the background.
    ///
    /// Waits for the process to finish; if it exits with a non-zero code,
    /// sends `BackgroundTaskResult::OpenFileFailed` with the captured stderr.
    fn dispatch_open_file(&self, cmd_template: String, file_path: String);

    /// Copies a file to the OS downloads directory in the background.
    fn dispatch_save_file(&self, file_id: i32, local_path: String, file_name: Option<String>);

    /// Resolves message info (reactions, viewers, read/edit dates) in the background.
    fn dispatch_message_info(&self, query: MessageInfoQuery);
}

/// Thread-based dispatcher that runs blocking API calls on background OS threads.
///
/// Each dispatched operation spawns a short-lived thread that calls the
/// synchronous source trait method (which internally does `rt.block_on()`),
/// then sends the result through the shared channel.
pub struct ThreadTaskDispatcher<C, M, MS, L, S>
where
    C: ListChatsSource + Send + Sync + 'static,
    M: MessagesSource + Send + Sync + 'static,
    MS: MessageSender + MessageEditor + VoiceNoteSender + Send + Sync + 'static,
    L: ChatLifecycle + ChatReadMarker + MessageDeleter + FileDownloader + Send + Sync + 'static,
    S: ChatSubtitleSource + MessageInfoSource + Send + Sync + 'static,
{
    chats_source: Arc<C>,
    messages_source: Arc<M>,
    message_sender: Arc<MS>,
    lifecycle: Arc<L>,
    subtitle_source: Arc<S>,
    result_tx: Sender<BackgroundTaskResult>,
}

impl<C, M, MS, L, S> ThreadTaskDispatcher<C, M, MS, L, S>
where
    C: ListChatsSource + Send + Sync + 'static,
    M: MessagesSource + Send + Sync + 'static,
    MS: MessageSender + MessageEditor + VoiceNoteSender + Send + Sync + 'static,
    L: ChatLifecycle + ChatReadMarker + MessageDeleter + FileDownloader + Send + Sync + 'static,
    S: ChatSubtitleSource + MessageInfoSource + Send + Sync + 'static,
{
    pub fn new(
        chats_source: Arc<C>,
        messages_source: Arc<M>,
        message_sender: Arc<MS>,
        lifecycle: Arc<L>,
        subtitle_source: Arc<S>,
        result_tx: Sender<BackgroundTaskResult>,
    ) -> Self {
        Self {
            chats_source,
            messages_source,
            message_sender,
            lifecycle,
            subtitle_source,
            result_tx,
        }
    }
}

impl<C, M, MS, L, S> TaskDispatcher for ThreadTaskDispatcher<C, M, MS, L, S>
where
    C: ListChatsSource + Send + Sync + 'static,
    M: MessagesSource + Send + Sync + 'static,
    MS: MessageSender + MessageEditor + VoiceNoteSender + Send + Sync + 'static,
    L: ChatLifecycle + ChatReadMarker + MessageDeleter + FileDownloader + Send + Sync + 'static,
    S: ChatSubtitleSource + MessageInfoSource + Send + Sync + 'static,
{
    fn dispatch_chat_list(&self, force: bool) {
        lifecycle::dispatch_chat_list(&self.chats_source, &self.result_tx, force);
    }

    fn dispatch_load_messages(&self, chat_id: i64) {
        messaging::dispatch_load_messages(&self.messages_source, &self.result_tx, chat_id);
    }

    fn dispatch_send_message(&self, chat_id: i64, text: String, reply_to_message_id: Option<i64>) {
        messaging::dispatch_send_message(
            &self.message_sender,
            &self.messages_source,
            &self.result_tx,
            chat_id,
            text,
            reply_to_message_id,
        );
    }

    fn dispatch_edit_message(&self, chat_id: i64, message_id: i64, text: String) {
        messaging::dispatch_edit_message(
            &self.message_sender,
            &self.result_tx,
            chat_id,
            message_id,
            text,
        );
    }

    fn dispatch_open_chat(&self, chat_id: i64) {
        lifecycle::dispatch_open_chat(&self.lifecycle, chat_id);
    }

    fn dispatch_close_chat(&self, chat_id: i64) {
        lifecycle::dispatch_close_chat(&self.lifecycle, chat_id);
    }

    fn dispatch_mark_as_read(&self, chat_id: i64, message_ids: Vec<i64>) {
        lifecycle::dispatch_mark_as_read(&self.lifecycle, chat_id, message_ids);
    }

    fn dispatch_mark_chat_as_read(&self, chat_id: i64, last_message_id: i64) {
        lifecycle::dispatch_mark_chat_as_read(&self.lifecycle, chat_id, last_message_id);
    }

    fn dispatch_prefetch_messages(&self, chat_id: i64) {
        messaging::dispatch_prefetch_messages(&self.messages_source, &self.result_tx, chat_id);
    }

    fn dispatch_delete_message(&self, chat_id: i64, message_id: i64) {
        lifecycle::dispatch_delete_message(&self.lifecycle, chat_id, message_id);
    }

    fn dispatch_chat_subtitle(&self, query: ChatSubtitleQuery) {
        lifecycle::dispatch_chat_subtitle(&self.subtitle_source, &self.result_tx, query);
    }

    fn dispatch_send_voice(&self, chat_id: i64, file_path: String) {
        messaging::dispatch_send_voice(
            &self.message_sender,
            &self.messages_source,
            &self.result_tx,
            chat_id,
            file_path,
        );
    }

    fn dispatch_download_file(&self, file_id: i32) {
        lifecycle::dispatch_download_file(&self.lifecycle, file_id);
    }

    fn dispatch_chat_info(&self, query: ChatInfoQuery) {
        lifecycle::dispatch_chat_info(&self.subtitle_source, &self.result_tx, query);
    }

    fn dispatch_open_file(&self, cmd_template: String, file_path: String) {
        file_ops::dispatch_open_file(&self.result_tx, cmd_template, file_path);
    }

    fn dispatch_save_file(&self, file_id: i32, local_path: String, file_name: Option<String>) {
        file_ops::dispatch_save_file(&self.result_tx, file_id, local_path, file_name);
    }

    fn dispatch_message_info(&self, query: MessageInfoQuery) {
        lifecycle::dispatch_message_info(&self.subtitle_source, &self.result_tx, query);
    }
}

#[cfg(test)]
pub mod tests;
