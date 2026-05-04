mod sources;

#[cfg(test)]
mod tests;

use std::time::Duration;

#[cfg(test)]
use std::collections::VecDeque;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::{
    domain::events::{AppEvent, BackgroundTaskResult, ChatUpdate, ConnectivityStatus, KeyInput},
    usecases::contracts::AppEventSource,
};

pub use crate::domain::events::CommandEvent;
pub use sources::*;

const EVENT_POLL_TIMEOUT: Duration = Duration::from_millis(100);
const NON_BLOCKING_POLL_TIMEOUT: Duration = Duration::from_millis(0);
const MAX_CONNECTIVITY_STREAK: u8 = 3;
const MAX_CHAT_UPDATE_STREAK: u8 = 8;
const MAX_CONNECTIVITY_DRAIN_PER_CYCLE: usize = 32;

// ─── Source traits ──────────────────────────────────────────────────────────

pub trait ConnectivityStatusSource {
    fn next_status(&mut self) -> Option<ConnectivityStatus>;
}

pub trait ChatUpdatesSignalSource {
    fn pending_updates(&mut self) -> Option<Vec<ChatUpdate>>;
}

pub trait BackgroundResultSource {
    fn next_result(&mut self) -> Option<BackgroundTaskResult>;
}

pub trait CommandOutputSource {
    fn next_command_event(&mut self) -> Option<CommandEvent>;
}

// ─── Terminal event abstraction ─────────────────────────────────────────────

trait TerminalEventSource {
    fn poll(&mut self, timeout: Duration) -> Result<bool>;
    fn read(&mut self) -> Result<Event>;
}

struct CrosstermTerminalEventSource;

impl TerminalEventSource for CrosstermTerminalEventSource {
    fn poll(&mut self, timeout: Duration) -> Result<bool> {
        Ok(event::poll(timeout)?)
    }

    fn read(&mut self) -> Result<Event> {
        Ok(event::read()?)
    }
}

// ─── CrosstermEventSource ───────────────────────────────────────────────────

pub struct CrosstermEventSource {
    connectivity_source: Box<dyn ConnectivityStatusSource>,
    chat_updates_source: Box<dyn ChatUpdatesSignalSource>,
    background_result_source: Box<dyn BackgroundResultSource>,
    command_output_source: Box<dyn CommandOutputSource>,
    pending_connectivity: Option<ConnectivityStatus>,
    last_emitted_connectivity: Option<ConnectivityStatus>,
    connectivity_streak: u8,
    chat_update_streak: u8,
    prefer_chat_update: bool,
}

impl Default for CrosstermEventSource {
    fn default() -> Self {
        Self {
            connectivity_source: Box::new(StubConnectivityStatusSource),
            chat_updates_source: Box::new(StubChatUpdatesSignalSource),
            background_result_source: Box::new(StubBackgroundResultSource),
            command_output_source: Box::new(StubCommandOutputSource),
            pending_connectivity: None,
            last_emitted_connectivity: None,
            connectivity_streak: 0,
            chat_update_streak: 0,
            prefer_chat_update: true,
        }
    }
}

impl CrosstermEventSource {
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn new(connectivity_source: Box<dyn ConnectivityStatusSource>) -> Self {
        Self::with_sources(
            connectivity_source,
            Box::new(StubChatUpdatesSignalSource),
            Box::new(StubBackgroundResultSource),
        )
    }

    pub fn with_sources(
        connectivity_source: Box<dyn ConnectivityStatusSource>,
        chat_updates_source: Box<dyn ChatUpdatesSignalSource>,
        background_result_source: Box<dyn BackgroundResultSource>,
    ) -> Self {
        Self {
            connectivity_source,
            chat_updates_source,
            background_result_source,
            command_output_source: Box::new(StubCommandOutputSource),
            pending_connectivity: None,
            last_emitted_connectivity: None,
            connectivity_streak: 0,
            chat_update_streak: 0,
            prefer_chat_update: true,
        }
    }

    /// Replaces the command output source (called when an external command starts).
    pub fn set_command_output_source(&mut self, source: Box<dyn CommandOutputSource>) {
        self.command_output_source = source;
    }

    /// Clears the command output source (called when the command popup closes).
    pub fn clear_command_output_source(&mut self) {
        self.command_output_source = Box::new(StubCommandOutputSource);
    }

