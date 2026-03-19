//! Background task dispatcher for non-blocking API operations.
//!
//! Provides the [`TaskDispatcher`] trait and a thread-based implementation
//! that moves blocking Telegram API calls off the UI thread.

use std::sync::{mpsc::Sender, Arc};

use crate::domain::events::{BackgroundError, BackgroundTaskResult};

use super::{
    chat_lifecycle::{ChatLifecycle, ChatReadMarker},
    list_chats::{list_chats, ListChatsQuery, ListChatsSource},
    load_messages::{load_messages, LoadMessagesQuery, MessagesSource},
    send_message::{send_message, MessageSender, SendMessageCommand},
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
}

/// Thread-based dispatcher that runs blocking API calls on background OS threads.
///
/// Each dispatched operation spawns a short-lived thread that calls the
/// synchronous source trait method (which internally does `rt.block_on()`),
/// then sends the result through the shared channel.
pub struct ThreadTaskDispatcher<C, M, MS, L>
where
    C: ListChatsSource + Send + Sync + 'static,
    M: MessagesSource + Send + Sync + 'static,
    MS: MessageSender + Send + Sync + 'static,
    L: ChatLifecycle + ChatReadMarker + Send + Sync + 'static,
{
    chats_source: Arc<C>,
    messages_source: Arc<M>,
    message_sender: Arc<MS>,
    lifecycle: Arc<L>,
    result_tx: Sender<BackgroundTaskResult>,
}

impl<C, M, MS, L> ThreadTaskDispatcher<C, M, MS, L>
where
    C: ListChatsSource + Send + Sync + 'static,
    M: MessagesSource + Send + Sync + 'static,
    MS: MessageSender + Send + Sync + 'static,
    L: ChatLifecycle + ChatReadMarker + Send + Sync + 'static,
{
    pub fn new(
        chats_source: Arc<C>,
        messages_source: Arc<M>,
        message_sender: Arc<MS>,
        lifecycle: Arc<L>,
        result_tx: Sender<BackgroundTaskResult>,
    ) -> Self {
        Self {
            chats_source,
            messages_source,
            message_sender,
            lifecycle,
            result_tx,
        }
    }
}

impl<C, M, MS, L> TaskDispatcher for ThreadTaskDispatcher<C, M, MS, L>
where
    C: ListChatsSource + Send + Sync + 'static,
    M: MessagesSource + Send + Sync + 'static,
    MS: MessageSender + Send + Sync + 'static,
    L: ChatLifecycle + ChatReadMarker + Send + Sync + 'static,
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
    }
}
