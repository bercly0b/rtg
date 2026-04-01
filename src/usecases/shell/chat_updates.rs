use crate::{
    domain::{chat::ChatType, events::ChatUpdate, message::DownloadStatus},
    usecases::{background::TaskDispatcher, chat_subtitle::ChatSubtitleQuery},
};

use super::{chat_list, OrchestratorCtx};

/// Processes push updates from TDLib for cache warming and UI refresh.
///
/// - `NewMessage`: inserts into `MessageCache` for any chat (warm cache passively)
/// - `MessagesDeleted`: removes from `MessageCache`
/// - `ChatMetadataChanged`: triggers chat list refresh
///
/// For the currently open chat, also dispatches a message refresh.
pub(super) fn handle_chat_updates<D: TaskDispatcher>(
    ctx: &mut OrchestratorCtx<'_, D>,
    updates: Vec<ChatUpdate>,
) {
    let mut reload_chat_ids = Vec::new();
    let mut should_refresh_chat_list = false;

    for update in updates {
        match update {
            ChatUpdate::NewMessage { chat_id, message } => {
                tracing::debug!(chat_id, message_id = message.id, "caching pushed message");
                maybe_auto_download(ctx, chat_id, &message);
                ctx.state.message_cache_mut().add_message(chat_id, *message);
                if !reload_chat_ids.contains(&chat_id) {
                    reload_chat_ids.push(chat_id);
                }
                should_refresh_chat_list = true;
            }
            ChatUpdate::MessagesDeleted {
                chat_id,
                message_ids,
            } => {
                tracing::debug!(
                    chat_id,
                    count = message_ids.len(),
                    "removing deleted messages from cache"
                );
                ctx.state
                    .message_cache_mut()
                    .remove_messages(chat_id, &message_ids);
                if !reload_chat_ids.contains(&chat_id) {
                    reload_chat_ids.push(chat_id);
                }
                should_refresh_chat_list = true;
            }
            ChatUpdate::ChatMetadataChanged { chat_id } => {
                should_refresh_chat_list = true;
                if !reload_chat_ids.contains(&chat_id) {
                    reload_chat_ids.push(chat_id);
                }
            }
            ChatUpdate::MessageReactionsChanged {
                chat_id,
                message_id,
                reaction_count,
            } => {
                ctx.state.message_cache_mut().update_reaction_count(
                    chat_id,
                    message_id,
                    reaction_count,
                );
                if ctx.state.open_chat().chat_id() == Some(chat_id) {
                    ctx.state
                        .open_chat_mut()
                        .update_message_reaction_count(message_id, reaction_count);
                }
            }
            ChatUpdate::UserStatusChanged { user_id } => {
                // Re-resolve subtitle for the open private chat if it belongs
                // to the user whose status changed.
                if let Some(chat_id) = ctx.state.open_chat().chat_id() {
                    let chat_type = ctx.state.open_chat().chat_type();
                    if chat_type == ChatType::Private {
                        ctx.dispatcher
                            .dispatch_chat_subtitle(ChatSubtitleQuery { chat_id, chat_type });
                        tracing::debug!(
                            user_id,
                            chat_id,
                            "user status changed, re-resolving chat subtitle"
                        );
                    }
                }
                should_refresh_chat_list = true;
            }
            ChatUpdate::FileUpdated {
                file_id,
                size,
                local_path,
                is_downloading_active,
                is_downloading_completed,
                downloaded_size,
            } => {
                handle_file_update(
                    ctx,
                    file_id,
                    size,
                    local_path,
                    is_downloading_active,
                    is_downloading_completed,
                    downloaded_size,
                );
            }
        }
    }

    if should_refresh_chat_list {
        chat_list::dispatch_chat_list_refresh(ctx, false);
    }
    if !reload_chat_ids.is_empty() {
        maybe_refresh_open_chat_messages(ctx, &reload_chat_ids);
    }
}

