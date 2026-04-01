use std::sync::{mpsc::Sender, Arc};

use crate::{
    domain::events::{BackgroundError, BackgroundTaskResult},
    usecases::{
        chat_lifecycle::{ChatLifecycle, ChatReadMarker, FileDownloader, MessageDeleter},
        chat_subtitle::{ChatInfoQuery, ChatSubtitleQuery, ChatSubtitleSource},
        list_chats::{list_chats, ListChatsQuery, ListChatsSource},
    },
};

use super::error_mapping::map_list_chats_error;

pub(super) fn dispatch_chat_list<C: ListChatsSource + Send + Sync + 'static>(
    source: &Arc<C>,
    tx: &Sender<BackgroundTaskResult>,
    force: bool,
) {
    let source = Arc::clone(source);
    let tx = tx.clone();
    let tx_fallback = tx.clone();

    let spawn_result = std::thread::Builder::new()
        .name("rtg-bg-chat-list".into())
        .spawn(move || {
            tracing::debug!(force, "background: fetching chat list");
            let query = ListChatsQuery {
                force,
                ..ListChatsQuery::default()
            };
            let result = list_chats(source.as_ref(), query)
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

pub(super) fn dispatch_open_chat<L: ChatLifecycle + Send + Sync + 'static>(
    lifecycle: &Arc<L>,
    chat_id: i64,
) {
    let lifecycle = Arc::clone(lifecycle);

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

pub(super) fn dispatch_close_chat<L: ChatLifecycle + Send + Sync + 'static>(
    lifecycle: &Arc<L>,
    chat_id: i64,
) {
    let lifecycle = Arc::clone(lifecycle);

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

pub(super) fn dispatch_mark_as_read<L: ChatReadMarker + Send + Sync + 'static>(
    lifecycle: &Arc<L>,
    chat_id: i64,
    message_ids: Vec<i64>,
) {
    let lifecycle = Arc::clone(lifecycle);

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

pub(super) fn dispatch_mark_chat_as_read<
    L: ChatLifecycle + ChatReadMarker + Send + Sync + 'static,
>(
    lifecycle: &Arc<L>,
    chat_id: i64,
    last_message_id: i64,
) {
    let lifecycle = Arc::clone(lifecycle);

    if let Err(error) = std::thread::Builder::new()
        .name("rtg-bg-mark-chat-read".into())
        .spawn(move || {
            tracing::debug!(chat_id, last_message_id, "background: marking chat as read from chat list");
            if let Err(e) = lifecycle.open_chat(chat_id) {
                tracing::warn!(chat_id, error = ?e, "background: openChat failed during mark-chat-read");
                return;
            }
            if let Err(e) = lifecycle.mark_messages_read(chat_id, vec![last_message_id]) {
                tracing::warn!(chat_id, error = ?e, "background: viewMessages failed during mark-chat-read");
            }
            if let Err(e) = lifecycle.close_chat(chat_id) {
                tracing::warn!(chat_id, error = ?e, "background: closeChat failed during mark-chat-read");
            }
        })
    {
        tracing::error!(error = %error, "failed to spawn mark-chat-as-read background thread");
    }
}

pub(super) fn dispatch_delete_message<L: MessageDeleter + Send + Sync + 'static>(
    lifecycle: &Arc<L>,
    chat_id: i64,
    message_id: i64,
) {
    let lifecycle = Arc::clone(lifecycle);

    if let Err(error) = std::thread::Builder::new()
        .name("rtg-bg-delete-msg".into())
        .spawn(move || {
            tracing::debug!(chat_id, message_id, "background: deleting message");
            let ids = vec![message_id];
            if let Err(e) = lifecycle.delete_messages(chat_id, ids.clone(), true) {
                tracing::debug!(chat_id, message_id, error = ?e, "revoke delete failed, trying self-only");
                if let Err(e2) = lifecycle.delete_messages(chat_id, ids, false) {
                    tracing::warn!(chat_id, message_id, error = ?e2, "self-only delete also failed");
                }
            }
        })
    {
        tracing::error!(error = %error, "failed to spawn delete message background thread");
    }
}

pub(super) fn dispatch_chat_subtitle<S: ChatSubtitleSource + Send + Sync + 'static>(
    source: &Arc<S>,
    tx: &Sender<BackgroundTaskResult>,
    query: ChatSubtitleQuery,
) {
    let source = Arc::clone(source);
    let tx = tx.clone();
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

pub(super) fn dispatch_chat_info<S: ChatSubtitleSource + Send + Sync + 'static>(
    source: &Arc<S>,
    tx: &Sender<BackgroundTaskResult>,
    query: ChatInfoQuery,
) {
    let source = Arc::clone(source);
    let tx = tx.clone();
    let tx_fallback = tx.clone();
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

pub(super) fn dispatch_download_file<L: FileDownloader + Send + Sync + 'static>(
    lifecycle: &Arc<L>,
    file_id: i32,
) {
    let lifecycle = Arc::clone(lifecycle);

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
