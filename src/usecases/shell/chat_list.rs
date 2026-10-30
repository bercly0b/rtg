use crate::{
    domain::{chat_folder::ChatListKind, chat_list_state::ChatListUiState},
    usecases::background::TaskDispatcher,
};

use super::OrchestratorCtx;

pub(super) fn dispatch_chat_list_refresh<D: TaskDispatcher>(
    ctx: &mut OrchestratorCtx<'_, D>,
    force: bool,
) {
    if *ctx.chat_list_in_flight {
        tracing::debug!(
            force,
            "chat list refresh already in-flight, marking pending"
        );
        *ctx.chat_list_refresh_pending = true;
        *ctx.chat_list_pending_force |= force;
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

    *ctx.chat_list_in_flight = true;
    let list = ctx.state.chat_list().kind();
    let limit = ctx.state.chat_list().total_limit();
    ctx.dispatcher.dispatch_chat_list(list, force, limit);
}

pub(super) fn show_chat_folder_picker<D: TaskDispatcher>(ctx: &mut OrchestratorCtx<'_, D>) {
    if !ctx.state.open_chat_folder_picker() {
        ctx.state.set_notification("No chat folders");
    }
}

pub(super) fn handle_chat_folder_picker_key<D: TaskDispatcher>(
    ctx: &mut OrchestratorCtx<'_, D>,
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
                switch_chat_list(ctx, list);
            }
        }
        "esc" | "q" | "f" => ctx.state.close_chat_folder_picker(),
        _ => {}
    }
}

fn switch_chat_list<D: TaskDispatcher>(ctx: &mut OrchestratorCtx<'_, D>, list: ChatListKind) {
    if ctx.state.chat_list().kind() == list {
        return;
    }
    tracing::debug!(?list, "switching chat list");
    ctx.state.chat_list_mut().switch_to(list);
    dispatch_chat_list_refresh(ctx, false);
}

pub(super) fn mark_selected_chat_as_read<D: TaskDispatcher>(ctx: &mut OrchestratorCtx<'_, D>) {
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
    if *ctx.tdlib_opened_chat_id == Some(chat_id) {
        // Mark-from-chat-list is only triggered from the root list, never
        // from inside a forum topic — no topic_id to thread.
        ctx.dispatcher
            .dispatch_mark_as_read(chat_id, None, vec![last_message_id]);
    } else {
        ctx.dispatcher
            .dispatch_mark_chat_as_read(chat_id, last_message_id);
    }
}

pub(super) fn show_chat_info_popup<D: TaskDispatcher>(ctx: &mut OrchestratorCtx<'_, D>) {
    let Some(chat) = ctx.state.chat_list().selected_chat() else {
        return;
    };

    let chat_id = chat.chat_id;
    let title = chat.title.clone();
    let chat_type = chat.chat_type;

    ctx.state.show_chat_info_loading(chat_id, &title);

    ctx.dispatcher
        .dispatch_chat_info(crate::usecases::chat_subtitle::ChatInfoQuery {
            chat_id,
            chat_type,
            title,
        });
}
