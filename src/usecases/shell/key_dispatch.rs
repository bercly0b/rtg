use anyhow::Result;

use crate::{
    domain::{keymap::Action, shell_state::ActivePane},
    usecases::background::TaskDispatcher,
};

use super::{chat_list, chat_open, message_actions, voice, OrchestratorCtx};

pub(super) fn dispatch_chat_list_action<D: TaskDispatcher>(
    ctx: &mut OrchestratorCtx<'_, D>,
    action: Action,
) -> Result<bool> {
    match action {
        Action::SelectNextChat => {
            ctx.state.chat_list_mut().select_next();
            chat_open::maybe_prefetch_selected_chat(ctx);
            if ctx.state.chat_list().needs_more_chats() && !*ctx.chat_list_in_flight {
                ctx.state.chat_list_mut().request_more_chats();
                chat_list::dispatch_chat_list_refresh(ctx, false);
            }
        }
        Action::SelectPreviousChat => {
            ctx.state.chat_list_mut().select_previous();
            chat_open::maybe_prefetch_selected_chat(ctx);
        }
        Action::SelectFirstChat => {
            ctx.state.chat_list_mut().select_first();
            chat_open::maybe_prefetch_selected_chat(ctx);
        }
        Action::RefreshChatList => {
            *ctx.user_requested_chat_refresh = true;
            ctx.state.set_notification("Refreshing chat list...");
            chat_list::dispatch_chat_list_refresh(ctx, true);
        }
        Action::MarkChatAsRead => chat_list::mark_selected_chat_as_read(ctx),
        Action::ShowChatInfo => chat_list::show_chat_info_popup(ctx),
        Action::SearchChats => ctx.state.open_chat_search(),
        Action::OpenChat => {
            if ctx.state.chat_list().selected_chat().is_some() {
                ctx.state.set_active_pane(ActivePane::Messages);
                chat_open::open_selected_chat(ctx);
                return Ok(true);
            }
        }
        Action::Quit => {
            chat_open::close_tdlib_chat(ctx);
            ctx.state.stop();
        }
        Action::ShowHelp => {
            ctx.state.show_help();
        }
        _ => {}
    }
    Ok(false)
}

pub(super) fn dispatch_messages_action<D: TaskDispatcher>(
    ctx: &mut OrchestratorCtx<'_, D>,
    action: Action,
) -> Result<()> {
    match action {
        Action::ScrollNextMessage => ctx.state.open_chat_mut().select_next(),
        Action::ScrollPreviousMessage => ctx.state.open_chat_mut().select_previous(),
        Action::BackToChatList => {
            chat_open::close_tdlib_chat(ctx);
            ctx.state.set_active_pane(ActivePane::ChatList);
        }
        Action::EnterMessageInput => {
            if ctx.state.open_chat().is_open() {
                ctx.state.set_active_pane(ActivePane::MessageInput);
            }
        }
        Action::CopyMessage => {
            if let Some(msg) = ctx.state.open_chat().selected_message() {
                let text = msg.display_content();
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    if clipboard.set_text(text).is_ok() {
                        ctx.state.set_notification("Copied to clipboard");
                    }
                }
            }
        }
        Action::OpenLink => message_actions::open_message_url(ctx)?,
        Action::OpenMessage => {
            if ctx.state.open_chat().is_open() {
                message_actions::open_selected_message(ctx);
            }
        }
        Action::RecordVoice => {
            if ctx.state.open_chat().is_open() {
                voice::start_voice_recording(ctx);
            }
        }
        Action::ShowMessageInfo => {
            show_message_info_popup(ctx);
        }
        Action::DownloadFile => {
            download_selected_message_file(ctx);
        }
        Action::SaveFile => {
            message_actions::save_selected_message_file(ctx);
        }
        Action::ReplyToMessage => {
            message_actions::reply_to_selected_message(ctx);
        }
        Action::EditMessage => {
            message_actions::edit_selected_message(ctx);
        }
        Action::DeleteMessage => {
            message_actions::delete_selected_message(ctx);
        }
        Action::Quit => {
            chat_open::close_tdlib_chat(ctx);
            ctx.state.stop();
        }
        Action::ShowHelp => {
            ctx.state.show_help();
        }
        _ => {}
    }
    Ok(())
}

fn show_message_info_popup<D: TaskDispatcher>(ctx: &mut OrchestratorCtx<'_, D>) {
    let Some(chat_id) = ctx.state.open_chat().chat_id() else {
        return;
    };

    let Some(msg) = ctx.state.open_chat().selected_message() else {
        return;
    };

    let message_id = msg.id;
    let is_outgoing = msg.is_outgoing;

    ctx.state.show_message_info_loading(chat_id, message_id);

    ctx.dispatcher
        .dispatch_message_info(crate::usecases::message_info::MessageInfoQuery {
            chat_id,
            message_id,
            is_outgoing,
        });
}

fn download_selected_message_file<D: TaskDispatcher>(ctx: &mut OrchestratorCtx<'_, D>) {
    use crate::domain::message::DownloadStatus;

    let Some(chat_id) = ctx.state.open_chat().chat_id() else {
        return;
    };

    let Some(msg) = ctx.state.open_chat().selected_message() else {
        return;
    };

    let Some(ref fi) = msg.file_info else {
        return;
    };

    if fi.download_status != DownloadStatus::NotStarted {
        return;
    }

    if ctx.active_downloads.contains_key(&fi.file_id) {
        return;
    }

    let file_id = fi.file_id;
    let message_id = msg.id;

    tracing::info!(
        file_id,
        chat_id,
        message_id,
        "manual file download triggered"
    );
    ctx.active_downloads.insert(file_id, (chat_id, message_id));
    ctx.dispatcher.dispatch_download_file(file_id);
}
