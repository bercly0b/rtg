use std::sync::Arc;

use crate::{
    domain::{
        chat::{ChatSummary, ChatType},
        open_chat_state::{MessageSource, OpenChatUiState},
        shell_state::ActivePane,
    },
    usecases::{
        background::{BackgroundError, TaskDispatcher},
        chat_subtitle::ChatSubtitleQuery,
        load_messages::CachedMessagesSource,
    },
};

use super::{ShellCtx, DEFAULT_CACHED_MESSAGES_LIMIT};

/// Keeps the open chat in sync with TDLib: which chat TDLib has open, which
/// message loads are in flight, and where cached messages come from.
pub(super) struct OpenChatSync {
    /// Tracks the chat_id that is currently "opened" in TDLib via `openChat`.
    /// Used to ensure proper `closeChat` pairing when navigating away.
    pub tdlib_opened_chat_id: Option<i64>,
    /// Guards against dispatching duplicate prefetch requests.
    /// Holds the `chat_id` of the currently in-flight prefetch, if any.
    pub prefetch_in_flight: Option<i64>,
    /// Guards against dispatching duplicate message refresh requests while one is in-flight.
    pub messages_refresh_in_flight: bool,
    /// Guards against dispatching duplicate older-messages pagination requests.
    pub older_messages_in_flight: bool,
    /// Minimum number of cached messages required to display them immediately.
    /// If the cache holds fewer messages, the UI shows Loading instead of a
    /// sparse preview (eliminates the "1 message flash" artifact).
    pub min_display_messages: usize,
    /// Synchronous cache source for instant message display.
    pub cache_source: Option<Arc<dyn CachedMessagesSource>>,
    /// The mention whose chat is being looked up.
    pub pending_mention: Option<PendingMention>,
}

/// A mention lookup in flight and the place the user started it from: the
/// found chat opens only if they are still there.
pub(super) struct PendingMention {
    username: String,
    from_chat_id: Option<i64>,
    from_topic_id: Option<i32>,
}

impl OpenChatSync {
    pub(super) fn new(
        cache_source: Option<Arc<dyn CachedMessagesSource>>,
        min_display_messages: usize,
    ) -> Self {
        Self {
            tdlib_opened_chat_id: None,
            prefetch_in_flight: None,
            messages_refresh_in_flight: false,
            older_messages_in_flight: false,
            min_display_messages: min_display_messages.max(1),
            cache_source,
            pending_mention: None,
        }
    }
}

pub(super) fn open_selected_chat<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    sync: &mut OpenChatSync,
) {
    let Some(selected) = ctx.state.chat_list().selected_chat() else {
        return;
    };

    let chat_id = selected.chat_id;
    let chat_title = selected.title.clone();
    let chat_type = selected.chat_type;
    let is_forum = selected.is_forum;

    open_chat(ctx, sync, chat_id, chat_title, chat_type, is_forum);
}

