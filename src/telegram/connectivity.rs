use std::{
    sync::mpsc::{self, Receiver, RecvTimeoutError, Sender},
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::domain::events::ConnectivityStatus;

const CONNECTIVITY_MONITOR_SHUTDOWN_FAILED: &str = "TELEGRAM_CONNECTIVITY_MONITOR_SHUTDOWN_FAILED";

/// Polling interval for shutdown signal while waiting on TDLib status.
/// The worker prefers to block on the connectivity receiver, but wakes up
/// periodically to honour the stop signal even when TDLib is silent.
const STOP_POLL_INTERVAL: Duration = Duration::from_millis(200);

#[derive(Debug)]
pub struct TelegramConnectivityMonitor {
    stop_tx: Option<Sender<()>>,
    worker: Option<JoinHandle<()>>,
}

impl TelegramConnectivityMonitor {
    pub fn start<F>(
        connectivity_rx: Receiver<ConnectivityStatus>,
        status_tx: Sender<ConnectivityStatus>,
        on_status: F,
    ) -> Result<Self, ConnectivityMonitorStartError>
    where
        F: Fn(ConnectivityStatus) + Send + 'static,
    {
        if std::env::var("RTG_TELEGRAM_CONNECTIVITY_MONITOR_FAIL")
            .ok()
            .as_deref()
            == Some("1")
        {
            return Err(ConnectivityMonitorStartError::StartupRejected);
        }

        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let worker = thread::Builder::new()
            .name("rtg-telegram-connectivity".to_owned())
            .spawn(move || run_monitor(connectivity_rx, status_tx, stop_rx, on_status))
            .map_err(ConnectivityMonitorStartError::WorkerSpawn)?;

        Ok(Self {
            stop_tx: Some(stop_tx),
            worker: Some(worker),
        })
    }

    #[cfg(test)]
    pub fn inert() -> Self {
        Self {
            stop_tx: None,
            worker: None,
        }
    }
}

impl Drop for TelegramConnectivityMonitor {
    fn drop(&mut self) {
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(());
        }

        if let Some(worker) = self.worker.take() {
            if let Err(error) = worker.join() {
                tracing::warn!(
                    code = CONNECTIVITY_MONITOR_SHUTDOWN_FAILED,
                    error = ?error,
                    "telegram connectivity monitor worker panicked on shutdown"
                );
            }
        }
    }
}

fn run_monitor<F>(
    connectivity_rx: Receiver<ConnectivityStatus>,
    status_tx: Sender<ConnectivityStatus>,
    stop_rx: Receiver<()>,
    on_status: F,
) where
    F: Fn(ConnectivityStatus),
{
    loop {
        match connectivity_rx.recv_timeout(STOP_POLL_INTERVAL) {
            Ok(status) => {
                on_status(status);
                if status_tx.send(status).is_err() {
                    break;
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                if stop_rx.try_recv().is_ok() {
                    break;
                }
            }
            Err(RecvTimeoutError::Disconnected) => {
                break;
            }
        }
    }
}

#[derive(Debug)]
pub enum ConnectivityMonitorStartError {
    StartupRejected,
    WorkerSpawn(std::io::Error),
}

impl std::fmt::Display for ConnectivityMonitorStartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StartupRejected => f.write_str("startup rejected by test switch"),
            Self::WorkerSpawn(source) => write!(f, "worker spawn failed: {source}"),
        }
    }
}

impl std::error::Error for ConnectivityMonitorStartError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    fn drain_until<F>(
        rx: &Receiver<ConnectivityStatus>,
        timeout: Duration,
        mut stop: F,
    ) -> Vec<ConnectivityStatus>
    where
        F: FnMut(&[ConnectivityStatus]) -> bool,
    {
        let deadline = Instant::now() + timeout;
        let mut out = Vec::new();
        while Instant::now() < deadline {
            match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(status) => {
                    out.push(status);
                    if stop(&out) {
                        break;
                    }
                }
                Err(RecvTimeoutError::Timeout) => {
                    if stop(&out) {
                        break;
                    }
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
        out
    }

    #[test]
    fn forwards_connectivity_updates_to_status_channel_and_callback() {
        let (conn_tx, conn_rx) = mpsc::channel::<ConnectivityStatus>();
        let (status_tx, status_rx) = mpsc::channel::<ConnectivityStatus>();
        let observed = Arc::new(Mutex::new(Vec::new()));
        let observed_clone = Arc::clone(&observed);

        let monitor = TelegramConnectivityMonitor::start(conn_rx, status_tx, move |status| {
            observed_clone.lock().unwrap().push(status);
        })
        .expect("monitor should start");

        conn_tx.send(ConnectivityStatus::Connecting).unwrap();
        conn_tx.send(ConnectivityStatus::Updating).unwrap();
        conn_tx.send(ConnectivityStatus::Connected).unwrap();

        let forwarded = drain_until(&status_rx, Duration::from_millis(500), |seen| {
            seen.len() >= 3
        });
        assert_eq!(
            forwarded,
            vec![
                ConnectivityStatus::Connecting,
                ConnectivityStatus::Updating,
                ConnectivityStatus::Connected,
            ]
        );

        drop(monitor);
        assert_eq!(
            *observed.lock().unwrap(),
            vec![
                ConnectivityStatus::Connecting,
                ConnectivityStatus::Updating,
                ConnectivityStatus::Connected,
            ]
        );
    }

    #[test]
    fn worker_stops_when_connectivity_channel_closes() {
        let (conn_tx, conn_rx) = mpsc::channel::<ConnectivityStatus>();
        let (status_tx, _status_rx) = mpsc::channel::<ConnectivityStatus>();
        let monitor = TelegramConnectivityMonitor::start(conn_rx, status_tx, |_| {})
            .expect("monitor should start");

        drop(conn_tx);
        // Drop join blocks until worker exits — if it doesn't, the test hangs.
        drop(monitor);
    }
}
