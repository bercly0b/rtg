use std::{path::Path, sync::mpsc::Sender};

use crate::{
    domain::events::ConnectivityStatus,
    infra::{
        self,
        config::{FileConfigAdapter, TelegramConfig},
        contracts::ConfigAdapter,
        error::AppError,
        stubs::{NoopOpener, StubStorageAdapter},
    },
    telegram::{ConnectivityMonitorStartError, TelegramAdapter, TelegramConnectivityMonitor},
    ui::{ChannelConnectivityStatusSource, CrosstermEventSource},
    usecases::{
        context::AppContext,
        contracts::{AppEventSource, ShellOrchestrator},
        shell::DefaultShellOrchestrator,
    },
};

const CONNECTIVITY_MONITOR_START_FAILED: &str = "TELEGRAM_CONNECTIVITY_MONITOR_START_FAILED";

pub struct ShellComposition {
    pub event_source: Box<dyn AppEventSource>,
    pub orchestrator: Box<dyn ShellOrchestrator>,
    _connectivity_monitor: Option<TelegramConnectivityMonitor>,
}

pub fn bootstrap(config_path: Option<&Path>) -> Result<AppContext, AppError> {
    let context = build_context(config_path)?;
    infra::logging::init(&context.config.logging)?;

    Ok(context)
}

pub fn compose_shell(context: &AppContext) -> ShellComposition {
    compose_shell_with_factory(context, &RealConnectivityMonitorFactory)
}

fn compose_shell_with_factory(
    context: &AppContext,
    monitor_factory: &dyn ConnectivityMonitorFactory,
) -> ShellComposition {
    let mut connectivity_monitor = None;
    let event_source: Box<dyn AppEventSource> = if context.config.telegram.is_configured() {
        let (status_tx, status_rx) = std::sync::mpsc::channel::<ConnectivityStatus>();
        match monitor_factory.start(&context.telegram, status_tx) {
            Ok(monitor) => {
                connectivity_monitor = Some(monitor);
                Box::new(CrosstermEventSource::new(Box::new(
                    ChannelConnectivityStatusSource::new(status_rx),
                )))
            }
            Err(error) => {
                tracing::warn!(
                    code = CONNECTIVITY_MONITOR_START_FAILED,
                    error = %error,
                    "telegram connectivity monitor failed to start; using safe fallback"
                );
                Box::new(CrosstermEventSource::default())
            }
        }
    } else {
        Box::new(CrosstermEventSource::default())
    };

    ShellComposition {
        event_source,
        orchestrator: Box::new(DefaultShellOrchestrator::new(
            StubStorageAdapter::default(),
            NoopOpener,
        )),
        _connectivity_monitor: connectivity_monitor,
    }
}

fn build_context(config_path: Option<&Path>) -> Result<AppContext, AppError> {
    let config_adapter = FileConfigAdapter::new(config_path);
    build_context_with(&config_adapter)
}

fn build_context_with(config_adapter: &dyn ConfigAdapter) -> Result<AppContext, AppError> {
    let config = config_adapter.load().map_err(AppError::Other)?;
    validate_telegram_config(&config.telegram)?;

    let telegram = TelegramAdapter::from_config(&config.telegram).map_err(|error| {
        AppError::Other(anyhow::anyhow!(
            "telegram bootstrap failed [{}]",
            crate::infra::secrets::sanitize_error_code(match error {
                crate::usecases::guided_auth::AuthBackendError::Transient { code, .. } => code,
                _ => "AUTH_BACKEND_UNAVAILABLE",
            })
        ))
    })?;

    Ok(AppContext::new(config, telegram))
}

fn validate_telegram_config(config: &TelegramConfig) -> Result<(), AppError> {
    let api_hash_is_default = config.api_hash == TelegramConfig::default().api_hash;
    let api_hash_missing = config.api_hash.trim().is_empty() || api_hash_is_default;
    let api_id_missing = config.api_id <= 0;

    let partially_configured = (config.api_id > 0 && api_hash_missing)
        || (config.api_hash.trim() != "" && !api_hash_is_default && api_id_missing);

    if partially_configured {
        return Err(AppError::ConfigValidation {
            code: "TELEGRAM_CONFIG_INVALID",
            details: "telegram.api_id and telegram.api_hash must both be set for real backend bootstrap".to_owned(),
        });
    }

    Ok(())
}

