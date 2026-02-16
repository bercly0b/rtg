use anyhow::Result;

use crate::{
    infra::stubs::{NoopOpener, StubStorageAdapter},
    usecases::{
        context::AppContext,
        contracts::{AppEventSource, ShellOrchestrator},
        shell::DefaultShellOrchestrator,
    },
};

use super::{event_source::CrosstermEventSource, terminal::TerminalSession, view};

pub fn start(context: &AppContext) -> Result<()> {
    tracing::info!(
        log_level = %context.config.logging.level,
        telegram_adapter = ?context.telegram,
        "starting TUI shell"
    );

    let mut terminal = TerminalSession::new()?;
    let mut source = CrosstermEventSource;
    let mut orchestrator = DefaultShellOrchestrator::new(StubStorageAdapter::default(), NoopOpener);

    while orchestrator.state().is_running() {
        terminal.draw(|frame| view::render(frame, orchestrator.state()))?;

        if let Some(event) = source.next_event()? {
            orchestrator.handle_event(event)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::events::AppEvent, ui::event_source::MockEventSource,
        usecases::shell::DefaultShellOrchestrator,
    };

    #[test]
    fn mock_source_produces_quit_event() {
        let mut source = MockEventSource::from(vec![AppEvent::QuitRequested]);
        let event = source.next_event().expect("must read mock event");

        assert_eq!(event, Some(AppEvent::QuitRequested));
    }

    #[test]
    fn orchestrator_stops_on_quit_from_source() {
        let mut source = MockEventSource::from(vec![AppEvent::QuitRequested]);
        let mut orchestrator =
            DefaultShellOrchestrator::new(StubStorageAdapter::default(), NoopOpener);

        if let Some(event) = source.next_event().expect("must read mock event") {
            orchestrator
                .handle_event(event)
                .expect("must handle quit event");
        }

        assert!(!orchestrator.state().is_running());
    }
}
