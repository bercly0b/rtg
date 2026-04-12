use anyhow::Result;

use crate::{domain::shell_state::ActivePane, usecases::background::TaskDispatcher};

use super::{chat_open, message_actions, OrchestratorCtx};

pub(super) fn handle_messages_key<D: TaskDispatcher>(
    ctx: &mut OrchestratorCtx<'_, D>,
    key: &str,
) -> Result<()> {
    // Vim-style `dd`: first `d` sets pending, second `d` triggers delete.
    if key == "d" {
        if *ctx.pending_d {
            *ctx.pending_d = false;
            message_actions::delete_selected_message(ctx);
        } else {
            *ctx.pending_d = true;
        }
        return Ok(());
    }
    // Any non-`d` key cancels the pending state.
    *ctx.pending_d = false;

    match key {
        "j" => ctx.state.open_chat_mut().select_next(),
        "k" => ctx.state.open_chat_mut().select_previous(),
        "h" | "esc" => {
            chat_open::close_tdlib_chat(ctx);
            ctx.state.set_active_pane(ActivePane::ChatList);
        }
        "i" => {
            if ctx.state.open_chat().is_open() {
                ctx.state.set_active_pane(ActivePane::MessageInput);
            }
        }
        "y" => {
            if let Some(msg) = ctx.state.open_chat().selected_message() {
                let text = msg.display_content();
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    if clipboard.set_text(text).is_ok() {
                        ctx.state.set_notification("Copied to clipboard");
                    }
                }
            }
        }
        "o" => message_actions::open_message_url(ctx)?,
        "l" => {
            if ctx.state.open_chat().is_open() {
                message_actions::open_selected_message(ctx);
            }
        }
        "v" => {
            if ctx.state.open_chat().is_open() {
                super::voice::start_voice_recording(ctx);
            }
        }
        "I" => {
            show_message_info_popup(ctx);
        }
        "D" => {
            download_selected_message_file(ctx);
        }
        "S" => {
            message_actions::save_selected_message_file(ctx);
        }
        "r" => {
            message_actions::reply_to_selected_message(ctx);
        }
        "e" => {
            message_actions::edit_selected_message(ctx);
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

/// Triggers a manual download of the selected message's file.
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
