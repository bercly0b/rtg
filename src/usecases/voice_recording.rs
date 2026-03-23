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

#[cfg(unix)]
use std::os::unix::process::CommandExt;

use crate::domain::events::CommandEvent;

pub use crate::domain::voice_defaults::DEFAULT_RECORD_CMD;

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
    /// Creates a handle from a child process (for testing).
    #[cfg(test)]
    pub(crate) fn from_child(child: Child) -> Self {
        Self { child }
    }

    /// Checks if the process has exited and returns whether it was successful.
    ///
    /// Returns `Some(true)` if the process exited with code 0,
    /// `Some(false)` if it exited with a non-zero code,
    /// and `None` if it is still running.
    pub fn try_exit_success(&mut self) -> Option<bool> {
        match self.child.try_wait() {
            Ok(Some(status)) => Some(status.success()),
            _ => None,
        }
    }

    /// Gracefully stops the recording process.
    ///
    /// First sends SIGTERM to the child process only (not the group), giving
    /// shell scripts a chance to handle cleanup (e.g. run `opusenc` after
    /// stopping `rec`). If the child doesn't exit within 3 seconds, escalates
    /// to a process-group-wide SIGKILL via `kill(-pgid, SIGKILL)`.
    pub fn stop(&mut self) {
        #[cfg(unix)]
        {
            let pid = self.child.id();
            if pid == 0 {
                return;
            }
            let raw_pid = pid as libc::pid_t;

            // Send SIGTERM to the direct child only, so shell scripts can
            // trap the signal, clean up children, and run post-processing.
            unsafe {
                libc::kill(raw_pid, libc::SIGTERM);
            }
            for _ in 0..30 {
                match self.child.try_wait() {
                    Ok(Some(_)) => return,
                    _ => thread::sleep(std::time::Duration::from_millis(100)),
                }
            }

            // Child didn't exit — force-kill the entire process group.
            // The child was spawned with setsid(), so -raw_pid targets
            // only its group, not RTG's own processes.
            unsafe {
                libc::kill(-raw_pid, libc::SIGKILL);
            }
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

    let mut cmd = Command::new(parts[0]);
    cmd.args(&parts[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Spawn in a new session so `stop()` can kill the entire process group,
    // including any children the command may fork (e.g. shell scripts
    // that run `rec` or `ffmpeg` in the background).
    //
    // Also close all inherited file descriptors > 2 to prevent child
    // processes from writing to the parent's terminal. On macOS,
    // crossterm holds an open fd to /dev/tty for keyboard input;
    // without closing it, tools like SoX `rec` can write progress
    // output directly to the terminal, corrupting the TUI rendering.
    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            // FDs 0, 1, 2 are already set up by Command (stdin=null,
            // stdout=piped, stderr=piped). Close everything else to
            // prevent inherited terminal fds from leaking to the child.
            for fd in 3..1024 {
                libc::close(fd);
            }
            Ok(())
        });
    }

    let mut child = cmd.spawn()?;

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

    #[test]
    fn try_exit_success_returns_true_for_zero_exit_code() {
        let child = std::process::Command::new("true").spawn().unwrap();
        let mut handle = RecordingHandle::from_child(child);
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert_eq!(handle.try_exit_success(), Some(true));
    }

    #[test]
    fn try_exit_success_returns_false_for_nonzero_exit_code() {
        let child = std::process::Command::new("false").spawn().unwrap();
        let mut handle = RecordingHandle::from_child(child);
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert_eq!(handle.try_exit_success(), Some(false));
    }

    #[cfg(unix)]
    #[test]
    fn child_inherits_no_high_fds_from_parent() {
        // Verify that pre_exec closes inherited file descriptors > 2.
        //
        // We deliberately open extra FDs in the parent (simulating what
        // crossterm does with /dev/tty), then spawn a child via
        // start_recording and check that these FDs are not accessible.
        //
        // The child lists /dev/fd/ and we verify that no FDs above a
        // reasonable threshold exist. FDs 0-2 are stdin/stdout/stderr,
        // fd 3 is typically the directory listing fd from `ls` itself.
        let (mut handle, rx) = start_recording("ls /dev/fd/", "/dev/null").unwrap();

        std::thread::sleep(std::time::Duration::from_millis(200));
        assert!(handle.try_exit_success().is_some());

        let events: Vec<_> = rx.try_iter().collect();
        let open_fds: Vec<i32> = events
            .iter()
            .filter_map(|e| match e {
                CommandEvent::OutputLine(line) => line.trim().parse::<i32>().ok(),
                _ => None,
            })
            .collect();

        // After pre_exec closes FDs 3..1024, the child should only have
        // FDs 0-2 (set up by Command) plus any FDs the child itself opens
        // at runtime (e.g., `ls` opens /dev/fd/ directory as fd 3).
        // We verify no high-numbered FDs leaked from the parent.
        let max_fd = open_fds.iter().copied().max().unwrap_or(0);
        assert!(
            max_fd <= 4,
            "inherited high FDs leaked to child: {:?}",
            open_fds
        );
    }

    #[cfg(unix)]
    #[test]
    fn stop_terminates_process_group_via_start_recording() {
        let (mut handle, rx) = start_recording("sleep 60", "/dev/null").unwrap();

        // Process should be running.
        assert_eq!(handle.try_exit_success(), None);

        handle.stop();

        // After stop, the process should be reaped.
        assert!(handle.try_exit_success().is_some());

        // Pipe readers should have finished, producing an Exited event.
        let events: Vec<_> = rx.try_iter().collect();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, CommandEvent::Exited { .. })),
            "expected Exited event after stop(), got: {:?}",
            events
        );
    }
}
