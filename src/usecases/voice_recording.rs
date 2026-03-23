//! Voice recording management: spawning ffmpeg, streaming output, and process control.
//!
//! The recording process runs in a background thread. Its stdout/stderr output
//! is streamed line-by-line through an mpsc channel that the UI event source
//! polls to update the command popup in real time.

use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicU8, Ordering},
        mpsc, Arc,
    },
    thread,
};

use crate::domain::events::CommandEvent;

/// Default recording command for macOS (AVFoundation).
#[cfg(target_os = "macos")]
pub const DEFAULT_RECORD_CMD: &str =
    "ffmpeg -f avfoundation -i ':0' -c:a libopus -b:a 32k {file_path}";

/// Default recording command for Linux (ALSA).
#[cfg(target_os = "linux")]
pub const DEFAULT_RECORD_CMD: &str = "ffmpeg -f alsa -i hw:0 -c:a libopus -b:a 32k {file_path}";

/// Fallback for other platforms.
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub const DEFAULT_RECORD_CMD: &str =
    "ffmpeg -f avfoundation -i ':0' -c:a libopus -b:a 32k {file_path}";

/// Generates a unique file path for a voice recording in the temp directory.
pub fn generate_voice_file_path() -> String {
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    format!("{}/voice-{}.oga", std::env::temp_dir().display(), timestamp)
}

/// Handle to a running recording process.
///
/// Holds the child process so it can be killed on user request.
pub struct RecordingHandle {
    child: Child,
}

impl Drop for RecordingHandle {
    fn drop(&mut self) {
        if let Ok(None) = self.child.try_wait() {
            self.stop();
        }
    }
}

