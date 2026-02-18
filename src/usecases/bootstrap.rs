use std::{path::Path, sync::mpsc::Sender};

use crate::{
    domain::events::ConnectivityStatus,
    infra::{
        self,
        config::{FileConfigAdapter, TelegramConfig},
        contracts::ConfigAdapter,
        error::AppError,
        secrets::sanitize_error_code,
        stubs::{NoopOpener, StubStorageAdapter},
    },
    telegram::{
        ChatUpdatesMonitorStartError, ConnectivityMonitorStartError, TelegramAdapter,
        TelegramChatUpdatesMonitor, TelegramConnectivityMonitor,
    },
    ui::{ChannelChatUpdatesSignalSource, ChannelConnectivityStatusSource, CrosstermEventSource},
    usecases::{
        context::AppContext,
        contracts::{AppEventSource, ShellOrchestrator},
        guided_auth::AuthBackendError,
        shell::DefaultShellOrchestrator,
    },
};

const CONNECTIVITY_MONITOR_START_FAILED: &str = "TELEGRAM_CONNECTIVITY_MONITOR_START_FAILED";
const CHAT_UPDATES_MONITOR_START_FAILED: &str = "TELEGRAM_CHAT_UPDATES_MONITOR_START_FAILED";

pub struct ShellComposition<'a> {
    pub event_source: Box<dyn AppEventSource>,
    pub orchestrator: Box<dyn ShellOrchestrator + 'a>,
    _connectivity_monitor: Option<TelegramConnectivityMonitor>,
    _chat_updates_monitor: Option<TelegramChatUpdatesMonitor>,
}

pub fn bootstrap(config_path: Option<&Path>) -> Result<AppContext, AppError> {
    let context = build_context(config_path)?;
    infra::logging::init(&context.config.logging)?;

    Ok(context)
}

pub fn compose_shell(context: &AppContext) -> ShellComposition<'_> {
    compose_shell_with_factory(context, &RealConnectivityMonitorFactory)
}

fn compose_shell_with_factory<'a>(
    context: &'a AppContext,
    monitor_factory: &dyn ConnectivityMonitorFactory,
) -> ShellComposition<'a> {
    let mut connectivity_monitor = None;
    let mut chat_updates_monitor = None;
    let event_source: Box<dyn AppEventSource> = if context.config.telegram.is_configured() {
        let (status_tx, status_rx) = std::sync::mpsc::channel::<ConnectivityStatus>();
        let (updates_tx, updates_rx) = std::sync::mpsc::channel::<()>();

        match monitor_factory.start(&context.telegram, status_tx) {
            Ok(monitor) => {
                tracing::info!("telegram connectivity monitor started");
                connectivity_monitor = Some(monitor);
            }
            Err(error) => {
                tracing::warn!(
                    code = CONNECTIVITY_MONITOR_START_FAILED,
                    error = %error,
                    "telegram connectivity monitor failed to start; using safe fallback"
                );
            }
        }

        match monitor_factory.start_chat_updates(&context.telegram, updates_tx) {
            Ok(monitor) => {
                tracing::info!("telegram chat updates monitor wired into event source");
                chat_updates_monitor = Some(monitor);
            }
            Err(error) => {
                tracing::warn!(
                    code = CHAT_UPDATES_MONITOR_START_FAILED,
                    error = %error,
                    "telegram chat updates monitor failed to start; using safe fallback"
                );
            }
        }

        Box::new(CrosstermEventSource::with_sources(
            Box::new(ChannelConnectivityStatusSource::new(status_rx)),
            Box::new(ChannelChatUpdatesSignalSource::new(updates_rx)),
        ))
    } else {
        Box::new(CrosstermEventSource::default())
    };

    ShellComposition {
        event_source,
        orchestrator: Box::new(DefaultShellOrchestrator::new(
            StubStorageAdapter::default(),
            NoopOpener,
            &context.telegram,
        )),
        _connectivity_monitor: connectivity_monitor,
        _chat_updates_monitor: chat_updates_monitor,
    }
}

fn build_context(config_path: Option<&Path>) -> Result<AppContext, AppError> {
    let config_adapter = FileConfigAdapter::new(config_path);
    build_context_with(&config_adapter)
}

fn build_context_with(config_adapter: &dyn ConfigAdapter) -> Result<AppContext, AppError> {
    build_context_with_factories(config_adapter, &RealTelegramAdapterFactory)
}

