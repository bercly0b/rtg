//! Background task dispatcher for non-blocking API operations.
//!
//! Provides the [`TaskDispatcher`] trait and a thread-based implementation
//! that moves blocking Telegram API calls off the UI thread.

use std::sync::{mpsc::Sender, Arc};

use crate::domain::events::{BackgroundError, BackgroundTaskResult};

use super::{
    chat_lifecycle::{ChatLifecycle, ChatReadMarker, FileDownloader, MessageDeleter},
    chat_subtitle::{ChatInfoQuery, ChatSubtitleQuery, ChatSubtitleSource},
    list_chats::{list_chats, ListChatsQuery, ListChatsSource},
    load_messages::{load_messages, LoadMessagesQuery, MessagesSource},
    send_message::{send_message, MessageSender, SendMessageCommand},
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
    fn dispatch_chat_list(&self);
    fn dispatch_load_messages(&self, chat_id: i64);
    fn dispatch_send_message(&self, chat_id: i64, text: String);

    /// Informs TDLib that the user has opened a chat (fire-and-forget).
    fn dispatch_open_chat(&self, chat_id: i64);

    /// Informs TDLib that the user has closed a chat (fire-and-forget).
    fn dispatch_close_chat(&self, chat_id: i64);

    /// Marks messages as read in a chat (fire-and-forget).
    fn dispatch_mark_as_read(&self, chat_id: i64, message_ids: Vec<i64>);

    /// Marks a chat as read from the chat list (fire-and-forget).
    ///
    /// Performs openChat → viewMessages(force_read) → closeChat sequence
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
    MS: MessageSender + VoiceNoteSender + Send + Sync + 'static,
    L: ChatLifecycle + ChatReadMarker + MessageDeleter + FileDownloader + Send + Sync + 'static,
    S: ChatSubtitleSource + Send + Sync + 'static,
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
    MS: MessageSender + VoiceNoteSender + Send + Sync + 'static,
    L: ChatLifecycle + ChatReadMarker + MessageDeleter + FileDownloader + Send + Sync + 'static,
    S: ChatSubtitleSource + Send + Sync + 'static,
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
    MS: MessageSender + VoiceNoteSender + Send + Sync + 'static,
    L: ChatLifecycle + ChatReadMarker + MessageDeleter + FileDownloader + Send + Sync + 'static,
    S: ChatSubtitleSource + Send + Sync + 'static,
{
    fn dispatch_chat_list(&self) {
        let source = Arc::clone(&self.chats_source);
        let tx = self.result_tx.clone();
        let tx_fallback = self.result_tx.clone();

        let spawn_result = std::thread::Builder::new()
            .name("rtg-bg-chat-list".into())
            .spawn(move || {
                tracing::debug!("background: fetching chat list");
                let result = list_chats(source.as_ref(), ListChatsQuery::default())
                    .map(|output| output.chats)
                    .map_err(|error| {
                        tracing::warn!(error = ?error, "background: chat list fetch failed");
                        BackgroundError::new(map_list_chats_error(&error))
                    });

                let _ = tx.send(BackgroundTaskResult::ChatListLoaded { result });
            });

        if let Err(error) = spawn_result {
            tracing::error!(error = %error, "failed to spawn chat list background thread");
            let _ = tx_fallback.send(BackgroundTaskResult::ChatListLoaded {
                result: Err(BackgroundError::new("THREAD_SPAWN_FAILED")),
            });
        }
    }

    fn dispatch_load_messages(&self, chat_id: i64) {
        let source = Arc::clone(&self.messages_source);
        let tx = self.result_tx.clone();
        let tx_fallback = self.result_tx.clone();

        let spawn_result = std::thread::Builder::new()
            .name("rtg-bg-messages".into())
            .spawn(move || {
                tracing::debug!(chat_id, "background: fetching messages");
                let result = load_messages(source.as_ref(), LoadMessagesQuery::new(chat_id))
                    .map(|output| output.messages)
                    .map_err(|error| {
                        tracing::warn!(chat_id, error = ?error, "background: messages fetch failed");
                        BackgroundError::new(map_load_messages_error(&error))
                    });

                let _ = tx.send(BackgroundTaskResult::MessagesLoaded { chat_id, result });
            });

        if let Err(error) = spawn_result {
            tracing::error!(error = %error, "failed to spawn messages background thread");
            let _ = tx_fallback.send(BackgroundTaskResult::MessagesLoaded {
                chat_id,
                result: Err(BackgroundError::new("THREAD_SPAWN_FAILED")),
            });
        }
    }

    fn dispatch_send_message(&self, chat_id: i64, text: String) {
        let sender = Arc::clone(&self.message_sender);
        let messages_source = Arc::clone(&self.messages_source);
        let tx = self.result_tx.clone();
        let tx_fallback = self.result_tx.clone();
        let original_text = text.clone();

        let fallback_text = original_text.clone();
        let spawn_result = std::thread::Builder::new()
            .name("rtg-bg-send-msg".into())
            .spawn(move || {
                tracing::debug!(chat_id, "background: sending message");
                let command = SendMessageCommand { chat_id, text };
                let send_result = send_message(sender.as_ref(), command).map_err(|error| {
                    tracing::warn!(chat_id, error = ?error, "background: send message failed");
                    BackgroundError::new(map_send_message_error(&error))
                });

                let is_ok = send_result.is_ok();

                let _ = tx.send(BackgroundTaskResult::MessageSent {
                    chat_id,
                    original_text,
                    result: send_result,
                });

                // If send succeeded, automatically refresh messages
                if is_ok {
                    tracing::debug!(chat_id, "background: refreshing messages after send");
                    let refresh_result =
                        load_messages(messages_source.as_ref(), LoadMessagesQuery::new(chat_id))
                            .map(|output| output.messages)
                            .map_err(|error| {
                                tracing::warn!(
                                    chat_id,
                                    error = ?error,
                                    "background: messages refresh after send failed"
                                );
                                BackgroundError::new(map_load_messages_error(&error))
                            });

                    let _ = tx.send(BackgroundTaskResult::MessageSentRefreshCompleted {
                        chat_id,
                        result: refresh_result,
                    });
                }
            });

        if let Err(error) = spawn_result {
            tracing::error!(error = %error, "failed to spawn send message background thread");
            let _ = tx_fallback.send(BackgroundTaskResult::MessageSent {
                chat_id,
                original_text: fallback_text,
                result: Err(BackgroundError::new("THREAD_SPAWN_FAILED")),
            });
        }
    }

    fn dispatch_open_chat(&self, chat_id: i64) {
        let lifecycle = Arc::clone(&self.lifecycle);

        if let Err(error) = std::thread::Builder::new()
            .name("rtg-bg-open-chat".into())
            .spawn(move || {
                tracing::debug!(chat_id, "background: opening chat in TDLib");
                if let Err(e) = lifecycle.open_chat(chat_id) {
                    tracing::warn!(chat_id, error = ?e, "background: openChat failed");
                }
            })
        {
            tracing::error!(error = %error, "failed to spawn open chat background thread");
        }
    }

    fn dispatch_close_chat(&self, chat_id: i64) {
        let lifecycle = Arc::clone(&self.lifecycle);

        if let Err(error) = std::thread::Builder::new()
            .name("rtg-bg-close-chat".into())
            .spawn(move || {
                tracing::debug!(chat_id, "background: closing chat in TDLib");
                if let Err(e) = lifecycle.close_chat(chat_id) {
                    tracing::warn!(chat_id, error = ?e, "background: closeChat failed");
                }
            })
        {
            tracing::error!(error = %error, "failed to spawn close chat background thread");
        }
    }

    fn dispatch_mark_as_read(&self, chat_id: i64, message_ids: Vec<i64>) {
        let lifecycle = Arc::clone(&self.lifecycle);

        if let Err(error) = std::thread::Builder::new()
            .name("rtg-bg-mark-read".into())
            .spawn(move || {
                tracing::debug!(
                    chat_id,
                    message_count = message_ids.len(),
                    "background: marking messages as read"
                );
                if let Err(e) = lifecycle.mark_messages_read(chat_id, message_ids) {
                    tracing::warn!(chat_id, error = ?e, "background: viewMessages failed");
                }
            })
        {
            tracing::error!(error = %error, "failed to spawn mark-as-read background thread");
        }
    }

    fn dispatch_mark_chat_as_read(&self, chat_id: i64, last_message_id: i64) {
        let lifecycle = Arc::clone(&self.lifecycle);

        if let Err(error) = std::thread::Builder::new()
            .name("rtg-bg-mark-chat-read".into())
            .spawn(move || {
                tracing::debug!(chat_id, last_message_id, "background: marking chat as read from chat list");
                // Open chat so TDLib tracks it as viewed
                if let Err(e) = lifecycle.open_chat(chat_id) {
                    tracing::warn!(chat_id, error = ?e, "background: openChat failed during mark-chat-read");
                    return;
                }
                // Mark the last message as read
                if let Err(e) = lifecycle.mark_messages_read(chat_id, vec![last_message_id]) {
                    tracing::warn!(chat_id, error = ?e, "background: viewMessages failed during mark-chat-read");
                }
                // Close the chat
                if let Err(e) = lifecycle.close_chat(chat_id) {
                    tracing::warn!(chat_id, error = ?e, "background: closeChat failed during mark-chat-read");
                }
            })
        {
            tracing::error!(error = %error, "failed to spawn mark-chat-as-read background thread");
        }
    }

    fn dispatch_prefetch_messages(&self, chat_id: i64) {
        let source = Arc::clone(&self.messages_source);
        let tx = self.result_tx.clone();
        let tx_fallback = self.result_tx.clone();

        let spawn_result = std::thread::Builder::new()
            .name("rtg-bg-prefetch".into())
            .spawn(move || {
                tracing::debug!(chat_id, "background: prefetching messages");
                let result = load_messages(source.as_ref(), LoadMessagesQuery::new(chat_id))
                    .map(|output| output.messages)
                    .map_err(|error| {
                        tracing::warn!(chat_id, error = ?error, "background: prefetch failed");
                        BackgroundError::new(map_load_messages_error(&error))
                    });

                let _ = tx.send(BackgroundTaskResult::MessagesPrefetched { chat_id, result });
            });

        if let Err(error) = spawn_result {
            tracing::error!(error = %error, "failed to spawn prefetch background thread");
            let _ = tx_fallback.send(BackgroundTaskResult::MessagesPrefetched {
                chat_id,
                result: Err(BackgroundError::new("THREAD_SPAWN_FAILED")),
            });
        }
    }

    fn dispatch_delete_message(&self, chat_id: i64, message_id: i64) {
        let lifecycle = Arc::clone(&self.lifecycle);

        if let Err(error) = std::thread::Builder::new()
            .name("rtg-bg-delete-msg".into())
            .spawn(move || {
                tracing::debug!(chat_id, message_id, "background: deleting message");
                // Try revoke (delete for everyone) first
                let ids = vec![message_id];
                if let Err(e) = lifecycle.delete_messages(chat_id, ids.clone(), true) {
                    tracing::debug!(chat_id, message_id, error = ?e, "revoke delete failed, trying self-only");
                    // Fall back to delete for self only
                    if let Err(e2) = lifecycle.delete_messages(chat_id, ids, false) {
                        tracing::warn!(chat_id, message_id, error = ?e2, "self-only delete also failed");
                    }
                }
            })
        {
            tracing::error!(error = %error, "failed to spawn delete message background thread");
        }
    }

    fn dispatch_chat_subtitle(&self, query: ChatSubtitleQuery) {
        let source = Arc::clone(&self.subtitle_source);
        let tx = self.result_tx.clone();
        let chat_id = query.chat_id;

        if let Err(error) = std::thread::Builder::new()
            .name("rtg-bg-subtitle".into())
            .spawn(move || {
                tracing::debug!(chat_id, "background: resolving chat subtitle");
                let result = source
                    .resolve_chat_subtitle(&query)
                    .map_err(|_| BackgroundError::new("SUBTITLE_UNAVAILABLE"));

                let _ = tx.send(BackgroundTaskResult::ChatSubtitleLoaded { chat_id, result });
            })
        {
            tracing::error!(error = %error, "failed to spawn subtitle background thread");
        }
    }

    fn dispatch_send_voice(&self, chat_id: i64, file_path: String) {
        let sender = Arc::clone(&self.message_sender);
        let messages_source = Arc::clone(&self.messages_source);
        let tx = self.result_tx.clone();
        let tx_fallback = self.result_tx.clone();

        let spawn_result = std::thread::Builder::new()
            .name("rtg-bg-send-voice".into())
            .spawn(move || {
                use super::voice_recording;

                tracing::info!(chat_id, file_path, "background: sending voice note");

                let duration = voice_recording::get_audio_duration(&file_path).unwrap_or(0);
                let waveform = voice_recording::generate_waveform_stub();

                let result = sender
                    .send_voice_note(chat_id, &file_path, duration, &waveform)
                    .map_err(|error| {
                        tracing::warn!(
                            chat_id,
                            error = ?error,
                            "background: send voice note failed"
                        );
                        BackgroundError::new("SEND_VOICE_FAILED")
                    });

                if result.is_err() {
                    let _ = tx.send(BackgroundTaskResult::VoiceSendFailed { chat_id });
                    return;
                }

                // Refresh messages after successful send
                let refresh_result =
                    load_messages(messages_source.as_ref(), LoadMessagesQuery::new(chat_id))
                        .map(|output| output.messages)
                        .map_err(|error| {
                            tracing::warn!(
                                chat_id,
                                error = ?error,
                                "background: messages refresh after voice send failed"
                            );
                            BackgroundError::new(map_load_messages_error(&error))
                        });

                let _ = tx.send(BackgroundTaskResult::MessageSentRefreshCompleted {
                    chat_id,
                    result: refresh_result,
                });
            });

        if let Err(error) = spawn_result {
            tracing::error!(error = %error, "failed to spawn send-voice background thread");
            let _ = tx_fallback.send(BackgroundTaskResult::VoiceSendFailed { chat_id });
        }
    }

    fn dispatch_download_file(&self, file_id: i32) {
        let lifecycle = Arc::clone(&self.lifecycle);

        if let Err(error) = std::thread::Builder::new()
            .name("rtg-bg-download".into())
            .spawn(move || {
                tracing::debug!(file_id, "background: starting file download");
                if let Err(e) = lifecycle.download_file(file_id) {
                    tracing::warn!(file_id, error = ?e, "background: downloadFile failed");
                }
            })
        {
            tracing::error!(error = %error, "failed to spawn download file background thread");
        }
    }

    fn dispatch_chat_info(&self, query: ChatInfoQuery) {
        let source = Arc::clone(&self.subtitle_source);
        let tx = self.result_tx.clone();
        let tx_fallback = self.result_tx.clone();
        let chat_id = query.chat_id;

        let spawn_result = std::thread::Builder::new()
            .name("rtg-bg-chat-info".into())
            .spawn(move || {
                tracing::debug!(chat_id, "background: resolving chat info");
                let result = source
                    .resolve_chat_info(&query)
                    .map_err(|_| BackgroundError::new("CHAT_INFO_UNAVAILABLE"));

                let _ = tx.send(BackgroundTaskResult::ChatInfoLoaded { chat_id, result });
            });

        if let Err(error) = spawn_result {
            tracing::error!(error = %error, "failed to spawn chat info background thread");
            let _ = tx_fallback.send(BackgroundTaskResult::ChatInfoLoaded {
                chat_id,
                result: Err(BackgroundError::new("THREAD_SPAWN_FAILED")),
            });
        }
    }
}

fn map_list_chats_error(error: &super::list_chats::ListChatsError) -> &'static str {
    match error {
        super::list_chats::ListChatsError::Unauthorized => "CHAT_LIST_UNAUTHORIZED",
        super::list_chats::ListChatsError::TemporarilyUnavailable => "CHAT_LIST_UNAVAILABLE",
        super::list_chats::ListChatsError::DataContractViolation => "CHAT_LIST_DATA_ERROR",
    }
}