impl RecordingHandle {
    /// Sends SIGTERM to the recording process and waits for exit.
    ///
    /// Falls back to SIGKILL if the process does not respond within 3 seconds.
    pub fn stop(&mut self) {
        #[cfg(unix)]
        {
            // SAFETY: child.id() returns a valid PID for a process we own.
            // The cast to pid_t is safe because PIDs fit in i32 on all supported platforms.
            unsafe {
                libc::kill(self.child.id() as libc::pid_t, libc::SIGTERM);
            }
            for _ in 0..30 {
                match self.child.try_wait() {
                    Ok(Some(_)) => return,
                    _ => thread::sleep(std::time::Duration::from_millis(100)),
                }
            }
            let _ = self.child.kill();
            let _ = self.child.wait();
        }

        #[cfg(not(unix))]
        {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

/// Starts the recording process and returns a handle and the event channel receiver.
///
/// The recording command is split by whitespace and `{file_path}` is replaced
/// with the actual path. stdout and stderr are merged and streamed line-by-line
/// through the returned channel.
///
/// When both pipe readers finish (process exited or was killed), a
/// `CommandEvent::Exited` is automatically sent through the channel.
pub fn start_recording(
    cmd_template: &str,
    file_path: &str,
) -> anyhow::Result<(RecordingHandle, mpsc::Receiver<CommandEvent>)> {
    let cmd_str = cmd_template.replace("{file_path}", file_path);
    let parts: Vec<&str> = cmd_str.split_whitespace().collect();

    if parts.is_empty() {
        anyhow::bail!("empty recording command");
    }

    let mut child = Command::new(parts[0])
        .args(&parts[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let (tx, rx) = mpsc::channel::<CommandEvent>();

    // Track how many pipe readers are spawned. The last reader to finish
    // sends `CommandEvent::Exited` through the channel.
    let mut spawned_readers: u8 = 0;
    let reader_gate = Arc::new(ReaderGate::new());

    if let Some(stderr) = child.stderr.take() {
        let tx_clone = tx.clone();
        let gate = Arc::clone(&reader_gate);
        match thread::Builder::new()
            .name("rtg-cmd-stderr".into())
            .spawn(move || {
                stream_lines(stderr, &tx_clone);
                gate.on_reader_finished(&tx_clone);
            }) {
            Ok(_) => spawned_readers += 1,
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(e.into());
            }
        }
    }

    if let Some(stdout) = child.stdout.take() {
        let tx_clone = tx.clone();
        let gate = Arc::clone(&reader_gate);
        match thread::Builder::new()
            .name("rtg-cmd-stdout".into())
            .spawn(move || {
                stream_lines(stdout, &tx_clone);
                gate.on_reader_finished(&tx_clone);
            }) {
            Ok(_) => spawned_readers += 1,
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(e.into());
            }
        }
    }

    reader_gate.set_expected(spawned_readers);

    Ok((RecordingHandle { child }, rx))
}

/// Reads lines from a reader and sends them as `CommandEvent::OutputLine`.
fn stream_lines<R: std::io::Read>(reader: R, tx: &mpsc::Sender<CommandEvent>) {
    let buf = BufReader::new(reader);
    for line in buf.lines() {
        match line {
            Ok(text) => {
                if tx.send(CommandEvent::OutputLine(text)).is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}

/// Coordinates pipe reader threads. When all expected readers finish,
/// sends `CommandEvent::Exited` through the channel.
struct ReaderGate {
    finished: AtomicU8,
    expected: AtomicU8,
}

impl ReaderGate {
    fn new() -> Self {
        Self {
            finished: AtomicU8::new(0),
            expected: AtomicU8::new(0),
        }
    }

    /// Sets the expected number of readers (called after all threads are spawned).
    fn set_expected(&self, count: u8) {
        self.expected.store(count, Ordering::Release);
    }

    /// Called when a pipe reader thread finishes.
    fn on_reader_finished(&self, tx: &mpsc::Sender<CommandEvent>) {
        let done = self.finished.fetch_add(1, Ordering::AcqRel) + 1;
        let expected = self.expected.load(Ordering::Acquire);
        if expected > 0 && done >= expected {
            let _ = tx.send(CommandEvent::Exited { success: true });
        }
    }
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
mod tests {
    use super::*;

    #[test]
    fn generate_voice_file_path_has_oga_extension() {
        let path = generate_voice_file_path();
        assert!(path.ends_with(".oga"));
    }

    #[test]
    fn generate_voice_file_path_contains_voice_prefix() {
        let path = generate_voice_file_path();
        assert!(path.contains("voice-"));
    }

    #[test]
    fn default_record_cmd_contains_ffmpeg() {
        assert!(DEFAULT_RECORD_CMD.contains("ffmpeg"));
    }

    #[test]
    fn default_record_cmd_contains_file_path_placeholder() {
        assert!(DEFAULT_RECORD_CMD.contains("{file_path}"));
    }

    #[test]
    fn default_record_cmd_uses_opus_codec() {
        assert!(DEFAULT_RECORD_CMD.contains("libopus"));
    }

    #[test]
    fn reader_gate_sends_exited_when_all_done() {
        let gate = Arc::new(ReaderGate::new());
        gate.set_expected(2);
        let (tx, rx) = mpsc::channel();

        // First reader finishes — should not send Exited yet.
        gate.on_reader_finished(&tx);
        assert!(rx.try_recv().is_err());

        // Second reader finishes — should send Exited.
        gate.on_reader_finished(&tx);
        let event = rx.try_recv().unwrap();
        assert_eq!(event, CommandEvent::Exited { success: true });
    }

    #[test]
    fn reader_gate_single_reader() {
        let gate = Arc::new(ReaderGate::new());
        gate.set_expected(1);
        let (tx, rx) = mpsc::channel();

        gate.on_reader_finished(&tx);
        let event = rx.try_recv().unwrap();
        assert_eq!(event, CommandEvent::Exited { success: true });
    }

    #[test]
    fn stream_lines_sends_output_lines() {
        let input = b"line 1\nline 2\nline 3\n";
        let (tx, rx) = mpsc::channel();

        stream_lines(&input[..], &tx);
        drop(tx);

        let events: Vec<_> = rx.iter().collect();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0], CommandEvent::OutputLine("line 1".into()));
        assert_eq!(events[1], CommandEvent::OutputLine("line 2".into()));
        assert_eq!(events[2], CommandEvent::OutputLine("line 3".into()));
    }

    #[test]
    fn stream_lines_handles_empty_input() {
        let input = b"";
        let (tx, rx) = mpsc::channel();

        stream_lines(&input[..], &tx);
        drop(tx);

        let events: Vec<_> = rx.iter().collect();
        assert!(events.is_empty());
    }
}
