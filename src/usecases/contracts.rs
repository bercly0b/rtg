use std::sync::mpsc::Receiver;

use anyhow::Result;

use crate::domain::{events, events::CommandEvent, keymap::Keymap, shell_state::ShellState};

use super::background::BackgroundTaskResult;

pub type AppEvent = events::AppEvent<BackgroundTaskResult>;

pub trait AppEventSource {
    fn next_event(&mut self) -> Result<Option<AppEvent>>;
}

/// Work the orchestrator hands back to the shell loop because it belongs to
/// the UI layer.
#[derive(Debug)]
pub enum ShellEffect {
    /// An external command started: stream its output into the event source.
    StreamCommandOutput(Receiver<CommandEvent>),
}

pub trait ShellOrchestrator {
    fn state(&self) -> &ShellState;
    fn state_mut(&mut self) -> &mut ShellState;
    fn keymap(&self) -> &Keymap;
    fn handle_event(&mut self, event: AppEvent) -> Result<()>;

    /// Drains the effects queued while handling events.
    ///
    /// The shell loop calls this after each `handle_event` and performs them.
    fn take_effects(&mut self) -> Vec<ShellEffect>;
}
