use crate::usecases::background::TaskDispatcher;

use super::{voice, OrchestratorCtx};

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
                            voice::stop_voice_recording(ctx);
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
                voice::send_voice_recording(ctx);
                ctx.state.close_command_popup();
            }
            "n" | "esc" => {
                voice::discard_voice_recording(ctx);
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
            *ctx.command_handle = None;
            ctx.state.close_command_popup();
        }
        CommandPopupKind::Viewer => {
            // Viewer stays open after the process exits so the user
            // can see the rendered output. Any key will close it.
            *ctx.command_handle = None;
            if let Some(popup) = ctx.state.command_popup_mut() {
                popup.set_phase(CommandPhase::Done);
            }
        }
        CommandPopupKind::Recording => voice::handle_recording_exited(ctx, phase),
    }
}

/// Stops the playback process immediately. Unlike recording stop,
/// this is fire-and-forget — the popup closes right away.
fn stop_playback<D: TaskDispatcher>(ctx: &mut OrchestratorCtx<'_, D>) {
    if let Some(mut handle) = ctx.command_handle.take() {
        if std::thread::Builder::new()
            .name("rtg-play-stop".into())
            .spawn(move || handle.stop())
            .is_err()
        {
            tracing::warn!("failed to spawn playback stop thread; handle dropped");
        }
    }
}
