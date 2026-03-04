//! Chat updates monitor for TDLib.
//!
//! Receives typed TDLib updates and converts them to simple refresh signals
//! for the UI layer.

use std::sync::mpsc::{Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use super::tdlib_updates::TdLibUpdate;

#[allow(dead_code)] // Used in tracing calls
const CHAT_UPDATES_MONITOR_STARTED: &str = "TELEGRAM_CHAT_UPDATES_MONITOR_STARTED";
#[allow(dead_code)] // Used in tracing calls
const CHAT_UPDATES_MONITOR_STOPPED: &str = "TELEGRAM_CHAT_UPDATES_MONITOR_STOPPED";
#[allow(dead_code)] // Used in tracing calls
const CHAT_UPDATES_MONITOR_SIGNAL_SEND_FAILED: &str =
    "TELEGRAM_CHAT_UPDATES_MONITOR_SIGNAL_SEND_FAILED";

/// Timeout for receiving updates from TDLib channel.
#[allow(dead_code)] // Will be used when wired in Phase 6.4
const UPDATE_RECV_TIMEOUT: Duration = Duration::from_millis(100);

/// Monitor that converts TDLib typed updates to simple refresh signals.
///
/// Runs a background thread that reads `TdLibUpdate` from a channel
/// and sends `()` signals to trigger UI refresh.
#[derive(Debug)]
pub struct TelegramChatUpdatesMonitor {
    /// Worker thread handle. Kept for debugging but not joined on drop.
    #[allow(dead_code)]
    worker: Option<JoinHandle<()>>,
}

impl TelegramChatUpdatesMonitor {
    /// Starts the chat updates monitor with a TDLib update receiver.
    ///
    /// # Arguments
    /// - `update_rx`: Receiver for typed TDLib updates from `TdLibClient::take_update_receiver()`
    /// - `signal_tx`: Sender for simple refresh signals consumed by the UI layer
    #[allow(dead_code)] // Will be used when wired in Phase 6.4
    pub fn start(
        update_rx: Receiver<TdLibUpdate>,
        signal_tx: Sender<()>,
    ) -> Result<Self, ChatUpdatesMonitorStartError> {
        // Test switch for failure injection
        if std::env::var("RTG_TELEGRAM_CHAT_UPDATES_MONITOR_FAIL")
            .ok()
            .as_deref()
            == Some("1")
        {
            return Err(ChatUpdatesMonitorStartError::StartupRejected);
        }

        let worker = thread::Builder::new()
            .name("rtg-chat-updates".into())
            .spawn(move || {
                run_update_monitor(update_rx, signal_tx);
            })
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to spawn chat updates monitor thread");
                ChatUpdatesMonitorStartError::StartupRejected
            })?;

        tracing::info!(
            code = CHAT_UPDATES_MONITOR_STARTED,
            "telegram chat updates monitor started"
        );

        Ok(Self {
            worker: Some(worker),
        })
    }

    /// Creates an inert monitor for testing (no background thread).
    #[cfg(test)]
    pub fn inert() -> Self {
        Self { worker: None }
    }
}

impl Drop for TelegramChatUpdatesMonitor {
    fn drop(&mut self) {
        // The worker will exit when update_rx is closed (sender dropped)
        // We don't need explicit shutdown signal since TdLibClient closing
        // will close the channel.
        // We don't join here to avoid blocking - the thread will exit on its own.
        tracing::debug!("TelegramChatUpdatesMonitor dropped");
    }
}

/// Background loop that processes TDLib updates and sends refresh signals.
#[allow(dead_code)] // Will be used when wired in Phase 6.4
fn run_update_monitor(update_rx: Receiver<TdLibUpdate>, signal_tx: Sender<()>) {
    loop {
        match update_rx.recv_timeout(UPDATE_RECV_TIMEOUT) {
            Ok(update) => {
                let kind = update.kind();
                tracing::debug!(
                    update_kind = kind,
                    "telegram update observed by chat monitor"
                );

                // Send refresh signal for any update
                if signal_tx.send(()).is_err() {
                    tracing::warn!(
                        code = CHAT_UPDATES_MONITOR_SIGNAL_SEND_FAILED,
                        "chat updates monitor failed to send refresh signal; stopping"
                    );
                    break;
                }

                tracing::debug!(
                    update_kind = kind,
                    "chat updates monitor requested chat list refresh"
                );
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // No update available, continue polling
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                // Channel closed (TdLibClient shutdown)
                tracing::info!(
                    code = CHAT_UPDATES_MONITOR_STOPPED,
                    "telegram chat updates monitor stopped (channel closed)"
                );
                break;
            }
        }
    }
}

/// Error type for chat updates monitor startup.
#[derive(Debug)]
pub enum ChatUpdatesMonitorStartError {
    /// Monitor startup was rejected (test switch or spawn failure).
    StartupRejected,
}

impl std::fmt::Display for ChatUpdatesMonitorStartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StartupRejected => f.write_str("startup rejected"),
        }
    }
}

impl std::error::Error for ChatUpdatesMonitorStartError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn monitor_sends_signal_on_update() {
        let (update_tx, update_rx) = mpsc::channel();
        let (signal_tx, signal_rx) = mpsc::channel();

        // Start monitor in separate thread
        let monitor =
            TelegramChatUpdatesMonitor::start(update_rx, signal_tx).expect("monitor should start");

        // Send an update
        update_tx
            .send(TdLibUpdate::NewMessage { chat_id: 123 })
            .expect("update should be sent");

        // Verify signal received
        let result = signal_rx.recv_timeout(Duration::from_millis(500));
        assert!(result.is_ok(), "should receive refresh signal");

        // Close the channel so monitor can exit
        drop(update_tx);
        drop(monitor);
    }

    #[test]
    fn monitor_stops_when_channel_closed() {
        let (update_tx, update_rx) = mpsc::channel::<TdLibUpdate>();
        let (signal_tx, _signal_rx) = mpsc::channel();

        let monitor =
            TelegramChatUpdatesMonitor::start(update_rx, signal_tx).expect("monitor should start");

        // Close the channel by dropping sender
        drop(update_tx);

        // Monitor should exit gracefully on drop
        drop(monitor);
    }

    #[test]
    fn inert_monitor_has_no_worker() {
        let monitor = TelegramChatUpdatesMonitor::inert();
        assert!(monitor.worker.is_none());
    }
}