fn build_context_with_factories(
    config_adapter: &dyn ConfigAdapter,
    telegram_factory: &dyn TelegramAdapterFactory,
) -> Result<AppContext, AppError> {
    let config = config_adapter.load().map_err(AppError::Other)?;
    validate_telegram_config(&config.telegram)?;

    let telegram = telegram_factory
        .from_config(&config.telegram)
        .map_err(map_telegram_bootstrap_error)?;

    Ok(AppContext::new(config, telegram))
}

fn map_telegram_bootstrap_error(error: AuthBackendError) -> AppError {
    let backend_code = match error {
        AuthBackendError::InvalidPhone => "AUTH_INVALID_PHONE".to_owned(),
        AuthBackendError::InvalidCode => "AUTH_INVALID_CODE".to_owned(),
        AuthBackendError::WrongPassword => "AUTH_WRONG_2FA".to_owned(),
        AuthBackendError::Timeout => "AUTH_TIMEOUT".to_owned(),
        AuthBackendError::FloodWait { .. } => "AUTH_FLOOD_WAIT".to_owned(),
        AuthBackendError::Transient { code, .. } => sanitize_error_code(code),
    };

    AppError::ConfigValidation {
        code: "TELEGRAM_BOOTSTRAP_FAILED",
        details: format!(
            "telegram client initialization failed [{backend_code}]; check telegram.api_id, telegram.api_hash, and network access"
        ),
    }
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
            details:
                "telegram.api_id and telegram.api_hash must both be set for real backend bootstrap"
                    .to_owned(),
        });
    }

    Ok(())
}

#[allow(clippy::wrong_self_convention)]
trait TelegramAdapterFactory {
    fn from_config(&self, config: &TelegramConfig) -> Result<TelegramAdapter, AuthBackendError>;
}

struct RealTelegramAdapterFactory;

impl TelegramAdapterFactory for RealTelegramAdapterFactory {
    fn from_config(&self, config: &TelegramConfig) -> Result<TelegramAdapter, AuthBackendError> {
        TelegramAdapter::from_config(config)
    }
}

trait ConnectivityMonitorFactory {
    fn start(
        &self,
        telegram: &TelegramAdapter,
        status_tx: Sender<ConnectivityStatus>,
    ) -> Result<TelegramConnectivityMonitor, ConnectivityMonitorStartError>;

    fn start_chat_updates(
        &self,
        telegram: &TelegramAdapter,
        updates_tx: Sender<()>,
    ) -> Result<TelegramChatUpdatesMonitor, ChatUpdatesMonitorStartError>;
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

