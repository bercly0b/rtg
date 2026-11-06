use std::collections::HashMap;

use crate::usecases::{background::TaskDispatcher, external_command::CommandHandle};

use super::{voice, ShellCtx};

/// The external command shown in the command popup (voice recording, media
/// playback) and the config it is started from.
pub(super) struct MediaCommands {
    /// Handle to the running external command.
    pub command_handle: Option<CommandHandle>,
    /// Path to the currently recorded voice file.
    pub recording_file_path: Option<String>,
    /// Voice recording command template (from config).
    pub voice_record_cmd: String,
    /// MIME-type → command mappings for opening message files (from config).
    pub open_handlers: HashMap<String, String>,
}

impl MediaCommands {
    pub(super) fn new(voice_record_cmd: String, open_handlers: HashMap<String, String>) -> Self {
        Self {
            command_handle: None,
            recording_file_path: None,
            voice_record_cmd,
            open_handlers,
        }
    }
}

pub(super) fn handle_command_popup_key<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    media: &mut MediaCommands,
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
                            voice::stop_voice_recording(media);
                            if let Some(popup) = ctx.state.command_popup_mut() {
                                popup.set_phase(CommandPhase::Stopping);
                            }
                        }
                    }
                    CommandPopupKind::Playback | CommandPopupKind::Viewer => {
                        stop_playback(media);
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
                voice::send_voice_recording(ctx, media);
                ctx.state.close_command_popup();
            }
            "n" | "esc" => {
                voice::discard_voice_recording(media);
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
    ctx: &mut ShellCtx<'_, D>,
    media: &mut MediaCommands,
    success: bool,
) {
    use crate::domain::command_popup_state::{CommandPhase, CommandPopupKind};

    let (phase, kind) = match ctx.state.command_popup() {
        Some(popup) => (popup.phase().clone(), popup.kind()),
        None => return,
    };

    match kind {
        CommandPopupKind::Playback => {
            // Playback auto-closes on process exit regardless of phase.
            media.command_handle = None;
            ctx.state.close_command_popup();
        }
        CommandPopupKind::Viewer => {
            // Viewer stays open after the process exits so the user
            // can see the rendered output. Any key will close it.
            media.command_handle = None;
            if let Some(popup) = ctx.state.command_popup_mut() {
                popup.set_phase(CommandPhase::Done);
            }
        }
        CommandPopupKind::Recording => voice::handle_recording_exited(ctx, media, phase, success),
    }
}

/// Stops the playback process immediately. Unlike recording stop,
/// this is fire-and-forget — the popup closes right away.
fn stop_playback(media: &mut MediaCommands) {
    if let Some(mut handle) = media.command_handle.take() {
        if std::thread::Builder::new()
            .name("rtg-play-stop".into())
            .spawn(move || handle.stop())
            .is_err()
        {
            tracing::warn!("failed to spawn playback stop thread; handle dropped");
        }
    }
}
