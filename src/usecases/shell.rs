use anyhow::Result;

use crate::{
    domain::{events::AppEvent, shell_state::ShellState},
    infra::contracts::{ExternalOpener, StorageAdapter},
};

use super::contracts::ShellOrchestrator;

pub struct DefaultShellOrchestrator<S, O>
where
    S: StorageAdapter,
    O: ExternalOpener,
{
    state: ShellState,
    storage: S,
    opener: O,
}

impl<S, O> DefaultShellOrchestrator<S, O>
where
    S: StorageAdapter,
    O: ExternalOpener,
{
    pub fn new(storage: S, opener: O) -> Self {
        Self {
            state: ShellState::default(),
            storage,
            opener,
        }
    }
}

impl<S, O> ShellOrchestrator for DefaultShellOrchestrator<S, O>
where
    S: StorageAdapter,
    O: ExternalOpener,
{
    fn state(&self) -> &ShellState {
        &self.state
    }

    fn handle_event(&mut self, event: AppEvent) -> Result<()> {
        match event {
            AppEvent::Tick => {
                self.storage.save_last_action("tick")?;
            }
            AppEvent::QuitRequested => self.state.stop(),
            AppEvent::InputKey(key) => {
                if key.ctrl && key.key == "o" {
                    self.opener.open("about:blank")?;
                    self.storage.save_last_action("open")?;
                }
            }
            AppEvent::ConnectivityChanged(status) => {
                self.state.set_connectivity_status(status);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::events::{ConnectivityStatus, KeyInput},
        infra::stubs::{NoopOpener, StubStorageAdapter},
    };

    #[test]
    fn stops_on_quit_event() {
        let mut orchestrator =
            DefaultShellOrchestrator::new(StubStorageAdapter::default(), NoopOpener::default());

        orchestrator
            .handle_event(AppEvent::QuitRequested)
            .expect("event must be handled");

        assert!(!orchestrator.state().is_running());
    }

    #[test]
    fn keeps_running_on_regular_key() {
        let mut orchestrator =
            DefaultShellOrchestrator::new(StubStorageAdapter::default(), NoopOpener::default());

        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("x", false)))
            .expect("event must be handled");

        assert!(orchestrator.state().is_running());
    }

    #[test]
    fn updates_connectivity_status_on_connectivity_event() {
        let mut orchestrator =
            DefaultShellOrchestrator::new(StubStorageAdapter::default(), NoopOpener::default());

        orchestrator
            .handle_event(AppEvent::ConnectivityChanged(
                ConnectivityStatus::Disconnected,
            ))
            .expect("connectivity event must be handled");

        assert_eq!(
            orchestrator.state().connectivity_status(),
            ConnectivityStatus::Disconnected
        );
    }
}