    fn start_chat_updates(
        &self,
        telegram: &TelegramAdapter,
        updates_tx: Sender<()>,
    ) -> Result<TelegramChatUpdatesMonitor, ChatUpdatesMonitorStartError> {
        telegram.start_chat_updates_monitor(updates_tx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::events::AppEvent,
        infra::{config::AppConfig, contracts::ConfigAdapter, stubs::StubConfigAdapter},
        usecases::guided_auth::AuthBackendError,
    };

    struct FixedConfigAdapter {
        config: AppConfig,
    }

    impl ConfigAdapter for FixedConfigAdapter {
        fn load(&self) -> anyhow::Result<AppConfig> {
            Ok(self.config.clone())
        }
    }

    struct StubTelegramAdapterFactory {
        result: Result<(), AuthBackendError>,
    }

    impl TelegramAdapterFactory for StubTelegramAdapterFactory {
        fn from_config(
            &self,
            _config: &TelegramConfig,
        ) -> Result<TelegramAdapter, AuthBackendError> {
            match &self.result {
                Ok(()) => Ok(TelegramAdapter::stub()),
                Err(error) => Err(error.clone()),
            }
        }
    }

    struct StubConnectivityMonitorFactory {
        should_fail: bool,
        chat_updates_should_fail: bool,
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

        fn start_chat_updates(
            &self,
            _telegram: &TelegramAdapter,
            updates_tx: Sender<()>,
        ) -> Result<TelegramChatUpdatesMonitor, ChatUpdatesMonitorStartError> {
            if self.chat_updates_should_fail {
                return Err(ChatUpdatesMonitorStartError::StartupRejected);
            }

            updates_tx
                .send(())
                .expect("chat update signal should be sent");

            Ok(TelegramChatUpdatesMonitor::inert())
        }
    }

    #[test]
    fn builds_context_with_default_config_when_file_is_missing() {
        let config_adapter = crate::infra::config::FileConfigAdapter::without_env(Some(Path::new(
            "./missing-config.toml",
        )));
        let context =
            build_context_with(&config_adapter).expect("context should build from defaults");

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
    fn build_context_uses_stub_when_telegram_is_unconfigured() {
        let config_adapter = FixedConfigAdapter {
            config: AppConfig::default(),
        };
        let telegram_factory = StubTelegramAdapterFactory { result: Ok(()) };

        let context = build_context_with_factories(&config_adapter, &telegram_factory)
            .expect("unconfigured bootstrap should succeed");

        assert!(!context.telegram.uses_real_backend());
    }

    #[test]
    fn maps_transient_bootstrap_error_to_safe_validation_error() {
        let mut config = AppConfig::default();
        config.telegram = TelegramConfig {
            api_id: 777,
            api_hash: "hash".to_owned(),
        };

        let config_adapter = FixedConfigAdapter { config };
        let telegram_factory = StubTelegramAdapterFactory {
            result: Err(AuthBackendError::Transient {
                code: "AUTH_BACKEND_UNAVAILABLE",
                message: "token=supersecret".to_owned(),
            }),
        };

        let error = build_context_with_factories(&config_adapter, &telegram_factory)
            .expect_err("configured bootstrap should fail fast");
        let rendered = error.to_string();

        assert!(rendered.contains("TELEGRAM_BOOTSTRAP_FAILED"));
        assert!(rendered.contains("AUTH_BACKEND_UNAVAILABLE"));
        assert!(!rendered.contains("supersecret"));
    }

    #[test]
    fn maps_non_transient_bootstrap_errors_to_stable_codes() {
        let mut config = AppConfig::default();
        config.telegram = TelegramConfig {
            api_id: 777,
            api_hash: "hash".to_owned(),
        };

        let config_adapter = FixedConfigAdapter { config };
        let telegram_factory = StubTelegramAdapterFactory {
            result: Err(AuthBackendError::InvalidPhone),
        };

        let error = build_context_with_factories(&config_adapter, &telegram_factory)
            .expect_err("error should be mapped");

        assert!(error
            .to_string()
            .contains("telegram client initialization failed [AUTH_INVALID_PHONE]"));
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

        let factory = StubConnectivityMonitorFactory {
            should_fail: false,
            chat_updates_should_fail: false,
        };

        let mut shell = compose_shell_with_factory(&context, &factory);
        let first_event = shell
            .event_source
            .next_event()
            .expect("event should be readable");
        let second_event = shell
            .event_source
            .next_event()
            .expect("second event should be readable");

        let events = [first_event, second_event];
        assert!(events.contains(&Some(AppEvent::ChatListUpdateRequested)));
        assert!(events.contains(&Some(AppEvent::ConnectivityChanged(
            ConnectivityStatus::Connected
        ))));
    }

    #[test]
    fn compose_shell_falls_back_when_telegram_monitor_start_fails() {
        let mut config = AppConfig::default();
        config.telegram = TelegramConfig {
            api_id: 100,
            api_hash: "configured".to_owned(),
        };
        let context = AppContext::new(config, TelegramAdapter::stub());

        let factory = StubConnectivityMonitorFactory {
            should_fail: true,
            chat_updates_should_fail: true,
        };

        let mut shell = compose_shell_with_factory(&context, &factory);
        shell
            .orchestrator
            .handle_event(AppEvent::QuitRequested)
            .expect("fallback composition should still wire orchestrator");

        assert!(!shell.orchestrator.state().is_running());
    }

    #[test]
    fn compose_shell_keeps_chat_updates_when_only_connectivity_monitor_fails() {
        let mut config = AppConfig::default();
        config.telegram = TelegramConfig {
            api_id: 100,
            api_hash: "configured".to_owned(),
        };
        let context = AppContext::new(config, TelegramAdapter::stub());

        let factory = StubConnectivityMonitorFactory {
            should_fail: true,
            chat_updates_should_fail: false,
        };

        let mut shell = compose_shell_with_factory(&context, &factory);
        let event = shell
            .event_source
            .next_event()
            .expect("event should be readable");

        assert_eq!(event, Some(AppEvent::ChatListUpdateRequested));
    }
}
