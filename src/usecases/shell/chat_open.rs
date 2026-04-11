use crate::{
    domain::{
        chat::ChatType,
        open_chat_state::{MessageSource, OpenChatUiState},
        shell_state::ActivePane,
    },
    usecases::{background::TaskDispatcher, chat_subtitle::ChatSubtitleQuery},
};

use super::{OrchestratorCtx, DEFAULT_CACHED_MESSAGES_LIMIT};

pub(super) fn open_selected_chat<D: TaskDispatcher>(ctx: &mut OrchestratorCtx<'_, D>) {
    let Some(selected) = ctx.state.chat_list().selected_chat() else {
        return;
    };

    let chat_id = selected.chat_id;
    let chat_title = selected.title.clone();
    let chat_type = selected.chat_type;

    // Cancel any in-flight prefetch — the user explicitly opened a chat.
    *ctx.prefetch_in_flight = None;

    // If the same chat is already open and Ready, just switch focus — no reload.
    // But always ensure the TDLib lifecycle is maintained.
    if ctx.state.open_chat().chat_id() == Some(chat_id)
        && ctx.state.open_chat().ui_state() == OpenChatUiState::Ready
    {
        tracing::debug!(chat_id, "chat already open and ready, skipping reload");
        // Re-open in TDLib if it was closed (e.g. user pressed h then l)
        if *ctx.tdlib_opened_chat_id != Some(chat_id) {
            ctx.dispatcher.dispatch_open_chat(chat_id);
            *ctx.tdlib_opened_chat_id = Some(chat_id);
            // Mark existing messages as read in the reopened chat
            mark_open_chat_messages_as_read(ctx);
        }
        return;
    }

    tracing::debug!(chat_id, chat_title = %chat_title, "opening chat (non-blocking)");

    // Close the previously opened TDLib chat if switching to a different one.
    close_tdlib_chat_if_needed(ctx, chat_id);

    // Open this chat in TDLib for update delivery and read tracking.
    ctx.dispatcher.dispatch_open_chat(chat_id);
    *ctx.tdlib_opened_chat_id = Some(chat_id);

    // Try app-level message cache first (instant, no TDLib call).
    // Fall back to TDLib local cache if the app cache has no data.
    // Apply the smart threshold: if cache has fewer than min_display_messages,
    // show Loading instead of a sparse preview (eliminates the "1 message flash").
    let min_msgs = ctx.min_display_messages;
    let showed_cache = if let Some(cached) = ctx
        .state
        .message_cache_mut()
        .get(chat_id)
        .filter(|m| m.len() >= min_msgs)
    {
        let messages = cached.to_vec();
        tracing::debug!(
            chat_id,
            count = messages.len(),
            "showing messages from app cache"
        );
        ctx.state
            .open_chat_mut()
            .set_loading(chat_id, chat_title.clone(), chat_type);
        ctx.state.open_chat_mut().set_ready(messages);
        ctx.state.open_chat_mut().set_refreshing(true);
        ctx.state
            .open_chat_mut()
            .set_message_source(MessageSource::Cache);
        true
    } else {
        try_show_cached_messages(ctx, chat_id, &chat_title, chat_type)
    };

    if !showed_cache {
        ctx.state
            .open_chat_mut()
            .set_loading(chat_id, chat_title, chat_type);
    }

    // Dispatch a full background load (pagination).
    *ctx.messages_refresh_in_flight = true;
    ctx.dispatcher.dispatch_load_messages(chat_id);

    // Dispatch subtitle resolution (user status / member count).
    ctx.dispatcher
        .dispatch_chat_subtitle(ChatSubtitleQuery { chat_id, chat_type });
}

