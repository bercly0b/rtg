use crate::{domain::command_popup_state::CommandPhase, usecases::background::TaskDispatcher};

use super::OrchestratorCtx;

// Generic on purpose: the config may live outside the default location (`--config`).
const RECORDING_FAILED_MESSAGE: &str =
    "Recording failed. Override recording command via [voice] record_cmd in the config file (press any key)";

pub(super) fn start_voice_recording<D: TaskDispatcher>(ctx: &mut OrchestratorCtx<'_, D>) {
    use crate::usecases::{external_command, voice_recording};

    if ctx.state.command_popup().is_some() {
        return; // already recording or popup active
    }

    let file_path = voice_recording::generate_voice_file_path();

    match external_command::start_command(ctx.voice_record_cmd, &file_path) {
        Ok((handle, rx)) => {
            *ctx.command_handle = Some(handle);
            *ctx.recording_file_path = Some(file_path);
            *ctx.pending_command_rx = Some(rx);
            ctx.state.open_command_popup(
                "Recording Voice",
                crate::domain::command_popup_state::CommandPopupKind::Recording,
            );
            tracing::info!("voice recording started");
        }
        Err(err) => {
            tracing::error!(%err, "failed to start voice recording");
        }
    }
}

/// Stops the recording process in a background thread to avoid blocking the UI.
///
/// The handle is moved to the thread, which calls `stop()` and then drops it.
/// The pipe readers will naturally send `CommandExited` when the process dies.
/// If the thread fails to spawn, `Drop` on the handle still terminates the process.
pub(super) fn stop_voice_recording<D: TaskDispatcher>(ctx: &mut OrchestratorCtx<'_, D>) {
    if let Some(mut handle) = ctx.command_handle.take() {
        if std::thread::Builder::new()
            .name("rtg-rec-stop".into())
            .spawn(move || handle.stop())
            .is_err()
        {
            tracing::warn!("failed to spawn stop thread; handle dropped (Drop will stop process)");
        }
    }
}

pub(super) fn handle_recording_exited<D: TaskDispatcher>(
    ctx: &mut OrchestratorCtx<'_, D>,
    phase: CommandPhase,
) {
    match phase {
        CommandPhase::Running => {
            let process_succeeded = ctx
                .command_handle
                .as_mut()
                .and_then(|h| h.try_exit_success())
                .unwrap_or(false);
            *ctx.command_handle = None;

            if let Some(popup) = ctx.state.command_popup_mut() {
                if process_succeeded {
                    popup.set_phase(CommandPhase::AwaitingConfirmation {
                        prompt: "Command finished. Send recording? (y/n)".into(),
                    });
                } else {
                    popup.set_phase(CommandPhase::Failed {
                        message: RECORDING_FAILED_MESSAGE.into(),
                    });
                    discard_voice_recording(ctx);
                }
            }
        }
        CommandPhase::Stopping => {
            let file_ok = ctx
                .recording_file_path
                .as_ref()
                .map(|p| std::fs::metadata(p).map(|m| m.len() > 0).unwrap_or(false))
                .unwrap_or(false);

            if let Some(popup) = ctx.state.command_popup_mut() {
                if file_ok {
                    popup.set_phase(CommandPhase::AwaitingConfirmation {
                        prompt: "Send recording? (y/n)".into(),
                    });
                } else {
                    popup.set_phase(CommandPhase::Failed {
                        message: RECORDING_FAILED_MESSAGE.into(),
                    });
                    discard_voice_recording(ctx);
                }
            }
        }
        _ => {
            // AwaitingConfirmation / Failed — do not override.
        }
    }
}

pub(super) fn send_voice_recording<D: TaskDispatcher>(ctx: &mut OrchestratorCtx<'_, D>) {
    let Some(file_path) = ctx.recording_file_path.take() else {
        return;
    };
    let Some(chat_id) = ctx.state.open_chat().chat_id() else {
        tracing::warn!("no chat open to send voice recording");
        return;
    };

    if !std::path::Path::new(&file_path).exists() {
        tracing::warn!(file_path, "recorded file does not exist");
        return;
    }

    // Optimistically show the voice message immediately
    ctx.state.open_chat_mut().add_pending_message(
        String::new(),
        crate::domain::message::MessageMedia::Voice,
        None,
    );
    let topic_id = ctx.state.open_chat().topic_id();
    ctx.dispatcher
        .dispatch_send_voice(chat_id, topic_id, file_path);
}

pub(super) fn discard_voice_recording<D: TaskDispatcher>(ctx: &mut OrchestratorCtx<'_, D>) {
    if let Some(file_path) = ctx.recording_file_path.take() {
        let _ = std::fs::remove_file(&file_path);
        tracing::info!(file_path, "voice recording discarded");
    }
}
