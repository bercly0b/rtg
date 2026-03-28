use std::{
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        mpsc, Arc,
    },
    thread,
};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

use crate::domain::events::CommandEvent;

use super::{handle::RecordingHandle, output::stream_output};

/// Starts an external command and returns a handle and the event channel receiver.
///
/// The command template is split by whitespace and `{file_path}` is replaced
/// with the actual path. stdout and stderr are merged and streamed line-by-line
/// through the returned channel.
///
/// When both pipe readers finish (process exited or was killed), a
/// `CommandEvent::Exited` is automatically sent through the channel.
///
/// Used for both voice recording and media playback.
pub fn start_command(
    cmd_template: &str,
    file_path: &str,
) -> anyhow::Result<(RecordingHandle, mpsc::Receiver<CommandEvent>)> {
    let template_parts: Vec<&str> = cmd_template.split_whitespace().collect();
    if template_parts.is_empty() {
        anyhow::bail!("empty command");
    }

    // Substitute {file_path} AFTER splitting so paths with spaces
    // remain a single argument token.
    let resolved: Vec<String> = template_parts
        .iter()
        .map(|p| p.replace("{file_path}", file_path))
        .collect();

    let mut cmd = Command::new(&resolved[0]);
    cmd.args(&resolved[1..]).stdin(Stdio::null());

    #[cfg(unix)]
    let output_reader = crate::usecases::pty::attach_output_pty(&mut cmd)?;

    #[cfg(not(unix))]
    {
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    }

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
            // FDs 0, 1, 2 are already set up by Command. Close everything else to
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

    #[cfg(unix)]
    {
        let tx_clone = tx.clone();
        let gate = Arc::clone(&reader_gate);
        match thread::Builder::new()
            .name("rtg-cmd-pty".into())
            .spawn(move || {
                stream_output(output_reader, &tx_clone);
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

    #[cfg(not(unix))]
    {
        if let Some(stderr) = child.stderr.take() {
            let tx_clone = tx.clone();
            let gate = Arc::clone(&reader_gate);
            match thread::Builder::new()
                .name("rtg-cmd-stderr".into())
                .spawn(move || {
                    stream_output(stderr, &tx_clone);
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
                    stream_output(stdout, &tx_clone);
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
    }

    reader_gate.set_expected(spawned_readers, &tx);

    Ok((RecordingHandle::from_parts(child), rx))
}

/// Coordinates pipe reader threads. When all expected readers finish,
/// sends `CommandEvent::Exited` through the channel.
pub(super) struct ReaderGate {
    finished: AtomicU8,
    expected: AtomicU8,
    exited_sent: AtomicBool,
}

impl ReaderGate {
    pub(super) fn new() -> Self {
        Self {
            finished: AtomicU8::new(0),
            expected: AtomicU8::new(0),
            exited_sent: AtomicBool::new(false),
        }
    }

    /// Sets the expected number of readers (called after all threads are spawned).
    pub(super) fn set_expected(&self, count: u8, tx: &mpsc::Sender<CommandEvent>) {
        self.expected.store(count, Ordering::Release);
        self.try_send_exited(tx);
    }

    /// Called when a pipe reader thread finishes.
    pub(super) fn on_reader_finished(&self, tx: &mpsc::Sender<CommandEvent>) {
        self.finished.fetch_add(1, Ordering::AcqRel);
        self.try_send_exited(tx);
    }

    fn try_send_exited(&self, tx: &mpsc::Sender<CommandEvent>) {
        let expected = self.expected.load(Ordering::Acquire);
        let done = self.finished.load(Ordering::Acquire);
        if expected == 0 || done < expected {
            return;
        }
        if self.exited_sent.swap(true, Ordering::AcqRel) {
            return;
        }

        let _ = tx.send(CommandEvent::Exited { success: true });
    }
}
