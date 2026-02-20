use anyhow::Result;

use crate::usecases::{
    context::AppContext,
    contracts::{AppEventSource, ShellOrchestrator},
};

use super::{terminal::TerminalSession, view};

pub fn start(
    context: &AppContext,
    event_source: &mut dyn AppEventSource,
    orchestrator: &mut dyn ShellOrchestrator,
) -> Result<()> {
    tracing::info!(
        log_level = %context.config.logging.level,
        telegram_adapter = ?context.telegram,
        "starting TUI shell"
    );

    let mut terminal = TerminalSession::new()?;

    while orchestrator.state().is_running() {
        terminal.draw(|frame| view::render(frame, orchestrator.state()))?;

        if let Some(event) = event_source.next_event()? {
            orchestrator.handle_event(event)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{chat::ChatSummary, events::AppEvent, message::Message},
        infra::stubs::{NoopOpener, StubStorageAdapter},
        ui::event_source::MockEventSource,
        usecases::{
            list_chats::{ListChatsSource, ListChatsSourceError},
            load_messages::{MessagesSource, MessagesSourceError},
            shell::DefaultShellOrchestrator,
        },
    };

    struct EmptyChatsSource;

    impl ListChatsSource for EmptyChatsSource {
        fn list_chats(&self, _limit: usize) -> Result<Vec<ChatSummary>, ListChatsSourceError> {
            Ok(vec![])
        }
    }

    struct EmptyMessagesSource;

    impl MessagesSource for EmptyMessagesSource {
        fn list_messages(
            &self,
            _chat_id: i64,
            _limit: usize,
        ) -> Result<Vec<Message>, MessagesSourceError> {
            Ok(vec![])
        }
    }

    #[test]
    fn mock_source_produces_quit_event() {
        let mut source = MockEventSource::from(vec![AppEvent::QuitRequested]);
        let event = source.next_event().expect("must read mock event");

        assert_eq!(event, Some(AppEvent::QuitRequested));
    }

    #[test]
    fn orchestrator_stops_on_quit_from_source() {
        let mut source = MockEventSource::from(vec![AppEvent::QuitRequested]);
        let mut orchestrator = DefaultShellOrchestrator::new(
            StubStorageAdapter::default(),
            NoopOpener,
            EmptyChatsSource,
            EmptyMessagesSource,
        );

        if let Some(event) = source.next_event().expect("must read mock event") {
            orchestrator
                .handle_event(event)
                .expect("must handle quit event");
        }

        assert!(!orchestrator.state().is_running());
    }
}
