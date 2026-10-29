use std::{
    process::Child,
    sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError},
    time::Duration,
};

/// How long `stop()` lets the command react to SIGTERM before killing its
/// whole process group.
#[cfg(unix)]
const STOP_GRACE_PERIOD: Duration = Duration::from_secs(3);

/// A child process shared between its [`RecordingHandle`] and the thread
/// that waits for it to exit.
///
/// Only [`SharedChild::wait`] reaps the child, and only while holding the
/// lock. Signals are sent under the same lock after checking that the child
/// is not reaped yet, so they can never reach a recycled PID.
pub(super) struct SharedChild {
    #[cfg(unix)]
    pid: u32,
    state: Mutex<ChildState>,
    reaped: Condvar,
}

struct ChildState {
    child: Child,
    /// `Some(success)` once the child has been reaped.
    exit: Option<bool>,
}

impl SharedChild {
    pub(super) fn new(child: Child) -> Self {
        Self {
            #[cfg(unix)]
            pid: child.id(),
            state: Mutex::new(ChildState { child, exit: None }),
            reaped: Condvar::new(),
        }
    }

    /// Blocks until the child exits, reaps it and returns whether it exited
    /// successfully.
    pub(super) fn wait(&self) -> bool {
        self.wait_until_exited();
        let mut state = self.lock();
        let success = state.child.wait().is_ok_and(|status| status.success());
        state.exit = Some(success);
        self.reaped.notify_all();
        success
    }

    /// Force-kills the child's process group (the child itself off unix)
    /// unless it has already been reaped.
    pub(super) fn kill(&self) {
        // The child was spawned with setsid(), so -pid targets only its group.
        #[cfg(unix)]
        self.send_signal(-(self.pid as libc::pid_t), libc::SIGKILL);

        #[cfg(not(unix))]
        {
            let mut state = self.lock();
            if state.exit.is_none() {
                let _ = state.child.kill();
            }
        }
    }

    /// Sends `signal` to `target` (the child's PID or its negated process
    /// group ID). Returns `false` without signalling if the child has
    /// already been reaped.
    #[cfg(unix)]
    fn send_signal(&self, target: libc::pid_t, signal: libc::c_int) -> bool {
        let state = self.lock();
        if state.exit.is_some() {
            return false;
        }
        // SAFETY: plain syscall. The child is reaped only under this lock, so
        // its PID and the process group it leads cannot be recycled yet.
        unsafe {
            libc::kill(target, signal);
        }
        true
    }

    /// Waits up to `timeout` for the child to be reaped and returns whether
    /// it was.
    #[cfg(unix)]
    fn wait_reaped(&self, timeout: Duration) -> bool {
        let (state, _) = self
            .reaped
            .wait_timeout_while(self.lock(), timeout, |state| state.exit.is_none())
            .unwrap_or_else(PoisonError::into_inner);
        state.exit.is_some()
    }

    /// Blocks until the child has exited, leaving it unreaped so that the
    /// lock stays free for signals meanwhile.
    #[cfg(unix)]
    fn wait_until_exited(&self) {
        loop {
            // SAFETY: `info` is a valid, writable `siginfo_t`. `WNOWAIT`
            // leaves the child waitable, so `wait` reaps it under the lock.
            let rc = unsafe {
                let mut info: libc::siginfo_t = std::mem::zeroed();
                libc::waitid(
                    libc::P_PID,
                    self.pid as libc::id_t,
                    &mut info,
                    libc::WEXITED | libc::WNOWAIT,
                )
            };
            if rc == 0 || std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted
            {
                return;
            }
        }
    }

    /// Blocks until the child has exited. std offers no non-reaping wait
    /// here, so poll without holding the lock in between.
    #[cfg(not(unix))]
    fn wait_until_exited(&self) {
        while matches!(self.lock().child.try_wait(), Ok(None)) {
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn lock(&self) -> MutexGuard<'_, ChildState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Handle to a running recording process.
///
/// Holds the child process so it can be killed on user request.
pub struct RecordingHandle {
    child: Arc<SharedChild>,
}

impl Drop for RecordingHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

impl RecordingHandle {
    pub(super) fn new(child: Arc<SharedChild>) -> Self {
        Self { child }
    }

    /// Gracefully stops the recording process. Does nothing once it has been
    /// reaped.
    ///
    /// First sends SIGTERM to the child process only (not the group), giving
    /// shell scripts a chance to handle cleanup (e.g. run `opusenc` after
    /// stopping `rec`). If the child isn't reaped within 3 seconds, escalates
    /// to a process-group-wide SIGKILL via `kill(-pgid, SIGKILL)`.
    ///
    /// The child is reaped only after its output is drained, so the
    /// escalation also hits children that outlive it holding the PTY. It does
    /// not wait for the reap: a process outside the group could keep the
    /// output open indefinitely.
    pub fn stop(&mut self) {
        #[cfg(unix)]
        {
            let terminated = self
                .child
                .send_signal(self.child.pid as libc::pid_t, libc::SIGTERM);
            if !terminated || self.child.wait_reaped(STOP_GRACE_PERIOD) {
                return;
            }
        }

        self.child.kill();
    }
}
