use anyhow::Result;

use crate::usecases::{
    context::AppContext,
    contracts::{AppEventSource, ShellOrchestrator},
};

use super::{
    event_source::{ChannelCommandOutputSource, CrosstermEventSource},
    terminal::TerminalSession,
    view,
};

pub fn start(
    context: &AppContext,
    event_source: &mut CrosstermEventSource,
    orchestrator: &mut dyn ShellOrchestrator,
) -> Result<()> {
    tracing::info!(
        log_level = %context.config.logging.level,
        telegram_adapter = ?context.telegram,
        "starting TUI shell"
    );

    let mut terminal = TerminalSession::new()?;

    let mut had_command_popup = false;

    while orchestrator.state().is_running() {
        terminal.draw(|frame| view::render(frame, orchestrator.state_mut()))?;

        if let Some(event) = event_source.next_event()? {
            orchestrator.handle_event(event)?;
        }

        // Wire up command output channels when a new command starts.
        if let Some(rx) = orchestrator.take_pending_command_rx() {
            event_source.set_command_output_source(Box::new(ChannelCommandOutputSource::new(rx)));
            had_command_popup = true;
        }

        // Clear the command output source once when the popup closes.
        if had_command_popup && orchestrator.state().command_popup().is_none() {
            event_source.clear_command_output_source();
            had_command_popup = false;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::events::AppEvent,
        infra::{contracts::ExternalOpener, stubs::StubStorageAdapter},
        ui::event_source::MockEventSource,
        usecases::{background::tests::StubTaskDispatcher, shell::DefaultShellOrchestrator},
    };

    #[derive(Debug, Default)]
    struct NoopOpener;

    impl ExternalOpener for NoopOpener {
        fn open(&self, _target: &str) -> anyhow::Result<()> {
            Ok(())
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
        let (dispatcher, _rx) = StubTaskDispatcher::new();
        let mut orchestrator =
            DefaultShellOrchestrator::new(StubStorageAdapter::default(), NoopOpener, dispatcher);

        if let Some(event) = source.next_event().expect("must read mock event") {
            orchestrator
                .handle_event(event)
                .expect("must handle quit event");
        }

        assert!(!orchestrator.state().is_running());
    }
}