/// Opens a chat by its identity, whether or not the chat list holds it.
fn open_chat<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    sync: &mut OpenChatSync,
    chat_id: i64,
    chat_title: String,
    chat_type: ChatType,
    is_forum: bool,
) {
    // Forum chats route into the topic-list panel rather than directly into
    // the messages view. The active pane stays ChatList (left panel just
    // renders topics instead of chats) — see ui/view/chat_list.rs.
    if is_forum {
        super::forum::enter_forum(ctx, sync, chat_id, chat_title);
        return;
    }

    // Non-forum chats focus the messages pane.
    ctx.state.set_active_pane(ActivePane::Messages);

    // Cancel any in-flight prefetch — the user explicitly opened a chat.
    sync.prefetch_in_flight = None;

    // If the same chat is already open and Ready, or its load is still in
    // flight, just switch focus — no reload. But always ensure the TDLib
    // lifecycle is maintained.
    let loaded_or_loading = match ctx.state.open_chat().ui_state() {
        OpenChatUiState::Ready => true,
        OpenChatUiState::Loading => sync.messages_refresh_in_flight,
        OpenChatUiState::Empty | OpenChatUiState::Error => false,
    };
    if ctx.state.open_chat().chat_id() == Some(chat_id) && loaded_or_loading {
        tracing::debug!(chat_id, "chat already open, skipping reload");
        // Re-open in TDLib if it was closed (e.g. user pressed h then l)
        if sync.tdlib_opened_chat_id != Some(chat_id) {
            ctx.dispatcher.dispatch_open_chat(chat_id);
            sync.tdlib_opened_chat_id = Some(chat_id);
            // Mark messages that arrived while the chat was closed as read
            mark_open_chat_messages_as_read(ctx);
        }
        return;
    }

    tracing::debug!(chat_id, chat_title = %chat_title, "opening chat (non-blocking)");

    // Close the previously opened TDLib chat if switching to a different one.
    close_tdlib_chat_if_needed(ctx, sync, chat_id);

    // Open this chat in TDLib for update delivery and read tracking. Not
    // ordered with the history load below — see `TaskDispatcher::dispatch_open_chat`.
    ctx.dispatcher.dispatch_open_chat(chat_id);
    sync.tdlib_opened_chat_id = Some(chat_id);

    // Try app-level message cache first (instant, no TDLib call).
    // Fall back to TDLib local cache if the app cache has no data.
    // Apply the smart threshold: if cache has fewer than min_display_messages,
    // show Loading instead of a sparse preview (eliminates the "1 message flash").
    let min_msgs = sync.min_display_messages;
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
            .set_chat_identity(chat_id, chat_title.clone(), chat_type);
        ctx.state.open_chat_mut().set_ready(messages);
        ctx.state.open_chat_mut().set_refreshing(true);
        ctx.state
            .open_chat_mut()
            .set_message_source(MessageSource::Cache);
        true
    } else {
        try_show_cached_messages(ctx, sync, chat_id, &chat_title, chat_type)
    };

    if !showed_cache {
        ctx.state
            .open_chat_mut()
            .set_loading(chat_id, chat_title, chat_type);
    }

    // Dispatch a full background load (pagination).
    sync.messages_refresh_in_flight = true;
    let topic_id = ctx.state.open_chat().topic_id();
    ctx.dispatcher.dispatch_load_messages(chat_id, topic_id);

    // Dispatch subtitle resolution (user status / member count).
    ctx.dispatcher
        .dispatch_chat_subtitle(ChatSubtitleQuery { chat_id, chat_type });
}

/// Starts opening the chat behind the mention of `username`: the lookup may
/// need the server, so it runs in the background and
/// [`handle_mention_resolved`] opens the chat.
pub(super) fn open_mention<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    sync: &mut OpenChatSync,
    username: String,
) {
    ctx.state
        .set_notification(format!("Opening @{username}..."));
    sync.pending_mention = Some(PendingMention {
        username: username.clone(),
        from_chat_id: ctx.state.open_chat().chat_id(),
        from_topic_id: ctx.state.open_chat().topic_id(),
    });
    ctx.dispatcher.dispatch_resolve_mention(username);
}

/// Opens the chat found for a mention, unless the user has moved on since
/// asking: a late answer must not override what they did meanwhile.
pub(super) fn handle_mention_resolved<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    sync: &mut OpenChatSync,
    username: &str,
    result: Result<Option<ChatSummary>, BackgroundError>,
) {
    let Some(pending) = sync
        .pending_mention
        .take_if(|pending| pending.username == username)
    else {
        tracing::debug!("background: discarding superseded mention result");
        return;
    };

    let current = ctx.state.open_chat();
    if ctx.state.active_pane() != ActivePane::Messages
        || current.chat_id() != pending.from_chat_id
        || current.topic_id() != pending.from_topic_id
    {
        tracing::debug!("background: discarding stale mention result (user navigated away)");
        return;
    }

    let chat = match result {
        Ok(Some(chat)) => chat,
        Ok(None) => {
            ctx.state.set_notification(format!("@{username} not found"));
            return;
        }
        Err(error) => {
            tracing::warn!(code = error.code, "background: mention lookup failed");
            ctx.state
                .set_notification(format!("Could not open @{username}"));
            return;
        }
    };

    if Some(chat.chat_id) == pending.from_chat_id {
        return;
    }

    // The topic list of the forum the user is leaving would otherwise keep
    // covering the chat list.
    if ctx.state.forum_topic_list().is_some() {
        super::forum::leave_forum(ctx, sync);
    }
    ctx.state.chat_list_mut().select_chat(chat.chat_id);
    open_chat(
        ctx,
        sync,
        chat.chat_id,
        chat.title,
        chat.chat_type,
        chat.is_forum,
    );
}

