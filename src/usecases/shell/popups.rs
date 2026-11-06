use crate::{
    domain::{
        chat_info_state::{ChatInfo, ChatInfoPopupState},
        message_info_state::{MessageInfo, MessageInfoPopupState},
        reaction_picker_state::{AvailableReaction, ReactionPickerData, ReactionPickerState},
    },
    usecases::{
        background::{BackgroundError, TaskDispatcher},
        chat_subtitle::ChatInfoQuery,
        message_info::MessageInfoQuery,
        message_reactions::AvailableReactionsQuery,
    },
};

use super::ShellCtx;

pub(super) fn show_chat_info_popup<D: TaskDispatcher>(ctx: &mut ShellCtx<'_, D>) {
    let Some(chat) = ctx.state.chat_list().selected_chat() else {
        return;
    };

    let chat_id = chat.chat_id;
    let title = chat.title.clone();
    let chat_type = chat.chat_type;

    ctx.state.show_chat_info_loading(chat_id, &title);

    ctx.dispatcher.dispatch_chat_info(ChatInfoQuery {
        chat_id,
        chat_type,
        title,
    });
}

pub(super) fn show_message_info_popup<D: TaskDispatcher>(ctx: &mut ShellCtx<'_, D>) {
    let Some(chat_id) = ctx.state.open_chat().chat_id() else {
        return;
    };

    let Some(msg) = ctx.state.open_chat().selected_message() else {
        return;
    };

    let message_id = msg.id;
    let is_outgoing = msg.is_outgoing;

    ctx.state.show_message_info_loading(chat_id, message_id);

    ctx.dispatcher.dispatch_message_info(MessageInfoQuery {
        chat_id,
        message_id,
        is_outgoing,
    });
}

pub(super) fn show_reaction_picker<D: TaskDispatcher>(ctx: &mut ShellCtx<'_, D>) {
    let Some(chat_id) = ctx.state.open_chat().chat_id() else {
        return;
    };

    let Some(msg) = ctx.state.open_chat().selected_message() else {
        return;
    };

    if msg.is_pending() {
        return;
    }

    let message_id = msg.id;

    ctx.state.show_reaction_picker_loading(chat_id, message_id);

    ctx.dispatcher
        .dispatch_available_reactions(AvailableReactionsQuery {
            chat_id,
            message_id,
        });
}

pub(super) fn handle_chat_info_popup_key<D: TaskDispatcher>(ctx: &mut ShellCtx<'_, D>, key: &str) {
    if matches!(key, "q" | "esc" | "I") {
        ctx.state.close_chat_info_popup();
    }
}

pub(super) fn handle_message_info_popup_key<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    key: &str,
) {
    if matches!(key, "q" | "esc" | "I") {
        ctx.state.close_message_info_popup();
    }
}

pub(super) fn handle_help_key<D: TaskDispatcher>(ctx: &mut ShellCtx<'_, D>, key: &str) {
    if matches!(key, "q" | "?" | "esc") {
        ctx.state.hide_help();
    }
}

pub(super) fn handle_reaction_picker_key<D: TaskDispatcher>(ctx: &mut ShellCtx<'_, D>, key: &str) {
    match key {
        "j" => {
            if let Some(data) = ctx.state.reaction_picker_mut().and_then(|p| p.data_mut()) {
                data.select_next();
            }
        }
        "k" => {
            if let Some(data) = ctx.state.reaction_picker_mut().and_then(|p| p.data_mut()) {
                data.select_previous();
            }
        }
        "enter" => {
            let action = ctx.state.reaction_picker().and_then(|picker| {
                if let ReactionPickerState::Ready(data) = picker {
                    data.selected_reaction()
                        .map(|r| (data.chat_id, data.message_id, r.emoji.clone(), r.is_chosen))
                } else {
                    None
                }
            });

            if let Some((chat_id, message_id, emoji, is_chosen)) = action {
                if is_chosen {
                    ctx.dispatcher
                        .dispatch_remove_reaction(chat_id, message_id, emoji);
                } else {
                    ctx.dispatcher
                        .dispatch_add_reaction(chat_id, message_id, emoji);
                }
                ctx.state.close_reaction_picker();
            }
        }
        "esc" | "q" | "R" => {
            ctx.state.close_reaction_picker();
        }
        _ => {}
    }
}

pub(super) fn handle_chat_info_loaded<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    chat_id: i64,
    result: Result<ChatInfo, BackgroundError>,
) {
    let popup_chat_id = ctx.state.chat_info_popup().and_then(|p| p.chat_id());
    if popup_chat_id != Some(chat_id) {
        return;
    }
    match result {
        Ok(info) => {
            tracing::debug!(chat_id, "chat info resolved for popup");
            ctx.state
                .set_chat_info_loaded(ChatInfoPopupState::Loaded(info));
        }
        Err(e) => {
            tracing::debug!(chat_id, code = e.code, "chat info resolution failed");
            let title = ctx
                .state
                .chat_info_popup()
                .map(|p| p.title().to_owned())
                .unwrap_or_default();
            ctx.state
                .set_chat_info_loaded(ChatInfoPopupState::Error { title });
        }
    }
}

pub(super) fn handle_message_info_loaded<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    chat_id: i64,
    message_id: i64,
    result: Result<MessageInfo, BackgroundError>,
) {
    let popup_ids = ctx.state.message_info_popup().and_then(|p| p.ids());
    if popup_ids != Some((chat_id, message_id)) {
        return;
    }
    match result {
        Ok(info) => {
            tracing::debug!(chat_id, message_id, "message info resolved for popup");
            ctx.state
                .set_message_info_loaded(MessageInfoPopupState::Loaded(info));
        }
        Err(e) => {
            tracing::debug!(
                chat_id,
                message_id,
                code = e.code,
                "message info resolution failed"
            );
            ctx.state
                .set_message_info_loaded(MessageInfoPopupState::Error);
        }
    }
}

pub(super) fn handle_available_reactions_loaded<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    chat_id: i64,
    message_id: i64,
    result: Result<Vec<AvailableReaction>, BackgroundError>,
) {
    let picker_ids = ctx.state.reaction_picker().and_then(|p| p.ids());
    if picker_ids != Some((chat_id, message_id)) {
        return;
    }
    match result {
        Ok(reactions) => {
            tracing::debug!(
                chat_id,
                message_id,
                count = reactions.len(),
                "available reactions loaded"
            );
            if reactions.is_empty() {
                ctx.state.close_reaction_picker();
                ctx.state.set_notification("No reactions available");
            } else {
                ctx.state
                    .set_reaction_picker(ReactionPickerState::Ready(ReactionPickerData::new(
                        reactions, chat_id, message_id,
                    )));
            }
        }
        Err(e) => {
            tracing::debug!(
                chat_id,
                message_id,
                code = e.code,
                "available reactions load failed"
            );
            ctx.state.set_reaction_picker(ReactionPickerState::Error);
        }
    }
}
