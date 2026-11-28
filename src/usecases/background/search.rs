use std::sync::{mpsc::Sender, Arc};

use crate::{domain::global_search_state::ChatScope, usecases::global_search::GlobalSearchSource};

use super::{BackgroundError, BackgroundTaskResult};

const SEARCH_UNAVAILABLE: &str = "GLOBAL_SEARCH_UNAVAILABLE";
const THREAD_SPAWN_FAILED: &str = "THREAD_SPAWN_FAILED";

pub(super) fn dispatch_search_chats<C: GlobalSearchSource + Send + Sync + 'static>(
    source: &Arc<C>,
    tx: &Sender<BackgroundTaskResult>,
    query: String,
    scope: ChatScope,
) {
    let source = Arc::clone(source);
    let tx = tx.clone();
    let tx_fallback = tx.clone();
    let query_fallback = query.clone();

    let spawn_result = std::thread::Builder::new()
        .name("rtg-bg-search-chats".into())
        .spawn(move || {
            tracing::debug!(?scope, "background: searching chats");
            let result = source
                .search_chats(&query, scope)
                .map_err(|_| BackgroundError::new(SEARCH_UNAVAILABLE));

            let _ = tx.send(BackgroundTaskResult::ChatsFound {
                query,
                scope,
                result,
            });
        });

    if let Err(error) = spawn_result {
        tracing::error!(error = %error, "failed to spawn the background thread of a search for chats");
        let _ = tx_fallback.send(BackgroundTaskResult::ChatsFound {
            query: query_fallback,
            scope,
            result: Err(BackgroundError::new(THREAD_SPAWN_FAILED)),
        });
    }
}

pub(super) fn dispatch_search_messages<C: GlobalSearchSource + Send + Sync + 'static>(
    source: &Arc<C>,
    tx: &Sender<BackgroundTaskResult>,
    query: String,
    offset: String,
) {
    let source = Arc::clone(source);
    let tx = tx.clone();
    let tx_fallback = tx.clone();
    let query_fallback = query.clone();
    let offset_fallback = offset.clone();

    let spawn_result = std::thread::Builder::new()
        .name("rtg-bg-search-messages".into())
        .spawn(move || {
            tracing::debug!("background: searching messages");
            let result = source
                .search_messages(&query, &offset)
                .map_err(|_| BackgroundError::new(SEARCH_UNAVAILABLE));

            let _ = tx.send(BackgroundTaskResult::MessagesFound {
                query,
                offset,
                result,
            });
        });

    if let Err(error) = spawn_result {
        tracing::error!(error = %error, "failed to spawn message search background thread");
        let _ = tx_fallback.send(BackgroundTaskResult::MessagesFound {
            query: query_fallback,
            offset: offset_fallback,
            result: Err(BackgroundError::new(THREAD_SPAWN_FAILED)),
        });
    }
}
