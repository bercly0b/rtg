use std::sync::mpsc::Receiver;

#[cfg(test)]
use std::sync::mpsc;

use crate::domain::events::{BackgroundTaskResult, ChatUpdate, CommandEvent, ConnectivityStatus};

use super::{
    BackgroundResultSource, ChatUpdatesSignalSource, CommandOutputSource, ConnectivityStatusSource,
};

// ─── Stub implementations ───────────────────────────────────────────────────

#[derive(Default)]
pub struct StubConnectivityStatusSource;

impl ConnectivityStatusSource for StubConnectivityStatusSource {
    fn next_status(&mut self) -> Option<ConnectivityStatus> {
        None
    }
}

#[derive(Default)]
pub struct StubChatUpdatesSignalSource;

impl ChatUpdatesSignalSource for StubChatUpdatesSignalSource {
    fn pending_updates(&mut self) -> Option<Vec<ChatUpdate>> {
        None
    }
}

#[derive(Default)]
pub struct StubBackgroundResultSource;

impl BackgroundResultSource for StubBackgroundResultSource {
    fn next_result(&mut self) -> Option<BackgroundTaskResult> {
        None
    }
}

#[derive(Default)]
pub struct StubCommandOutputSource;

impl CommandOutputSource for StubCommandOutputSource {
    fn next_command_event(&mut self) -> Option<CommandEvent> {
        None
    }
}

// ─── Channel implementations ────────────────────────────────────────────────

pub struct ChannelConnectivityStatusSource {
    receiver: Receiver<ConnectivityStatus>,
    latest: Option<ConnectivityStatus>,
}

impl ChannelConnectivityStatusSource {
    pub fn new(receiver: Receiver<ConnectivityStatus>) -> Self {
        Self {
            receiver,
            latest: None,
        }
    }

    #[cfg(test)]
    pub fn from_values(statuses: Vec<ConnectivityStatus>) -> Self {
        let (tx, rx) = mpsc::channel();
        for status in statuses {
            tx.send(status)
                .expect("status should be sent into test channel");
        }

        Self::new(rx)
    }
}

impl ConnectivityStatusSource for ChannelConnectivityStatusSource {
    fn next_status(&mut self) -> Option<ConnectivityStatus> {
        while let Ok(status) = self.receiver.try_recv() {
            self.latest = Some(status);
        }

        self.latest.take()
    }
}

pub struct ChannelChatUpdatesSignalSource {
    receiver: Receiver<ChatUpdate>,
}

impl ChannelChatUpdatesSignalSource {
    pub fn new(receiver: Receiver<ChatUpdate>) -> Self {
        Self { receiver }
    }

    #[cfg(test)]
    pub fn from_updates(updates: Vec<ChatUpdate>) -> Self {
        let (tx, rx) = mpsc::channel();
        for update in updates {
            tx.send(update).expect("update should be sent");
        }
        Self::new(rx)
    }
}

impl ChatUpdatesSignalSource for ChannelChatUpdatesSignalSource {
    fn pending_updates(&mut self) -> Option<Vec<ChatUpdate>> {
        let mut updates = Vec::new();
        while let Ok(update) = self.receiver.try_recv() {
            updates.push(update);
        }
        if updates.is_empty() {
            None
        } else {
            Some(updates)
        }
    }
}

pub struct ChannelBackgroundResultSource {
    receiver: Receiver<BackgroundTaskResult>,
}

impl ChannelBackgroundResultSource {
    pub fn new(receiver: Receiver<BackgroundTaskResult>) -> Self {
        Self { receiver }
    }
}

impl BackgroundResultSource for ChannelBackgroundResultSource {
    fn next_result(&mut self) -> Option<BackgroundTaskResult> {
        self.receiver.try_recv().ok()
    }
}

pub struct ChannelCommandOutputSource {
    receiver: Receiver<CommandEvent>,
}

impl ChannelCommandOutputSource {
    pub fn new(receiver: Receiver<CommandEvent>) -> Self {
        Self { receiver }
    }
}

impl CommandOutputSource for ChannelCommandOutputSource {
    fn next_command_event(&mut self) -> Option<CommandEvent> {
        self.receiver.try_recv().ok()
    }
}