fn map_load_messages_error(error: &super::load_messages::LoadMessagesError) -> &'static str {
    match error {
        super::load_messages::LoadMessagesError::Unauthorized => "MESSAGES_UNAUTHORIZED",
        super::load_messages::LoadMessagesError::TemporarilyUnavailable => "MESSAGES_UNAVAILABLE",
        super::load_messages::LoadMessagesError::ChatNotFound => "MESSAGES_CHAT_NOT_FOUND",
    }
}

fn map_send_message_error(error: &super::send_message::SendMessageError) -> &'static str {
    match error {
        super::send_message::SendMessageError::EmptyMessage => "SEND_EMPTY_MESSAGE",
        super::send_message::SendMessageError::Unauthorized => "SEND_UNAUTHORIZED",
        super::send_message::SendMessageError::ChatNotFound => "SEND_CHAT_NOT_FOUND",
        super::send_message::SendMessageError::TemporarilyUnavailable => "SEND_UNAVAILABLE",
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::sync::mpsc;

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
        fn dispatch_chat_list(&self) {
            // Stub: does not dispatch; tests inject results manually
        }

        fn dispatch_load_messages(&self, _chat_id: i64) {
            // Stub: does not dispatch; tests inject results manually
        }

        fn dispatch_send_message(&self, _chat_id: i64, _text: String) {
            // Stub: does not dispatch; tests inject results manually
        }

        fn dispatch_open_chat(&self, _chat_id: i64) {
            // Stub: fire-and-forget, no action needed in tests
        }

        fn dispatch_close_chat(&self, _chat_id: i64) {
            // Stub: fire-and-forget, no action needed in tests
        }

        fn dispatch_mark_as_read(&self, _chat_id: i64, _message_ids: Vec<i64>) {
            // Stub: fire-and-forget, no action needed in tests
        }

        fn dispatch_mark_chat_as_read(&self, _chat_id: i64, _last_message_id: i64) {
            // Stub: fire-and-forget, no action needed in tests
        }

        fn dispatch_prefetch_messages(&self, _chat_id: i64) {
            // Stub: does not dispatch; tests inject results manually
        }

        fn dispatch_delete_message(&self, _chat_id: i64, _message_id: i64) {
            // Stub: fire-and-forget, no action needed in tests
        }

        fn dispatch_chat_subtitle(&self, _query: ChatSubtitleQuery) {
            // Stub: does not dispatch; tests inject results manually
        }

        fn dispatch_send_voice(&self, _chat_id: i64, _file_path: String) {
            // Stub: fire-and-forget, no action needed in tests
        }

        fn dispatch_download_file(&self, _file_id: i32) {
            // Stub: fire-and-forget, no action needed in tests
        }

        fn dispatch_chat_info(&self, _query: ChatInfoQuery) {
            // Stub: does not dispatch; tests inject results manually
        }
    }

    // ── Error mapper tests ──

    #[test]
    fn map_list_chats_error_unauthorized() {
        use super::super::list_chats::ListChatsError;
        assert_eq!(
            map_list_chats_error(&ListChatsError::Unauthorized),
            "CHAT_LIST_UNAUTHORIZED"
        );
    }

    #[test]
    fn map_list_chats_error_unavailable() {
        use super::super::list_chats::ListChatsError;
        assert_eq!(
            map_list_chats_error(&ListChatsError::TemporarilyUnavailable),
            "CHAT_LIST_UNAVAILABLE"
        );
    }

    #[test]
    fn map_list_chats_error_data_contract() {
        use super::super::list_chats::ListChatsError;
        assert_eq!(
            map_list_chats_error(&ListChatsError::DataContractViolation),
            "CHAT_LIST_DATA_ERROR"
        );
    }

    #[test]
    fn map_load_messages_error_unauthorized() {
        use super::super::load_messages::LoadMessagesError;
        assert_eq!(
            map_load_messages_error(&LoadMessagesError::Unauthorized),
            "MESSAGES_UNAUTHORIZED"
        );
    }

    #[test]
    fn map_load_messages_error_unavailable() {
        use super::super::load_messages::LoadMessagesError;
        assert_eq!(
            map_load_messages_error(&LoadMessagesError::TemporarilyUnavailable),
            "MESSAGES_UNAVAILABLE"
        );
    }

    #[test]
    fn map_load_messages_error_chat_not_found() {
        use super::super::load_messages::LoadMessagesError;
        assert_eq!(
            map_load_messages_error(&LoadMessagesError::ChatNotFound),
            "MESSAGES_CHAT_NOT_FOUND"
        );
    }

    #[test]
    fn map_send_message_error_empty() {
        use super::super::send_message::SendMessageError;
        assert_eq!(
            map_send_message_error(&SendMessageError::EmptyMessage),
            "SEND_EMPTY_MESSAGE"
        );
    }

    #[test]
    fn map_send_message_error_unauthorized() {
        use super::super::send_message::SendMessageError;
        assert_eq!(
            map_send_message_error(&SendMessageError::Unauthorized),
            "SEND_UNAUTHORIZED"
        );
    }

    #[test]
    fn map_send_message_error_chat_not_found() {
        use super::super::send_message::SendMessageError;
        assert_eq!(
            map_send_message_error(&SendMessageError::ChatNotFound),
            "SEND_CHAT_NOT_FOUND"
        );
    }

    #[test]
    fn map_send_message_error_unavailable() {
        use super::super::send_message::SendMessageError;
        assert_eq!(
            map_send_message_error(&SendMessageError::TemporarilyUnavailable),
            "SEND_UNAVAILABLE"
        );
    }
}
