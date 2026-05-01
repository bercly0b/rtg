mod credentials_prompt;
mod validation;

use std::{io::IsTerminal, path::Path, sync::mpsc::Sender};

use crate::{
    domain::{
        events::{BackgroundTaskResult, ConnectivityStatus},
        shell_state::ShellState,
    },
    infra::{
        self,
        config::{FileConfigAdapter, TelegramConfig},
        contracts::ConfigAdapter,
        error::AppError,
        opener::BrowserOpener,
        stubs::StubStorageAdapter,
    },
    telegram::{
        ChatUpdatesMonitorStartError, ConnectivityMonitorStartError, TelegramAdapter,
        TelegramChatUpdatesMonitor, TelegramConnectivityMonitor,
    },
    ui::{
        ChannelBackgroundResultSource, ChannelChatUpdatesSignalSource,
        ChannelConnectivityStatusSource, CrosstermEventSource, StubChatUpdatesSignalSource,
        StubConnectivityStatusSource,
    },
    usecases::{
        background::ThreadTaskDispatcher,
        context::AppContext,
        contracts::ShellOrchestrator,
        guided_auth::{AuthBackendError, AuthTerminal, StdTerminal},
        shell::DefaultShellOrchestrator,
    },
};

use validation::{map_telegram_bootstrap_error, validate_telegram_config};

const CONNECTIVITY_MONITOR_START_FAILED: &str = "TELEGRAM_CONNECTIVITY_MONITOR_START_FAILED";
const CHAT_UPDATES_MONITOR_START_FAILED: &str = "TELEGRAM_CHAT_UPDATES_MONITOR_START_FAILED";

pub struct ShellComposition {
    pub event_source: Box<CrosstermEventSource>,
    pub orchestrator: Box<dyn ShellOrchestrator>,
    _connectivity_monitor: Option<TelegramConnectivityMonitor>,
    _chat_updates_monitor: Option<TelegramChatUpdatesMonitor>,
}

pub fn bootstrap(config_path: Option<&Path>) -> Result<AppContext, AppError> {
    let context = build_context(config_path)?;
    infra::logging::init(&context.config.logging)?;

    Ok(context)
}

pub fn compose_shell(context: &AppContext) -> ShellComposition {
    compose_shell_with_factory(context, &RealConnectivityMonitorFactory)
}

#[cfg_attr(not(test), allow(dead_code))]
fn compose_shell_with_factory(
    context: &AppContext,
    monitor_factory: &dyn ConnectivityMonitorFactory,
) -> ShellComposition {
    let mut connectivity_monitor = None;
    let mut chat_updates_monitor = None;

    let (bg_tx, bg_rx) = std::sync::mpsc::channel::<BackgroundTaskResult>();

    let event_source: Box<CrosstermEventSource> = if context.config.telegram.is_configured() {
        let (status_tx, status_rx) = std::sync::mpsc::channel::<ConnectivityStatus>();
        let (updates_tx, updates_rx) =
            std::sync::mpsc::channel::<crate::domain::events::ChatUpdate>();

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
            Box::new(ChannelBackgroundResultSource::new(bg_rx)),
        ))
    } else {
        Box::new(CrosstermEventSource::with_sources(
            Box::new(StubConnectivityStatusSource),
            Box::new(StubChatUpdatesSignalSource),
            Box::new(ChannelBackgroundResultSource::new(bg_rx)),
        ))
    };

    let dispatcher = ThreadTaskDispatcher::new(
        std::sync::Arc::clone(&context.telegram),
        std::sync::Arc::clone(&context.telegram),
        std::sync::Arc::clone(&context.telegram),
        std::sync::Arc::clone(&context.telegram),
        std::sync::Arc::clone(&context.telegram),
        bg_tx,
    );

    let cache_cfg = &context.config.cache;

    // Start with an empty Loading state for instant TUI display.
    // The first Tick in the event loop will trigger a background chat list
    // fetch, which populates the list asynchronously. This avoids blocking
    // the main thread with ~100 sequential TDLib calls during startup.
    let initial_state = ShellState::with_cache_limits(
        vec![],
        cache_cfg.max_cached_chats,
        cache_cfg.max_messages_per_chat,
    );

    // Provide the cache source to the orchestrator for instant message display.
    let cache_source: Option<
        std::sync::Arc<dyn crate::usecases::load_messages::CachedMessagesSource>,
    > = if context.config.telegram.is_configured() {
        Some(std::sync::Arc::clone(&context.telegram)
            as std::sync::Arc<
                dyn crate::usecases::load_messages::CachedMessagesSource,
            >)
    } else {
        None
    };

    ShellComposition {
        event_source,
        orchestrator: Box::new(DefaultShellOrchestrator::new_with_initial_state(
            StubStorageAdapter::default(),
            BrowserOpener,
            dispatcher,
            initial_state,
            cache_source,
            cache_cfg.min_display_messages,
            context.config.voice.record_cmd.clone(),
            context.config.open.handlers.clone(),
            context.config.download.max_auto_download_bytes(),
            context.config.keys.overrides.clone(),
        )),
        _connectivity_monitor: connectivity_monitor,
        _chat_updates_monitor: chat_updates_monitor,
    }
}

fn build_context(config_path: Option<&Path>) -> Result<AppContext, AppError> {
    let config_adapter = FileConfigAdapter::new(config_path);
    build_context_with(&config_adapter)
}

#[cfg_attr(not(test), allow(dead_code))]
fn build_context_with(config_adapter: &dyn ConfigAdapter) -> Result<AppContext, AppError> {
    build_context_with_factories(config_adapter, &RealTelegramAdapterFactory)
}

#[cfg_attr(not(test), allow(dead_code))]
fn build_context_with_factories(
    config_adapter: &dyn ConfigAdapter,
    telegram_factory: &dyn TelegramAdapterFactory,
) -> Result<AppContext, AppError> {
    let mut terminal = StdTerminal;
    let interactive = std::io::stdin().is_terminal();
    build_context_with_factories_inner(config_adapter, telegram_factory, &mut terminal, interactive)
}

fn build_context_with_factories_inner(
    config_adapter: &dyn ConfigAdapter,
    telegram_factory: &dyn TelegramAdapterFactory,
    terminal: &mut dyn AuthTerminal,
    interactive: bool,
) -> Result<AppContext, AppError> {
    let mut config = config_adapter.load().map_err(AppError::Other)?;

    validate_telegram_config(&config.telegram)?;

    credentials_prompt::ensure_telegram_credentials(
        &mut config,
        config_adapter,
        terminal,
        interactive,
    )?;

    let telegram = telegram_factory
        .from_config(&config.telegram)
        .map_err(map_telegram_bootstrap_error)?;

    Ok(AppContext::new(config, telegram))
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
        updates_tx: Sender<crate::domain::events::ChatUpdate>,
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
        updates_tx: Sender<crate::domain::events::ChatUpdate>,
    ) -> Result<TelegramChatUpdatesMonitor, ChatUpdatesMonitorStartError> {
        telegram.start_chat_updates_monitor(updates_tx)
    }
}

#[cfg(test)]
mod tests;
