use std::sync::mpsc::Receiver;

use anyhow::Result;

use crate::domain::{
    events::{AppEvent, CommandEvent},
    shell_state::ShellState,
};

pub trait AppEventSource {
    fn next_event(&mut self) -> Result<Option<AppEvent>>;
}

pub trait ShellOrchestrator {
    fn state(&self) -> &ShellState;
    fn state_mut(&mut self) -> &mut ShellState;
    fn handle_event(&mut self, event: AppEvent) -> Result<()>;

    /// Takes the pending command event receiver, if a new command was just started.
    ///
    /// The shell loop calls this after each `handle_event` to wire the receiver
    /// into the event source for real-time command output streaming.
    fn take_pending_command_rx(&mut self) -> Option<Receiver<CommandEvent>>;
}