trait ConnectivityMonitorFactory {
    fn start(
        &self,
        telegram: &TelegramAdapter,
        status_tx: Sender<ConnectivityStatus>,
    ) -> Result<TelegramConnectivityMonitor, ConnectivityMonitorStartError>;
}

struct RealConnectivityMonitorFactory;

impl ConnectivityMonitorFactory for RealConnectivityMonitorFactory {
    fn start(
        &self,
        telegram: &TelegramAdapter,
        status_tx: Sender<ConnectivityStatus>,
    ) -> Result<TelegramConnectivityMonitor, ConnectivityMonitorStartError> {
        telegram.start_connectivity_monitor(status_tx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::events::AppEvent,
        infra::{config::AppConfig, stubs::StubConfigAdapter},
    };

    struct StubConnectivityMonitorFactory {
        should_fail: bool,
    }

    impl ConnectivityMonitorFactory for StubConnectivityMonitorFactory {
        fn start(
            &self,
            _telegram: &TelegramAdapter,
            status_tx: Sender<ConnectivityStatus>,
        ) -> Result<TelegramConnectivityMonitor, ConnectivityMonitorStartError> {
            if self.should_fail {
                return Err(ConnectivityMonitorStartError::StartupRejected);
            }

            status_tx
                .send(ConnectivityStatus::Connected)
                .expect("test status should be sent");

            Ok(TelegramConnectivityMonitor::inert())
        }
    }

    #[test]
    fn builds_context_with_default_config_when_file_is_missing() {
        let context = build_context(Some(Path::new("./missing-config.toml")))
            .expect("context should build from defaults");

        assert_eq!(context.config, crate::infra::config::AppConfig::default());
        assert!(!context.telegram.uses_real_backend());
    }

    #[test]
    fn builds_context_via_config_contract() {
        let adapter = StubConfigAdapter;
        let context =
            build_context_with(&adapter).expect("context should build from config adapter");

        assert_eq!(context.config, crate::infra::config::AppConfig::default());
        assert!(!context.telegram.uses_real_backend());
    }

    #[test]
    fn rejects_partially_configured_telegram_config() {
        let mut config = AppConfig::default();
        config.telegram.api_id = 100;

        let error = validate_telegram_config(&config.telegram).expect_err("must fail");
        let rendered = error.to_string();

        assert!(rendered.contains("TELEGRAM_CONFIG_INVALID"));
        assert!(rendered.contains("telegram.api_id"));
        assert!(!rendered.contains("api_hash ="));
    }

    #[test]
    fn composes_shell_dependencies_in_bootstrap_layer() {
        let context = AppContext::new(AppConfig::default(), TelegramAdapter::stub());
        let mut shell = compose_shell(&context);

        assert!(shell.orchestrator.state().is_running());

        shell
            .orchestrator
            .handle_event(AppEvent::QuitRequested)
            .expect("quit event should be handled");

        assert!(!shell.orchestrator.state().is_running());
    }

    #[test]
    fn compose_shell_injects_channel_backed_source_when_telegram_monitor_starts() {
        let mut config = AppConfig::default();
        config.telegram = TelegramConfig {
            api_id: 100,
            api_hash: "configured".to_owned(),
        };
        let context = AppContext::new(config, TelegramAdapter::stub());

        let factory = StubConnectivityMonitorFactory { should_fail: false };

        let mut shell = compose_shell_with_factory(&context, &factory);
        let event = shell
            .event_source
            .next_event()
            .expect("event should be readable");

        assert_eq!(
            event,
            Some(AppEvent::ConnectivityChanged(ConnectivityStatus::Connected))
        );
    }

    #[test]
    fn compose_shell_falls_back_when_telegram_monitor_start_fails() {
        let mut config = AppConfig::default();
        config.telegram = TelegramConfig {
            api_id: 100,
            api_hash: "configured".to_owned(),
        };
        let context = AppContext::new(config, TelegramAdapter::stub());

        let factory = StubConnectivityMonitorFactory { should_fail: true };

        let mut shell = compose_shell_with_factory(&context, &factory);
        shell
            .orchestrator
            .handle_event(AppEvent::QuitRequested)
            .expect("fallback composition should still wire orchestrator");

        assert!(!shell.orchestrator.state().is_running());
    }
}
