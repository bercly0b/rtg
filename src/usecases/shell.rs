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
                    return Ok(());
                }

                match key.key.as_str() {
                    "j" => self.state.chat_list_mut().select_next(),
                    "k" => self.state.chat_list_mut().select_previous(),
                    "enter" => {
                        if self.state.chat_list().selected_chat().is_some() {
                            self.storage.save_last_action("open_chat_intent")?;
                        }
                    }
                    _ => {}
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
        domain::{
            chat::ChatSummary,
            events::{ConnectivityStatus, KeyInput},
        },
        infra::stubs::{NoopOpener, StubStorageAdapter},
    };

    fn chat(chat_id: i64, title: &str) -> ChatSummary {
        ChatSummary {
            chat_id,
            title: title.to_owned(),
            unread_count: 0,
            last_message_preview: None,
            last_message_unix_ms: None,
        }
    }

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

    #[test]
    fn key_contract_navigates_chat_list_with_vim_keys() {
        let mut orchestrator =
            DefaultShellOrchestrator::new(StubStorageAdapter::default(), NoopOpener::default());
        orchestrator.state.chat_list_mut().set_ready(vec![
            chat(1, "General"),
            chat(2, "Backend"),
            chat(3, "Ops"),
        ]);

        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("j", false)))
            .expect("j key should be handled");
        assert_eq!(orchestrator.state().chat_list().selected_index(), Some(1));

        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("k", false)))
            .expect("k key should be handled");
        assert_eq!(orchestrator.state().chat_list().selected_index(), Some(0));
    }

    #[test]
    fn key_contract_enter_triggers_open_chat_intent_placeholder() {
        let mut orchestrator =
            DefaultShellOrchestrator::new(StubStorageAdapter::default(), NoopOpener::default());
        orchestrator
            .state
            .chat_list_mut()
            .set_ready(vec![chat(1, "General")]);

        orchestrator
            .handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
            .expect("enter key should be handled");

        assert_eq!(
            orchestrator.storage.last_action,
            Some("open_chat_intent".to_owned())
        );
    }
}
