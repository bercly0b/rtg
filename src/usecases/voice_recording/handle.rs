use std::{process::Child, thread};

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
    pub(super) fn from_parts(child: Child) -> Self {
        Self { child }
    }

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
