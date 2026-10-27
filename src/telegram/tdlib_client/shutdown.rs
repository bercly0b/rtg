use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use tdlib_rs::enums::AuthorizationState;

use super::types::{AuthStateUpdate, TdLibError};

/// Waits until TDLib reports `AuthorizationState::Closed`, then joins the
/// update loop thread, which exits right after forwarding that state.
pub(super) fn await_closed_and_join(
    auth_state_rx: &Receiver<AuthStateUpdate>,
    update_thread: Option<JoinHandle<()>>,
    timeout: Duration,
) -> Result<(), TdLibError> {
    await_closed(auth_state_rx, timeout)?;

    if let Some(update_thread) = update_thread {
        update_thread.join().map_err(|_| TdLibError::Shutdown {
            message: "TDLib update loop thread panicked".to_owned(),
        })?;
    }

    Ok(())
}

fn await_closed(
    auth_state_rx: &Receiver<AuthStateUpdate>,
    timeout: Duration,
) -> Result<(), TdLibError> {
    let deadline = Instant::now() + timeout;

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match auth_state_rx.recv_timeout(remaining) {
            Ok(AuthStateUpdate {
                state: AuthorizationState::Closed,
            }) => return Ok(()),
            Ok(update) => {
                tracing::debug!(state = ?update.state, "Waiting for TDLib to close");
            }
            // The update loop drops its sender only after forwarding `Closed`,
            // so a disconnect means another reader has already consumed it.
            Err(RecvTimeoutError::Disconnected) => return Ok(()),
            Err(RecvTimeoutError::Timeout) => {
                return Err(TdLibError::Timeout {
                    message: "waiting for TDLib to report Closed".to_owned(),
                })
            }
        }
    }
}