/// Prefetches messages for the currently highlighted chat in the chat list.
///
/// Triggered by j/k navigation. Skips if:
/// - Another prefetch is already in-flight (debounce)
/// - The highlighted chat already has data in the message cache
pub(super) fn maybe_prefetch_selected_chat<D: TaskDispatcher>(ctx: &mut OrchestratorCtx<'_, D>) {
    if ctx.prefetch_in_flight.is_some() {
        return;
    }

    let Some(selected) = ctx.state.chat_list().selected_chat() else {
        return;
    };

    let chat_id = selected.chat_id;

    if ctx.state.message_cache().has_messages(chat_id) {
        return;
    }

    tracing::debug!(chat_id, "prefetching messages for highlighted chat");
    *ctx.prefetch_in_flight = Some(chat_id);
    ctx.dispatcher.dispatch_prefetch_messages(chat_id);
}

/// Closes the currently TDLib-opened chat if it differs from `next_chat_id`.
///
/// Called before opening a new chat or when navigating away.
pub(super) fn close_tdlib_chat_if_needed<D: TaskDispatcher>(
    ctx: &mut OrchestratorCtx<'_, D>,
    next_chat_id: i64,
) {
    if let Some(prev_id) = *ctx.tdlib_opened_chat_id {
        if prev_id != next_chat_id {
            tracing::debug!(prev_id, "closing previous TDLib chat");
            ctx.dispatcher.dispatch_close_chat(prev_id);
            *ctx.tdlib_opened_chat_id = None;
        }
    }
}

/// Closes the currently TDLib-opened chat unconditionally.
pub(super) fn close_tdlib_chat<D: TaskDispatcher>(ctx: &mut OrchestratorCtx<'_, D>) {
    if let Some(chat_id) = ctx.tdlib_opened_chat_id.take() {
        tracing::debug!(chat_id, "closing TDLib chat on navigate away");
        ctx.dispatcher.dispatch_close_chat(chat_id);
        *ctx.messages_refresh_in_flight = false;
    }
}

/// Dispatches a mark-as-read request for all messages currently loaded in the open chat.
///
/// Only marks messages when the user is actively viewing the chat
/// (Messages or MessageInput pane has focus). This prevents incoming
/// messages from being auto-read while the user is browsing the chat list.
pub(super) fn mark_open_chat_messages_as_read<D: TaskDispatcher>(ctx: &mut OrchestratorCtx<'_, D>) {
    if !matches!(
        ctx.state.active_pane(),
        ActivePane::Messages | ActivePane::MessageInput
    ) {
        return;
    }

    let Some(chat_id) = ctx.state.open_chat().chat_id() else {
        return;
    };

    let messages = ctx.state.open_chat().messages();
    if messages.is_empty() {
        return;
    }

    let message_ids: Vec<i64> = messages.iter().map(|m| m.id).collect();
    ctx.dispatcher.dispatch_mark_as_read(chat_id, message_ids);
}

/// Attempts to synchronously load cached messages for instant display.
///
/// Returns `true` if cached messages were found (above the smart threshold)
/// and the state was set to Ready. Sparse results below the threshold are
/// ignored to avoid the "1 message flash" artifact.
pub(super) fn try_show_cached_messages<D: TaskDispatcher>(
    ctx: &mut OrchestratorCtx<'_, D>,
    chat_id: i64,
    chat_title: &str,
    chat_type: ChatType,
) -> bool {
    let Some(cache) = ctx.cache_source else {
        return false;
    };

    match cache.list_cached_messages(chat_id, DEFAULT_CACHED_MESSAGES_LIMIT) {
        Ok(messages) if messages.len() >= ctx.min_display_messages => {
            tracing::debug!(
                chat_id,
                count = messages.len(),
                "showing cached messages instantly"
            );
            ctx.state
                .open_chat_mut()
                .set_loading(chat_id, chat_title.to_owned(), chat_type);
            ctx.state.open_chat_mut().set_ready(messages);
            ctx.state.open_chat_mut().set_refreshing(true);
            ctx.state
                .open_chat_mut()
                .set_message_source(MessageSource::Cache);
            true
        }
        Ok(_) => {
            tracing::debug!(
                chat_id,
                "no/sparse cached messages, skipping instant display"
            );
            false
        }
        Err(e) => {
            tracing::debug!(chat_id, error = ?e, "failed to load cached messages");
            false
        }
    }
}
