use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
    time::Duration,
};

use grammers_client::{types::Update, Client};
use tokio::runtime::Builder;

const CHAT_UPDATES_MONITOR_SHUTDOWN_FAILED: &str = "TELEGRAM_CHAT_UPDATES_MONITOR_SHUTDOWN_FAILED";
const CHAT_UPDATES_MONITOR_RUNTIME_INIT_FAILED: &str =
    "TELEGRAM_CHAT_UPDATES_MONITOR_RUNTIME_INIT_FAILED";
const CHAT_UPDATES_MONITOR_UPDATE_READ_FAILED: &str =
    "TELEGRAM_CHAT_UPDATES_MONITOR_UPDATE_READ_FAILED";
const UPDATE_POLL_TIMEOUT: Duration = Duration::from_millis(300);

#[derive(Debug)]
pub struct TelegramChatUpdatesMonitor {
    stop_tx: Option<Sender<()>>,
    worker: Option<JoinHandle<()>>,
}

impl TelegramChatUpdatesMonitor {
    pub fn start(
        client: Client,
        update_tx: Sender<()>,
    ) -> Result<Self, ChatUpdatesMonitorStartError> {
        if std::env::var("RTG_TELEGRAM_CHAT_UPDATES_MONITOR_FAIL")
            .ok()
            .as_deref()
            == Some("1")
        {
            return Err(ChatUpdatesMonitorStartError::StartupRejected);
        }

        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let worker = thread::Builder::new()
            .name("rtg-telegram-chat-updates".to_owned())
            .spawn(move || run_monitor(client, update_tx, stop_rx))
            .map_err(ChatUpdatesMonitorStartError::WorkerSpawn)?;

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

impl Drop for TelegramChatUpdatesMonitor {
    fn drop(&mut self) {
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(());
        }

        if let Some(worker) = self.worker.take() {
            if let Err(error) = worker.join() {
                tracing::warn!(
                    code = CHAT_UPDATES_MONITOR_SHUTDOWN_FAILED,
                    error = ?error,
                    "telegram chat updates monitor worker panicked on shutdown"
                );
            }
        }
    }
}

fn run_monitor(client: Client, update_tx: Sender<()>, stop_rx: Receiver<()>) {
    let runtime = match Builder::new_current_thread()
        .enable_time()
        .enable_io()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            tracing::warn!(
                code = CHAT_UPDATES_MONITOR_RUNTIME_INIT_FAILED,
                error = %error,
                "chat updates monitor runtime init failed"
            );
            return;
        }
    };

    loop {
        if stop_rx.try_recv().is_ok() {
            return;
        }

        let update_result = runtime.block_on(async {
            tokio::time::timeout(UPDATE_POLL_TIMEOUT, client.next_update()).await
        });

        match update_result {
            Ok(Ok(update)) => {
                if is_chat_list_relevant_update(&update) {
                    let _ = update_tx.send(());
                }
            }
            Ok(Err(error)) => {
                tracing::warn!(
                    code = CHAT_UPDATES_MONITOR_UPDATE_READ_FAILED,
                    error = %error,
                    "chat updates monitor stopped after update read failure"
                );
                return;
            }
            Err(_) => {}
        }
    }
}

fn is_chat_list_relevant_update(update: &Update) -> bool {
    matches!(
        update,
        Update::NewMessage(_) | Update::MessageEdited(_) | Update::MessageDeleted(_)
    )
}

#[derive(Debug)]
pub enum ChatUpdatesMonitorStartError {
    StartupRejected,
    WorkerSpawn(std::io::Error),
}

impl std::fmt::Display for ChatUpdatesMonitorStartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StartupRejected => f.write_str("startup rejected by test switch"),
            Self::WorkerSpawn(source) => write!(f, "worker spawn failed: {source}"),
        }
    }
}

impl std::error::Error for ChatUpdatesMonitorStartError {}
