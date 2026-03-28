mod channel;
mod crossterm_source;
mod key_mapping;
mod mock_event_source;

use std::collections::VecDeque;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::Event;

use crate::domain::events::{ChatUpdate, ConnectivityStatus};

use super::{ChatUpdatesSignalSource, ConnectivityStatusSource, TerminalEventSource};

// ─── Test doubles ───────────────────────────────────────────────────────────

struct TestConnectivitySource {
    statuses: VecDeque<ConnectivityStatus>,
}

impl TestConnectivitySource {
    fn from(statuses: Vec<ConnectivityStatus>) -> Self {
        Self {
            statuses: statuses.into(),
        }
    }
}

impl ConnectivityStatusSource for TestConnectivitySource {
    fn next_status(&mut self) -> Option<ConnectivityStatus> {
        self.statuses.pop_front()
    }
}

struct TestChatUpdatesSource {
    results: VecDeque<Option<Vec<ChatUpdate>>>,
}

impl TestChatUpdatesSource {
    fn from_bools(bools: Vec<bool>) -> Self {
        Self {
            results: bools
                .into_iter()
                .map(|b| {
                    if b {
                        Some(vec![ChatUpdate::ChatMetadataChanged { chat_id: 1 }])
                    } else {
                        None
                    }
                })
                .collect(),
        }
    }
}

impl ChatUpdatesSignalSource for TestChatUpdatesSource {
    fn pending_updates(&mut self) -> Option<Vec<ChatUpdate>> {
        self.results.pop_front().flatten()
    }
}

#[derive(Default)]
struct BurstyChatUpdatesSource;

impl ChatUpdatesSignalSource for BurstyChatUpdatesSource {
    fn pending_updates(&mut self) -> Option<Vec<ChatUpdate>> {
        Some(vec![ChatUpdate::ChatMetadataChanged { chat_id: 1 }])
    }
}

#[derive(Default)]
struct BurstyConnectivitySource {
    connected: bool,
}

impl ConnectivityStatusSource for BurstyConnectivitySource {
    fn next_status(&mut self) -> Option<ConnectivityStatus> {
        self.connected = !self.connected;
        Some(if self.connected {
            ConnectivityStatus::Connected
        } else {
            ConnectivityStatus::Disconnected
        })
    }
}

#[derive(Default)]
struct TestTerminalEventSource {
    polled: VecDeque<bool>,
    events: VecDeque<Event>,
}

impl TestTerminalEventSource {
    fn with_polls(polls: Vec<bool>) -> Self {
        Self {
            polled: polls.into(),
            events: VecDeque::new(),
        }
    }

    fn with_polls_and_events(polls: Vec<bool>, events: Vec<Event>) -> Self {
        Self {
            polled: polls.into(),
            events: events.into(),
        }
    }
}

impl TerminalEventSource for TestTerminalEventSource {
    fn poll(&mut self, _timeout: Duration) -> Result<bool> {
        Ok(self.polled.pop_front().unwrap_or(false))
    }

    fn read(&mut self) -> Result<Event> {
        Ok(self
            .events
            .pop_front()
            .expect("read is called only after poll=true in tests"))
    }
}
