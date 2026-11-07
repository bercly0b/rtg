use crate::{
    domain::{chat::ChatSummary, chat_folder::ChatListKind, chat_list_state::ChatListUiState},
    usecases::background::{BackgroundError, TaskDispatcher},
};

use super::{chat_open::OpenChatSync, ShellCtx};

/// Bookkeeping of the background chat list loads.
#[derive(Default)]
pub(super) struct ChatListLoads {
    /// Guards against dispatching duplicate chat list requests while one is in-flight.
    pub in_flight: bool,
    /// When `true`, a refresh request arrived while one was already in-flight.
    /// The in-flight result may be stale, so another refresh is dispatched
    /// automatically when the current one completes.
    pub refresh_pending: bool,
    /// Accumulated `force` flag for the pending refresh. OR-merged from all
    /// refresh requests that arrived while in-flight: if any of them was
    /// `force=true`, the pending re-dispatch will also be forced.
    pub pending_force: bool,
    /// When `true`, the current in-flight chat list refresh was triggered by the user (R key)
    /// so we should show a status-bar notification when it completes.
    pub user_requested_refresh: bool,
    /// Guards against dispatching duplicate forum badge warm-up sweeps.
    pub forum_warmup_in_flight: bool,
}

pub(super) fn dispatch_chat_list_refresh<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    loads: &mut ChatListLoads,
    force: bool,
) {
    if loads.in_flight {
        tracing::debug!(
            force,
            "chat list refresh already in-flight, marking pending"
        );
        loads.refresh_pending = true;
        loads.pending_force |= force;
        return;
    }

    tracing::debug!(force, "dispatching chat list refresh to background");

    // Only show the loader when there is no data to display (initial load,
    // after error, or empty state).  When the list is already visible
    // (Ready), keep showing stale data while the background fetch runs —
    // this prevents the "blink" where the chat list is momentarily replaced
    // by a loading indicator on every Telegram update.
    if ctx.state.chat_list().ui_state() != ChatListUiState::Ready {
        ctx.state.chat_list_mut().set_loading();
    }

    loads.in_flight = true;
    let list = ctx.state.chat_list().kind();
    let limit = ctx.state.chat_list().total_limit();
    ctx.dispatcher.dispatch_chat_list(list, force, limit);
}

pub(super) fn handle_chat_list_loaded<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    loads: &mut ChatListLoads,
    list: ChatListKind,
    result: Result<Vec<ChatSummary>, BackgroundError>,
    all_loaded: bool,
) {
    loads.in_flight = false;
    let was_pending = std::mem::take(&mut loads.refresh_pending);
    let pending_force = std::mem::take(&mut loads.pending_force);

    // When a re-dispatch is about to happen, defer the user notification
    // to the next completion — the pending result will be more up-to-date.
    let user_requested = if was_pending {
        false
    } else {
        std::mem::take(&mut loads.user_requested_refresh)
    };

    match result {
        // The user switched chat lists while this request was in
        // flight; the switch marked a refresh of the new list pending.
        _ if list != ctx.state.chat_list().kind() => {
            tracing::debug!(?list, "background: discarding stale chat list result");
        }
        Ok(chats) => {
            tracing::debug!(
                chat_count = chats.len(),
                all_loaded,
                "background: chat list loaded"
            );
            if user_requested {
                ctx.state.set_notification("Chat list refreshed");
            }
            // Forums reporting unread whose badge is unknown (topic
            // cache not seeded yet) get their counts resolved by a
            // background sweep so the list itself renders instantly.
            let warmup_ids: Vec<i64> = chats
                .iter()
                .filter(|c| c.is_forum && c.unread_count > 0 && c.unread_topic_count.is_none())
                .map(|c| c.chat_id)
                .collect();
            ctx.state.chat_list_mut().set_all_chats_loaded(all_loaded);
            ctx.state.chat_list_mut().set_ready(chats);
            if !warmup_ids.is_empty() && !loads.forum_warmup_in_flight {
                loads.forum_warmup_in_flight = true;
                ctx.dispatcher.dispatch_forum_unread_counts(warmup_ids);
            }
        }
        Err(error) => {
            tracing::warn!(code = error.code, "background: chat list load failed");
            if user_requested {
                ctx.state.set_notification("Chat list refresh failed");
            }
            ctx.state.chat_list_mut().set_error();
        }
    }

    if was_pending {
        tracing::debug!(
            pending_force,
            "re-dispatching chat list refresh (pending flag was set)"
        );
        dispatch_chat_list_refresh(ctx, loads, pending_force);
    }
}

