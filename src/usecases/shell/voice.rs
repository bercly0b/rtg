use crate::usecases::background::TaskDispatcher;

use super::OrchestratorCtx;

pub(super) fn start_voice_recording<D: TaskDispatcher>(ctx: &mut OrchestratorCtx<'_, D>) {
    use crate::usecases::voice_recording;

    if ctx.state.command_popup().is_some() {
        return; // already recording or popup active
    }

    let file_path = voice_recording::generate_voice_file_path();

    match voice_recording::start_command(ctx.voice_record_cmd, &file_path) {
        Ok((handle, rx)) => {
            *ctx.recording_handle = Some(handle);
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
    if let Some(mut handle) = ctx.recording_handle.take() {
        if std::thread::Builder::new()
            .name("rtg-rec-stop".into())
            .spawn(move || handle.stop())
            .is_err()
        {
            tracing::warn!("failed to spawn stop thread; handle dropped (Drop will stop process)");
        }
    }
}

pub(super) fn handle_command_popup_key<D: TaskDispatcher>(
    ctx: &mut OrchestratorCtx<'_, D>,
    key: &str,
) {
    use crate::domain::command_popup_state::{CommandPhase, CommandPopupKind};

    let (phase, kind) = match ctx.state.command_popup() {
        Some(popup) => (popup.phase().clone(), popup.kind()),
        None => return,
    };

    match phase {
        CommandPhase::Running => {
            if key == "q" || key == "esc" {
                match kind {
                    CommandPopupKind::Recording => {
                        if key == "q" {
                            stop_voice_recording(ctx);
                            if let Some(popup) = ctx.state.command_popup_mut() {
                                popup.set_phase(CommandPhase::Stopping);
                            }
                        }
                    }
                    CommandPopupKind::Playback | CommandPopupKind::Viewer => {
                        stop_playback(ctx);
                        ctx.state.close_command_popup();
                    }
                }
            }
        }
        CommandPhase::Stopping => {
            // Ignore all keys while the process is being terminated.
        }
        CommandPhase::AwaitingConfirmation { .. } => match key {
            "y" => {
                send_voice_recording(ctx);
                ctx.state.close_command_popup();
            }
            "n" | "esc" => {
                discard_voice_recording(ctx);
                ctx.state.close_command_popup();
            }
            _ => {}
        },
        CommandPhase::Done => {
            ctx.state.close_command_popup();
        }
        CommandPhase::Failed { .. } => {
            ctx.state.close_command_popup();
        }
    }
}

pub(super) fn handle_command_exited<D: TaskDispatcher>(
    ctx: &mut OrchestratorCtx<'_, D>,
    _event_success: bool,
) {
    use crate::domain::command_popup_state::{CommandPhase, CommandPopupKind};

    let (phase, kind) = match ctx.state.command_popup() {
        Some(popup) => (popup.phase().clone(), popup.kind()),
        None => return,
    };

    match kind {
        CommandPopupKind::Playback => {
            // Playback auto-closes on process exit regardless of phase.
            *ctx.recording_handle = None;
            ctx.state.close_command_popup();
        }
        CommandPopupKind::Viewer => {
            // Viewer stays open after the process exits so the user
            // can see the rendered output. Any key will close it.
            *ctx.recording_handle = None;
            if let Some(popup) = ctx.state.command_popup_mut() {
                popup.set_phase(CommandPhase::Done);
            }
        }
        CommandPopupKind::Recording => match phase {
            CommandPhase::Running => {
                let process_succeeded = ctx
                    .recording_handle
                    .as_mut()
                    .and_then(|h| h.try_exit_success())
                    .unwrap_or(false);
                *ctx.recording_handle = None;

                if let Some(popup) = ctx.state.command_popup_mut() {
                    if process_succeeded {
                        popup.set_phase(CommandPhase::AwaitingConfirmation {
                            prompt: "Command finished. Send recording? (y/n)".into(),
                        });
                    } else {
                        popup.set_phase(CommandPhase::Failed {
                            message:
                                "Recording failed. Override recording command via [voice] record_cmd in ~/.config/rtg/config.toml (press any key)"
                                    .into(),
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
                            message:
                                "Recording failed. Override recording command via [voice] record_cmd in ~/.config/rtg/config.toml (press any key)"
                                    .into(),
                        });
                        discard_voice_recording(ctx);
                    }
                }
            }
            _ => {
                // AwaitingConfirmation / Failed — do not override.
            }
        },
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
    ctx.dispatcher.dispatch_send_voice(chat_id, file_path);
}

/// Stops the playback process immediately. Unlike recording stop,
/// this is fire-and-forget — the popup closes right away.
pub(super) fn stop_playback<D: TaskDispatcher>(ctx: &mut OrchestratorCtx<'_, D>) {
    if let Some(mut handle) = ctx.recording_handle.take() {
        if std::thread::Builder::new()
            .name("rtg-play-stop".into())
            .spawn(move || handle.stop())
            .is_err()
        {
            tracing::warn!("failed to spawn playback stop thread; handle dropped");
        }
    }
}

pub(super) fn discard_voice_recording<D: TaskDispatcher>(ctx: &mut OrchestratorCtx<'_, D>) {
    if let Some(file_path) = ctx.recording_file_path.take() {
        let _ = std::fs::remove_file(&file_path);
        tracing::info!(file_path, "voice recording discarded");
    }
}
