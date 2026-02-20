use anyhow::Result;

use crate::domain::{events::AppEvent, shell_state::ShellState};

pub trait AppEventSource {
    fn next_event(&mut self) -> Result<Option<AppEvent>>;
}

pub trait ShellOrchestrator {
    fn state(&self) -> &ShellState;
    fn state_mut(&mut self) -> &mut ShellState;
    fn handle_event(&mut self, event: AppEvent) -> Result<()>;
}