    fn next_event_with_terminal<T: TerminalEventSource>(
        &mut self,
        terminal: &mut T,
    ) -> Result<Option<AppEvent>> {
        self.refresh_pending_connectivity();

        let has_ready_terminal_input = terminal.poll(NON_BLOCKING_POLL_TIMEOUT).unwrap_or(false);
        if has_ready_terminal_input {
            self.connectivity_streak = 0;
            self.chat_update_streak = 0;
            if let Event::Key(key) = terminal.read()? {
                return Ok(map_key_event(key));
            }
            return Ok(None);
        }

        // Command output has the highest non-input priority so the popup
        // updates in real time while the external process is running.
        if let Some(cmd_event) = self.command_output_source.next_command_event() {
            self.connectivity_streak = 0;
            self.chat_update_streak = 0;
            return Ok(Some(match cmd_event {
                CommandEvent::OutputLine { text, replace_last } => {
                    AppEvent::CommandOutputLine { text, replace_last }
                }
                CommandEvent::Exited { success } => AppEvent::CommandExited { success },
            }));
        }

        // Background task results have high priority — deliver them before
        // chat updates to keep the UI responsive after dispatched operations.
        if let Some(result) = self.background_result_source.next_result() {
            self.connectivity_streak = 0;
            self.chat_update_streak = 0;
            return Ok(Some(AppEvent::BackgroundTaskCompleted(result)));
        }

        let connectivity_ready = self.connectivity_streak < MAX_CONNECTIVITY_STREAK
            && self.pending_connectivity.is_some();

        // Round-robin: when connectivity is pending and it's connectivity's turn, emit it first.
        if connectivity_ready && !self.prefer_chat_update {
            if let Some(status) = self.pending_connectivity.take() {
                self.connectivity_streak += 1;
                self.last_emitted_connectivity = Some(status);
                self.prefer_chat_update = true;
                return Ok(Some(AppEvent::ConnectivityChanged(status)));
            }
        }

        if self.chat_update_streak < MAX_CHAT_UPDATE_STREAK {
            if let Some(updates) = self.chat_updates_source.pending_updates() {
                self.connectivity_streak = 0;
                self.chat_update_streak += 1;
                if connectivity_ready {
                    self.prefer_chat_update = false;
                }
                tracing::debug!(
                    update_count = updates.len(),
                    "event source emitted chat update received"
                );
                return Ok(Some(AppEvent::ChatUpdateReceived { updates }));
            }
        }

        if self.connectivity_streak < MAX_CONNECTIVITY_STREAK {
            if let Some(status) = self.pending_connectivity.take() {
                self.connectivity_streak += 1;
                self.last_emitted_connectivity = Some(status);
                self.prefer_chat_update = true;
                return Ok(Some(AppEvent::ConnectivityChanged(status)));
            }
        }

        self.connectivity_streak = 0;
        self.chat_update_streak = 0;

        if !terminal.poll(EVENT_POLL_TIMEOUT)? {
            return Ok(Some(AppEvent::Tick));
        }

        if let Event::Key(key) = terminal.read()? {
            return Ok(map_key_event(key));
        }

        Ok(None)
    }

    fn refresh_pending_connectivity(&mut self) {
        for _ in 0..MAX_CONNECTIVITY_DRAIN_PER_CYCLE {
            let Some(status) = self.connectivity_source.next_status() else {
                break;
            };

            self.pending_connectivity = Some(status);
        }

        if self.pending_connectivity == self.last_emitted_connectivity {
            self.pending_connectivity = None;
        }
    }
}

impl AppEventSource for CrosstermEventSource {
    fn next_event(&mut self) -> Result<Option<AppEvent>> {
        let mut terminal = CrosstermTerminalEventSource;
        self.next_event_with_terminal(&mut terminal)
    }
}

// ─── Key mapping ────────────────────────────────────────────────────────────

fn map_key_event(key: KeyEvent) -> Option<AppEvent> {
    if key.kind != KeyEventKind::Press {
        return None;
    }

    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Some(AppEvent::QuitRequested);
    }

    if let KeyCode::Char(ch) = key.code {
        return Some(AppEvent::InputKey(KeyInput::new(
            ch.to_string(),
            key.modifiers.contains(KeyModifiers::CONTROL),
        )));
    }

    let special_key = match key.code {
        KeyCode::Enter => Some("enter"),
        KeyCode::Esc => Some("esc"),
        KeyCode::Backspace => Some("backspace"),
        KeyCode::Delete => Some("delete"),
        KeyCode::Left => Some("left"),
        KeyCode::Right => Some("right"),
        KeyCode::Home => Some("home"),
        KeyCode::End => Some("end"),
        _ => None,
    };

    special_key.map(|k| AppEvent::InputKey(KeyInput::new(k, false)))
}

// ─── Mock (test-only) ───────────────────────────────────────────────────────

#[cfg(test)]
pub struct MockEventSource {
    queue: VecDeque<AppEvent>,
    connectivity_queue: VecDeque<ConnectivityStatus>,
}

#[cfg(test)]
impl MockEventSource {
    pub fn from(events: Vec<AppEvent>) -> Self {
        Self {
            queue: events.into(),
            connectivity_queue: Default::default(),
        }
    }

    pub fn with_connectivity(events: Vec<AppEvent>, connectivity: Vec<ConnectivityStatus>) -> Self {
        Self {
            queue: events.into(),
            connectivity_queue: connectivity.into(),
        }
    }
}

#[cfg(test)]
impl AppEventSource for MockEventSource {
    fn next_event(&mut self) -> Result<Option<AppEvent>> {
        if let Some(status) = self.connectivity_queue.pop_front() {
            return Ok(Some(AppEvent::ConnectivityChanged(status)));
        }

        Ok(self.queue.pop_front())
    }
}