/// Prefetches messages for the currently highlighted chat in the chat list.
///
/// Triggered by j/k navigation. Skips if:
/// - Another prefetch is already in-flight (debounce)
/// - The highlighted chat already has data in the message cache
pub(super) fn maybe_prefetch_selected_chat<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    sync: &mut OpenChatSync,
) {
    if sync.prefetch_in_flight.is_some() {
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
    sync.prefetch_in_flight = Some(chat_id);
    // Prefetch from the chat-list hover; never scoped to a forum topic.
    ctx.dispatcher.dispatch_prefetch_messages(chat_id, None);
}

/// Closes the currently TDLib-opened chat if it differs from `next_chat_id`.
///
/// Called before opening a new chat or when navigating away.
pub(super) fn close_tdlib_chat_if_needed<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    sync: &mut OpenChatSync,
    next_chat_id: i64,
) {
    if let Some(prev_id) = sync.tdlib_opened_chat_id {
        if prev_id != next_chat_id {
            tracing::debug!(prev_id, "closing previous TDLib chat");
            ctx.dispatcher.dispatch_close_chat(prev_id);
            sync.tdlib_opened_chat_id = None;
        }
    }
}

/// Closes the currently TDLib-opened chat unconditionally.
///
/// Keeps `messages_refresh_in_flight`: the open chat state survives
/// navigating away, so the in-flight load still lands in it and clears the
/// flag itself — reopening the chat meanwhile must not load it again.
pub(super) fn close_tdlib_chat<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    sync: &mut OpenChatSync,
) {
    if let Some(chat_id) = sync.tdlib_opened_chat_id.take() {
        tracing::debug!(chat_id, "closing TDLib chat on navigate away");
        ctx.dispatcher.dispatch_close_chat(chat_id);
    }
}

/// Dispatches a mark-as-read request for the loaded messages of the open chat
/// that have not been marked yet.
///
/// Only marks messages when the user is actively viewing the chat
/// (Messages or MessageInput pane has focus). This prevents incoming
/// messages from being auto-read while the user is browsing the chat list.
pub(super) fn mark_open_chat_messages_as_read<D: TaskDispatcher>(ctx: &mut ShellCtx<'_, D>) {
    if !matches!(
        ctx.state.active_pane(),
        ActivePane::Messages | ActivePane::MessageInput
    ) {
        return;
    }

    let Some(chat_id) = ctx.state.open_chat().chat_id() else {
        return;
    };

    let message_ids = ctx.state.open_chat_mut().take_unacked_message_ids();
    if message_ids.is_empty() {
        return;
    }

    let topic_id = ctx.state.open_chat().topic_id();
    ctx.dispatcher
        .dispatch_mark_as_read(chat_id, topic_id, message_ids);
}

/// Attempts to synchronously load cached messages for instant display.
///
/// Returns `true` if cached messages were found (above the smart threshold)
/// and the state was set to Ready. Sparse results below the threshold are
/// ignored to avoid the "1 message flash" artifact.
pub(super) fn try_show_cached_messages<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    sync: &OpenChatSync,
    chat_id: i64,
    chat_title: &str,
    chat_type: ChatType,
) -> bool {
    let Some(cache) = &sync.cache_source else {
        return false;
    };

    match cache.list_cached_messages(chat_id, DEFAULT_CACHED_MESSAGES_LIMIT) {
        Ok(messages) if messages.len() >= sync.min_display_messages => {
            tracing::debug!(
                chat_id,
                count = messages.len(),
                "showing cached messages instantly"
            );
            ctx.state
                .open_chat_mut()
                .set_chat_identity(chat_id, chat_title.to_owned(), chat_type);
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