/// Handles a file download progress/completion update from TDLib.
///
/// Updates the `FileInfo` on the relevant message in both the open chat
/// and the message cache.
fn handle_file_update<D: TaskDispatcher>(
    ctx: &mut OrchestratorCtx<'_, D>,
    file_id: i32,
    size: u64,
    local_path: String,
    is_downloading_active: bool,
    is_downloading_completed: bool,
    downloaded_size: u64,
) {
    let Some(&(chat_id, message_id)) = ctx.active_downloads.get(&file_id) else {
        return;
    };

    let new_status = if is_downloading_completed && !local_path.is_empty() {
        DownloadStatus::Completed
    } else if is_downloading_active {
        let percent = if size > 0 {
            (downloaded_size * 100 / size).min(99) as u8
        } else {
            0
        };
        DownloadStatus::Downloading {
            progress_percent: percent,
        }
    } else {
        DownloadStatus::NotStarted
    };

    let new_local_path = if is_downloading_completed && !local_path.is_empty() {
        Some(local_path)
    } else {
        None
    };

    // Update the message in the open chat view
    if ctx.state.open_chat().chat_id() == Some(chat_id) {
        ctx.state
            .open_chat_mut()
            .update_message_file_info(message_id, |fi| {
                fi.download_status = new_status;
                if let Some(ref path) = new_local_path {
                    fi.local_path = Some(path.clone());
                }
                if size > 0 {
                    fi.size = Some(size);
                }
            });
    }

    // Update the message in the cache
    ctx.state
        .message_cache_mut()
        .update_file_info(chat_id, message_id, |fi| {
            fi.download_status = new_status;
            if let Some(ref path) = new_local_path {
                fi.local_path = Some(path.clone());
            }
            if size > 0 {
                fi.size = Some(size);
            }
        });

    // Clean up completed or failed/cancelled downloads
    if is_downloading_completed {
        ctx.active_downloads.remove(&file_id);
        tracing::info!(file_id, chat_id, message_id, "file download completed");
    } else if !is_downloading_active {
        ctx.active_downloads.remove(&file_id);
        tracing::warn!(
            file_id,
            chat_id,
            message_id,
            "file download failed or cancelled"
        );
    }
}

/// Checks whether a new message should be auto-downloaded and triggers the download.
fn maybe_auto_download<D: TaskDispatcher>(
    ctx: &mut OrchestratorCtx<'_, D>,
    chat_id: i64,
    message: &crate::domain::message::Message,
) {
    let Some(ref fi) = message.file_info else {
        return;
    };

    if fi.download_status != DownloadStatus::NotStarted {
        return;
    }

    let size = fi.size.unwrap_or(0);
    if size == 0 || size > ctx.max_auto_download_bytes {
        return;
    }

    // Avoid duplicate downloads
    if ctx.active_downloads.contains_key(&fi.file_id) {
        return;
    }

    tracing::debug!(
        file_id = fi.file_id,
        size,
        message_id = message.id,
        "auto-downloading file"
    );
    ctx.active_downloads
        .insert(fi.file_id, (chat_id, message.id));
    ctx.dispatcher.dispatch_download_file(fi.file_id);
}

pub(super) fn maybe_refresh_open_chat_messages<D: TaskDispatcher>(
    ctx: &mut OrchestratorCtx<'_, D>,
    affected_chat_ids: &[i64],
) {
    use crate::domain::open_chat_state::OpenChatUiState;

    if *ctx.messages_refresh_in_flight {
        return;
    }

    let Some(open_id) = ctx.state.open_chat().chat_id() else {
        return;
    };

    if ctx.state.open_chat().ui_state() != OpenChatUiState::Ready {
        return;
    }

    if !affected_chat_ids.contains(&open_id) {
        return;
    }

    tracing::debug!(
        chat_id = open_id,
        "refreshing open chat messages from update"
    );
    *ctx.messages_refresh_in_flight = true;
    ctx.dispatcher.dispatch_load_messages(open_id);
}
