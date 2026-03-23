#[cfg(unix)]
use std::{
    fs::File,
    os::fd::{FromRawFd, RawFd},
    process::{Command, Stdio},
};

/// Attaches command stdout/stderr to a pseudo-terminal and returns a reader
/// for the PTY master stream.
#[cfg(unix)]
pub fn attach_output_pty(cmd: &mut Command) -> anyhow::Result<File> {
    let mut master_fd: RawFd = -1;
    let mut slave_fd: RawFd = -1;

    let rc = unsafe {
        // SAFETY: pointers are valid and writable for the duration of the call;
        // null pointers for name/termios/winsize are explicitly supported.
        libc::openpty(
            &mut master_fd,
            &mut slave_fd,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };

    if rc == -1 {
        return Err(std::io::Error::last_os_error().into());
    }

    // SAFETY: `slave_fd` is freshly created by `openpty` and owned here.
    let slave_file = unsafe { File::from_raw_fd(slave_fd) };
    let stderr_file = match slave_file.try_clone() {
        Ok(file) => file,
        Err(err) => {
            unsafe {
                // SAFETY: `master_fd` is owned by this function and still open.
                libc::close(master_fd);
            }
            return Err(err.into());
        }
    };

    cmd.stdout(Stdio::from(slave_file));
    cmd.stderr(Stdio::from(stderr_file));

    // SAFETY: `master_fd` is freshly created by `openpty` and owned here.
    let master_file = unsafe { File::from_raw_fd(master_fd) };
    Ok(master_file)
}
