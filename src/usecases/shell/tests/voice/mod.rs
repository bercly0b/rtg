mod command_lifecycle;
mod recording;
mod send_and_optimistic;

use super::*;

/// Simulates the state after `start_voice_recording` succeeds:
/// opens the command popup and sets a recording file path.
/// Does NOT spawn an external process (recording_handle is None).
///
/// Use `simulate_voice_recording_with_process` when testing exit code paths.
fn simulate_voice_recording_started(o: &mut TestOrchestrator, file_path: &str) {
    o.state.open_command_popup(
        "Recording Voice",
        crate::domain::command_popup_state::CommandPopupKind::Recording,
    );
    o.recording_file_path = Some(file_path.to_owned());
}

/// Simulates voice recording with a real process for exit-code tests.
/// `success`: if true, runs `true` (exit 0); if false, runs `false` (exit 1).
///
/// Returns the `CommandExited` event the process produced, for the test to
/// feed into the orchestrator.
fn simulate_voice_recording_with_process(
    o: &mut TestOrchestrator,
    file_path: &str,
    success: bool,
) -> AppEvent {
    let cmd = if success { "true" } else { "false" };
    let (handle, rx) = crate::usecases::voice_recording::start_command(cmd, file_path)
        .expect("failed to spawn test process");
    let exited = wait_for_command_exit(&rx);

    o.state.open_command_popup(
        "Recording Voice",
        crate::domain::command_popup_state::CommandPopupKind::Recording,
    );
    o.recording_file_path = Some(file_path.to_owned());
    o.recording_handle = Some(handle);
    exited
}

/// Drains a command's output and returns its exit as an `AppEvent`.
fn wait_for_command_exit(
    rx: &std::sync::mpsc::Receiver<crate::domain::events::CommandEvent>,
) -> AppEvent {
    use crate::domain::events::CommandEvent;

    loop {
        match rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("command must exit")
        {
            CommandEvent::OutputLine { .. } => {}
            CommandEvent::Exited { success } => return AppEvent::CommandExited { success },
        }
    }
}
