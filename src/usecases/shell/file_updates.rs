use std::collections::{HashMap, HashSet};
use std::time::Instant;

use crate::{
    domain::message::{DownloadStatus, Message},
    usecases::background::TaskDispatcher,
};

use super::ShellCtx;

/// File downloads in flight and the saves waiting for them.
pub(super) struct Downloads {
    /// Tracks active downloads: file_id → (chat_id, message_id).
    /// Used to route `updateFile` events to the correct message in the cache.
    pub active_downloads: HashMap<i32, (i64, i64)>,
    /// File IDs pending save-to-downloads after download completes.
    pub pending_saves: HashSet<i32>,
    /// Maximum file size (in bytes) for auto-download.
    pub max_auto_download_bytes: u64,
}

impl Downloads {
    pub(super) fn new(max_auto_download_bytes: u64) -> Self {
        Self {
            active_downloads: HashMap::new(),
            pending_saves: HashSet::new(),
            max_auto_download_bytes,
        }
    }
}

/// Handles a file download progress/completion update from TDLib.
///
/// Updates the `FileInfo` on the relevant message in both the open chat
/// and the message cache.
pub(super) fn handle_file_update<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    downloads: &mut Downloads,
    file_id: i32,
    size: u64,
    downloaded_size: u64,
    local_path: Option<String>,
    download_status: DownloadStatus,
) {
    let Some(&(chat_id, message_id)) = downloads.active_downloads.get(&file_id) else {
        return;
    };

    // Update the message in the open chat view
    if ctx.state.open_chat().chat_id() == Some(chat_id) {
        ctx.state
            .open_chat_mut()
            .update_message_file_info(message_id, |fi| {
                fi.download_status = download_status;
                if let Some(ref path) = local_path {
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
            fi.download_status = download_status;
            if let Some(ref path) = local_path {
                fi.local_path = Some(path.clone());
            }
            if size > 0 {
                fi.size = Some(size);
            }
        });

    // Auto-save to downloads if this file_id was requested via S hotkey.
    if let Some(ref path) = local_path {
        if downloads.pending_saves.remove(&file_id) {
            let file_name = resolve_file_name(ctx, file_id, chat_id, message_id);
            ctx.dispatcher
                .dispatch_save_file(file_id, path.clone(), file_name);
        }
    }

    // Clean up completed or failed/cancelled downloads
    match download_status {
        DownloadStatus::Completed => {
            downloads.active_downloads.remove(&file_id);
            ctx.state.download_speed_mut().finish(file_id);
            tracing::info!(file_id, chat_id, message_id, "file download completed");
        }
        DownloadStatus::NotStarted => {
            downloads.active_downloads.remove(&file_id);
            downloads.pending_saves.remove(&file_id);
            ctx.state.download_speed_mut().finish(file_id);
            tracing::warn!(
                file_id,
                chat_id,
                message_id,
                "file download failed or cancelled"
            );
        }
        DownloadStatus::Downloading { .. } => {
            ctx.state.download_speed_mut().record_progress(
                file_id,
                size,
                downloaded_size,
                Instant::now(),
            );
        }
    }
}

/// Checks whether a new message should be auto-downloaded and triggers the download.
pub(super) fn maybe_auto_download<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    downloads: &mut Downloads,
    chat_id: i64,
    message: &Message,
) {
    let Some(ref fi) = message.file_info else {
        return;
    };

    if fi.download_status != DownloadStatus::NotStarted {
        return;
    }

    let size = fi.size.unwrap_or(0);
    if size == 0 || size > downloads.max_auto_download_bytes {
        return;
    }

    // Avoid duplicate downloads
    if downloads.active_downloads.contains_key(&fi.file_id) {
        return;
    }

    tracing::debug!(
        file_id = fi.file_id,
        size,
        message_id = message.id,
        "auto-downloading file"
    );
    downloads
        .active_downloads
        .insert(fi.file_id, (chat_id, message.id));
    ctx.dispatcher.dispatch_download_file(fi.file_id);
}

fn resolve_file_name<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    _file_id: i32,
    chat_id: i64,
    message_id: i64,
) -> Option<String> {
    ctx.state.message_cache_mut().get(chat_id).and_then(|msgs| {
        msgs.iter()
            .find(|m| m.id == message_id)
            .and_then(|m| m.file_info.as_ref())
            .and_then(|fi| fi.file_name.clone())
    })
}
