use std::path::Path;

use crate::{
    infra::{
        self,
        config::FileConfigAdapter,
        contracts::ConfigAdapter,
        error::AppError,
        stubs::{NoopOpener, StubStorageAdapter},
    },
    ui::CrosstermEventSource,
    usecases::{
        context::AppContext,
        contracts::{AppEventSource, ShellOrchestrator},
        shell::DefaultShellOrchestrator,
    },
};

pub struct ShellComposition {
    pub event_source: Box<dyn AppEventSource>,
    pub orchestrator: Box<dyn ShellOrchestrator>,
}

pub fn bootstrap(config_path: Option<&Path>) -> Result<AppContext, AppError> {
    let context = build_context(config_path)?;
    infra::logging::init(&context.config.logging)?;

    Ok(context)
}

pub fn compose_shell() -> ShellComposition {
    ShellComposition {
        event_source: Box::new(CrosstermEventSource),
        orchestrator: Box::new(DefaultShellOrchestrator::new(
            StubStorageAdapter::default(),
            NoopOpener,
        )),
    }
}

fn build_context(config_path: Option<&Path>) -> Result<AppContext, AppError> {
    let config_adapter = FileConfigAdapter::new(config_path);
    build_context_with(&config_adapter)
}

fn build_context_with(config_adapter: &dyn ConfigAdapter) -> Result<AppContext, AppError> {
    let config = config_adapter.load().map_err(AppError::Other)?;
    Ok(AppContext::new(config))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{domain::events::AppEvent, infra::stubs::StubConfigAdapter};

    #[test]
    fn builds_context_with_default_config_when_file_is_missing() {
        let context = build_context(Some(Path::new("./missing-config.toml")))
            .expect("context should build from defaults");

        assert_eq!(context.config, crate::infra::config::AppConfig::default());
    }

    #[test]
    fn builds_context_via_config_contract() {
        let adapter = StubConfigAdapter;
        let context =
            build_context_with(&adapter).expect("context should build from config adapter");

        assert_eq!(context.config, crate::infra::config::AppConfig::default());
    }

    #[test]
    fn composes_shell_dependencies_in_bootstrap_layer() {
        let mut shell = compose_shell();

        assert!(shell.orchestrator.state().is_running());

        shell
            .orchestrator
            .handle_event(AppEvent::QuitRequested)
            .expect("quit event should be handled");

        assert!(!shell.orchestrator.state().is_running());
    }
}
