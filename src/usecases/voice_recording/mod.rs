//! Voice recording management: spawning ffmpeg, streaming output, and process control.
//!
//! The recording process runs in a background thread. Its output is streamed
//! through an mpsc channel that the UI event source polls to update the command
//! popup in real time.

mod command;
pub(crate) mod handle;
mod output;

use std::process::{Command, Stdio};

pub use crate::domain::voice_defaults::DEFAULT_RECORD_CMD;
pub use command::start_command;
pub use handle::RecordingHandle;

/// Generates a unique file path for a voice recording in the temp directory.
pub fn generate_voice_file_path() -> String {
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    format!("{}/voice-{}.oga", std::env::temp_dir().display(), timestamp)
}

/// Gets the duration of an audio file in seconds using ffprobe.
pub fn get_audio_duration(file_path: &str) -> Option<i32> {
    let output = Command::new("ffprobe")
        .args(["-v", "error", "-show_format", "-i", file_path])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some(val) = line.strip_prefix("duration=") {
            return val.parse::<f64>().ok().map(|d| d as i32);
        }
    }
    None
}

/// Generates a flat waveform stub for the voice note.
///
/// TDLib expects 5-bit encoded amplitude values. A flat mid-level amplitude
/// produces a clean, uniform waveform visualization in Telegram clients.
pub fn generate_waveform_stub() -> String {
    use base64::Engine;
    let bytes: Vec<u8> = vec![0x55; 100];
    base64::engine::general_purpose::STANDARD.encode(&bytes)
}

#[cfg(test)]
mod tests;
