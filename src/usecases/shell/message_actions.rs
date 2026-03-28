use anyhow::Result;

use crate::usecases::background::TaskDispatcher;

use super::OrchestratorCtx;

/// Opens the currently selected message using the configured handler.
///
/// Resolves the command via MIME matching and chooses the open strategy:
/// - **Custom handler configured** (exact or wildcard MIME match) --
///   playback popup with live output (auto-closes on exit).
/// - **No custom handler** (platform default `open` / `xdg-open`) --
///   launches the external app in the background without a popup.
///   If the OS has no app for the file type, a notification is shown.
///
/// Silently ignores unsupported media types and messages without files.
pub(super) fn open_selected_message<D: TaskDispatcher>(ctx: &mut OrchestratorCtx<'_, D>) {
    use crate::domain::command_popup_state::CommandPopupKind;
    use crate::domain::message::MessageMedia;
    use crate::domain::open_defaults::is_platform_default;

    if ctx.state.command_popup().is_some() {
        return;
    }

    let msg = match ctx.state.open_chat().selected_message() {
        Some(m) => m,
        None => return,
    };

    if matches!(
        msg.media,
        MessageMedia::None
            | MessageMedia::Sticker
            | MessageMedia::Contact
            | MessageMedia::Location
            | MessageMedia::Poll
            | MessageMedia::Other
    ) {
        return;
    }

    let file_info = match &msg.file_info {
        Some(fi) => fi,
        None => return,
    };

    let local_path = match &file_info.local_path {
        Some(p) => p.clone(),
        None => {
            ctx.state.set_notification("File not downloaded yet");
            return;
        }
    };

    let cmd_template =
        crate::domain::open_handler::resolve_open_command(&file_info.mime_type, ctx.open_handlers);

    if is_platform_default(cmd_template) {
        // No custom handler -- delegate to the OS default opener.
        // Runs in background; shows notification on failure.
        ctx.dispatcher
            .dispatch_open_file(cmd_template.to_owned(), local_path);
    } else {
        // Custom handler -- run with a playback popup showing live output.
        match crate::usecases::voice_recording::start_command(cmd_template, &local_path) {
            Ok((handle, rx)) => {
                *ctx.recording_handle = Some(handle);
                *ctx.recording_file_path = None;
                *ctx.pending_command_rx = Some(rx);
                ctx.state
                    .open_command_popup("Playing", CommandPopupKind::Playback);
                tracing::info!(cmd_template, "command started");
            }
            Err(err) => {
                tracing::error!(%err, "failed to start command");
            }
        }
    }
}

pub(super) fn delete_selected_message<D: TaskDispatcher>(ctx: &mut OrchestratorCtx<'_, D>) {
    let Some(chat_id) = ctx.state.open_chat().chat_id() else {
        return;
    };
    let Some(msg) = ctx.state.open_chat().selected_message() else {
        return;
    };
    let message_id = msg.id;
    if message_id == 0 {
        return; // Pending messages have id=0, skip
    }

    // Optimistically remove from UI
    ctx.state.open_chat_mut().remove_message(message_id);
    ctx.state.set_notification("Message deleted");
    // Dispatch background deletion (fire-and-forget)
    ctx.dispatcher.dispatch_delete_message(chat_id, message_id);
}

pub(super) fn reply_to_selected_message<D: TaskDispatcher>(ctx: &mut OrchestratorCtx<'_, D>) {
    use crate::domain::{message_input_state::ReplyContext, shell_state::ActivePane};

    if !ctx.state.open_chat().is_open() {
        return;
    }

    let Some(msg) = ctx.state.open_chat().selected_message() else {
        return;
    };

    // Don't reply to pending (unsent) messages
    if msg.id == 0 {
        return;
    }

    let reply_context = ReplyContext {
        message_id: msg.id,
        sender_name: if msg.is_outgoing {
            "You".to_owned()
        } else {
            msg.sender_name.clone()
        },
        text: msg.display_content(),
    };

    ctx.state.message_input_mut().set_reply_to(reply_context);
    ctx.state.set_active_pane(ActivePane::MessageInput);
}

pub(super) fn open_message_url<D: TaskDispatcher>(ctx: &mut OrchestratorCtx<'_, D>) -> Result<()> {
    use crate::domain::message::extract_first_url;

    let Some(msg) = ctx.state.open_chat().selected_message() else {
        return Ok(());
    };
    let text = msg.display_content();
    if let Some(url) = extract_first_url(&text, &msg.links) {
        ctx.opener.open(&url)?;
    }
    Ok(())
}
