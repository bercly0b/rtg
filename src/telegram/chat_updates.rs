use std::sync::mpsc::Sender;

use grammers_client::{types::Update, Client};
use tokio::{runtime::Runtime, sync::watch};

const CHAT_UPDATES_MONITOR_SHUTDOWN_FAILED: &str = "TELEGRAM_CHAT_UPDATES_MONITOR_SHUTDOWN_FAILED";
const CHAT_UPDATES_MONITOR_STARTED: &str = "TELEGRAM_CHAT_UPDATES_MONITOR_STARTED";
const CHAT_UPDATES_MONITOR_STOPPED: &str = "TELEGRAM_CHAT_UPDATES_MONITOR_STOPPED";
const CHAT_UPDATES_MONITOR_SIGNAL_SEND_FAILED: &str =
    "TELEGRAM_CHAT_UPDATES_MONITOR_SIGNAL_SEND_FAILED";
const CHAT_UPDATES_MONITOR_UPDATE_READ_FAILED: &str =
    "TELEGRAM_CHAT_UPDATES_MONITOR_UPDATE_READ_FAILED";

#[derive(Debug)]
pub struct TelegramChatUpdatesMonitor {
    stop_tx: Option<watch::Sender<bool>>,
}

impl TelegramChatUpdatesMonitor {
    pub fn start(
        runtime: &Runtime,
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

        let (stop_tx, stop_rx) = watch::channel(false);
        runtime.spawn(run_monitor(client, update_tx, stop_rx));

        tracing::info!(
            code = CHAT_UPDATES_MONITOR_STARTED,
            "telegram chat updates monitor started"
        );

        Ok(Self {
            stop_tx: Some(stop_tx),
        })
    }

    #[cfg(test)]
    pub fn inert() -> Self {
        Self { stop_tx: None }
    }
}

impl Drop for TelegramChatUpdatesMonitor {
    fn drop(&mut self) {
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(true);
            tracing::info!(
                code = CHAT_UPDATES_MONITOR_SHUTDOWN_FAILED,
                "telegram chat updates monitor shutdown signal sent"
            );
        }
    }
}

async fn run_monitor(client: Client, update_tx: Sender<()>, mut stop_rx: watch::Receiver<bool>) {
    loop {
        tokio::select! {
            changed = stop_rx.changed() => {
                if changed.is_err() || *stop_rx.borrow() {
                    tracing::info!(
                        code = CHAT_UPDATES_MONITOR_STOPPED,
                        "telegram chat updates monitor stopped"
                    );
                    return;
                }
            }
            update_result = client.next_update() => {
                match update_result {
                    Ok(update) => {
                        let kind = update_kind(&update);
                        tracing::debug!(
                            update_kind = kind,
                            "telegram update observed by chat monitor"
                        );

                        if let Err(error) = update_tx.send(()) {
                            tracing::warn!(
                                code = CHAT_UPDATES_MONITOR_SIGNAL_SEND_FAILED,
                                error = %error,
                                "chat updates monitor failed to send refresh signal"
                            );
                            return;
                        }

                        tracing::debug!(
                            update_kind = kind,
                            "chat updates monitor requested chat list refresh"
                        );
                    }
                    Err(error) => {
                        tracing::warn!(
                            code = CHAT_UPDATES_MONITOR_UPDATE_READ_FAILED,
                            error = %error,
                            "chat updates monitor update read failed; keeping monitor alive"
                        );
                    }
                }
            }
        }
    }
}

fn update_kind(update: &Update) -> &'static str {
    match update {
        Update::NewMessage(_) => "new_message",
        Update::MessageEdited(_) => "message_edited",
        Update::MessageDeleted(_) => "message_deleted",
        Update::CallbackQuery(_) => "callback_query",
        Update::InlineQuery(_) => "inline_query",
        Update::InlineSend(_) => "inline_send",
        Update::Raw(_) => "raw",
        _ => "unknown",
    }
}

#[derive(Debug)]
pub enum ChatUpdatesMonitorStartError {
    StartupRejected,
}

impl std::fmt::Display for ChatUpdatesMonitorStartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StartupRejected => f.write_str("startup rejected by test switch"),
        }
    }
}

impl std::error::Error for ChatUpdatesMonitorStartError {}
