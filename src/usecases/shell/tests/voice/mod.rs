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
/// `success`: if true, spawns `true` (exit 0); if false, spawns `false` (exit 1).
fn simulate_voice_recording_with_process(o: &mut TestOrchestrator, file_path: &str, success: bool) {
    use std::process::Command;

    let cmd = if success { "true" } else { "false" };
    let child = Command::new(cmd)
        .spawn()
        .expect("failed to spawn test process");
    let mut handle = crate::usecases::voice_recording::RecordingHandle::from_child(child);
    std::thread::sleep(std::time::Duration::from_millis(50));
    let _ = handle.try_exit_success();

    o.state.open_command_popup(
        "Recording Voice",
        crate::domain::command_popup_state::CommandPopupKind::Recording,
    );
    o.recording_file_path = Some(file_path.to_owned());
    o.recording_handle = Some(handle);
}