pub(super) fn handle_forum_unread_counts_loaded<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    loads: &mut ChatListLoads,
    counts: Vec<(i64, u32)>,
) {
    loads.forum_warmup_in_flight = false;
    tracing::debug!(
        count = counts.len(),
        "background: forum unread-topic counts loaded"
    );
    for (chat_id, count) in counts {
        ctx.state
            .chat_list_mut()
            .set_forum_unread_topic_count(chat_id, count);
    }
}

/// Archives the selected chat, or unarchives it when it is archived.
///
/// The direction follows the chat's own `is_archived`, not the active list,
/// so the toggle works the same in the main list, the archive and folders.
/// A chat that leaves the active list disappears from it at once; in a
/// folder the row stays until the update-driven reload decides its fate.
pub(super) fn toggle_selected_chat_archive<D: TaskDispatcher>(ctx: &mut ShellCtx<'_, D>) {
    let Some(chat) = ctx.state.chat_list().selected_chat() else {
        return;
    };
    let chat_id = chat.chat_id;
    let archive = !chat.is_archived;
    let (from, to) = if archive {
        (ChatListKind::Main, ChatListKind::Archive)
    } else {
        (ChatListKind::Archive, ChatListKind::Main)
    };

    tracing::debug!(chat_id, archive, "toggling chat archive");
    ctx.dispatcher.dispatch_add_chat_to_list(chat_id, to);

    let chat_list = ctx.state.chat_list_mut();
    if chat_list.kind() == from {
        chat_list.remove_selected_chat();
    } else {
        chat_list.set_selected_chat_archived(archive);
    }
    ctx.state.set_notification(if archive {
        "Chat archived"
    } else {
        "Chat unarchived"
    });
}

/// Reloads the chat list so that it shows the chat where TDLib kept it,
/// undoing the optimistic change of [`toggle_selected_chat_archive`].
pub(super) fn handle_add_chat_to_list_failed<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    loads: &mut ChatListLoads,
    chat_id: i64,
    list: ChatListKind,
) {
    tracing::warn!(
        chat_id,
        ?list,
        "background: addChatToList failed, reloading chat list"
    );
    ctx.state
        .set_notification(if list == ChatListKind::Archive {
            "Failed to archive chat"
        } else {
            "Failed to unarchive chat"
        });
    dispatch_chat_list_refresh(ctx, loads, false);
}

pub(super) fn handle_chat_folder_picker_key<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    loads: &mut ChatListLoads,
    key: &str,
) {
    match key {
        "j" => {
            if let Some(picker) = ctx.state.chat_folder_picker_mut() {
                picker.select_next();
            }
        }
        "k" => {
            if let Some(picker) = ctx.state.chat_folder_picker_mut() {
                picker.select_previous();
            }
        }
        "enter" => {
            let selected = ctx
                .state
                .chat_folder_picker()
                .and_then(|picker| picker.selected())
                .map(|entry| entry.list);
            ctx.state.close_chat_folder_picker();
            if let Some(list) = selected {
                switch_chat_list(ctx, loads, list);
            }
        }
        "esc" | "q" | "f" => ctx.state.close_chat_folder_picker(),
        _ => {}
    }
}

fn switch_chat_list<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    loads: &mut ChatListLoads,
    list: ChatListKind,
) {
    if ctx.state.chat_list().kind() == list {
        return;
    }
    tracing::debug!(?list, "switching chat list");
    ctx.state.chat_list_mut().switch_to(list);
    dispatch_chat_list_refresh(ctx, loads, false);
}

pub(super) fn mark_selected_chat_as_read<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    sync: &OpenChatSync,
) {
    let Some(chat) = ctx.state.chat_list().selected_chat() else {
        return;
    };

    if chat.unread_count == 0 {
        return;
    }

    let Some(last_message_id) = chat.last_message_id else {
        return;
    };

    let chat_id = chat.chat_id;

    // Optimistic update: clear unread counter immediately in local state
    ctx.state.chat_list_mut().clear_selected_chat_unread();

    // If this chat is already opened in TDLib, just mark messages as read directly
    if sync.tdlib_opened_chat_id == Some(chat_id) {
        // Mark-from-chat-list is only triggered from the root list, never
        // from inside a forum topic — no topic_id to thread.
        ctx.dispatcher
            .dispatch_mark_as_read(chat_id, None, vec![last_message_id]);
    } else {
        ctx.dispatcher
            .dispatch_mark_chat_as_read(chat_id, last_message_id);
    }
}
