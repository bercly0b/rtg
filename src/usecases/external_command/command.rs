use std::{
    process::{Command, Stdio},
    sync::{mpsc, Arc},
    thread,
};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

use anyhow::Context;

use crate::domain::{command_template, events::CommandEvent};

use super::{
    handle::{CommandHandle, SharedChild},
    output::stream_output,
};

/// Upper bound for the inherited descriptors the child marks close-on-exec.
///
/// `RLIMIT_NOFILE` can be huge or even unlimited, and every candidate costs a
/// syscall between fork and exec (~18 ms for 65 536 on macOS). Descriptors
/// are allocated lowest-first, so RTG never holds one this high in practice.
#[cfg(unix)]
const MAX_INHERITED_FD: libc::rlim_t = 65_536;

/// Starts an external command and returns a handle and the event channel receiver.
///
/// The command template is split with shell-like quoting rules and
/// `{file_path}` is replaced with the actual path. stdout and stderr are
/// merged and streamed line-by-line through the returned channel.
///
/// Once the output is drained and the process has exited, a
/// `CommandEvent::Exited` with its real exit status is sent through the
/// channel. A command that cannot be started (e.g. a missing binary) is
/// reported as an error instead.
///
/// Used for both voice recording and media playback.
pub fn start_command(
    cmd_template: &str,
    file_path: &str,
) -> anyhow::Result<(CommandHandle, mpsc::Receiver<CommandEvent>)> {
    let resolved = command_template::resolve_argv(cmd_template, file_path)?;

    let mut cmd = Command::new(&resolved[0]);
    cmd.args(&resolved[1..]).stdin(Stdio::null());

    #[cfg(unix)]
    let output_reader = crate::usecases::pty::attach_output_pty(&mut cmd)?;

    #[cfg(not(unix))]
    let output_reader = {
        let (reader, writer) = std::io::pipe()?;
        cmd.stdout(writer.try_clone()?).stderr(writer);
        reader
    };

    // Spawn in a new session so `stop()` can kill the entire process group,
    // including any children the command may fork (e.g. shell scripts
    // that run `rec` or `ffmpeg` in the background).
    //
    // Also keep all inherited file descriptors > 2 (including the PTY
    // master, which is opened without close-on-exec) away from the command
    // to prevent child processes from writing to the parent's terminal. On
    // macOS, crossterm holds an open fd to /dev/tty for keyboard input;
    // without closing it, tools like SoX `rec` can write progress output
    // directly to the terminal, corrupting the TUI rendering.
    #[cfg(unix)]
    {
        let fd_limit = inherited_fd_limit();
        unsafe {
            cmd.pre_exec(move || {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                // FDs 0, 1, 2 are already set up by Command. Mark the rest
                // close-on-exec rather than closing them: std reports exec
                // failures through its own close-on-exec pipe, and closing
                // that pipe turns a missing binary into an abort in the child.
                for fd in 3..fd_limit {
                    libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC);
                }
                Ok(())
            });
        }
    }

    let child = cmd
        .spawn()
        .with_context(|| format!("failed to start `{}`", resolved[0]))?;
    let child = Arc::new(SharedChild::new(child));

    let (tx, rx) = mpsc::channel::<CommandEvent>();
    let waited_child = Arc::clone(&child);
    let spawned = thread::Builder::new()
        .name("rtg-cmd".into())
        .spawn(move || {
            stream_output(output_reader, &tx);
            // Waiting only once the output is drained keeps every line ahead
            // of `Exited`. The status comes from the process itself, so a
            // command that closes its output early is still waited for.
            let success = waited_child.wait();
            let _ = tx.send(CommandEvent::Exited { success });
        });

    if let Err(err) = spawned {
        child.kill();
        child.wait();
        return Err(err.into());
    }

    Ok((CommandHandle::new(child), rx))
}

/// Exclusive upper bound for the descriptors to mark close-on-exec in the child.
#[cfg(unix)]
fn inherited_fd_limit() -> libc::c_int {
    let mut limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    // SAFETY: `limit` is a valid, writable `rlimit`.
    let soft_limit = if unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut limit) } == 0 {
        limit.rlim_cur
    } else {
        MAX_INHERITED_FD
    };
    soft_limit.min(MAX_INHERITED_FD) as libc::c_int
}
