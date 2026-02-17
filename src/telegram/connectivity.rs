use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
};

use crate::domain::events::ConnectivityStatus;

const CONNECTIVITY_MONITOR_SHUTDOWN_FAILED: &str = "TELEGRAM_CONNECTIVITY_MONITOR_SHUTDOWN_FAILED";

#[derive(Debug)]
pub struct TelegramConnectivityMonitor {
    stop_tx: Option<Sender<()>>,
    worker: Option<JoinHandle<()>>,
}

impl TelegramConnectivityMonitor {
    pub fn start(status_tx: Sender<ConnectivityStatus>) -> Result<Self, ConnectivityMonitorStartError> {
        if std::env::var("RTG_TELEGRAM_CONNECTIVITY_MONITOR_FAIL").ok().as_deref() == Some("1") {
            return Err(ConnectivityMonitorStartError::StartupRejected);
        }

        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let worker = thread::Builder::new()
            .name("rtg-telegram-connectivity".to_owned())
            .spawn(move || run_monitor(status_tx, stop_rx))
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

fn run_monitor(status_tx: Sender<ConnectivityStatus>, stop_rx: Receiver<()>) {
    let _ = status_tx.send(ConnectivityStatus::Connecting);
    let _ = status_tx.send(ConnectivityStatus::Connected);
    let _ = stop_rx.recv();
    let _ = status_tx.send(ConnectivityStatus::Disconnected);
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
