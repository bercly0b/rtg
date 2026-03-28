mod composition;
mod context;

use std::sync::mpsc::Sender;

use crate::{
    domain::events::ConnectivityStatus,
    infra::{
        config::{AppConfig, TelegramConfig},
        contracts::ConfigAdapter,
    },
    telegram::{
        ChatUpdatesMonitorStartError, ConnectivityMonitorStartError, TelegramAdapter,
        TelegramChatUpdatesMonitor, TelegramConnectivityMonitor,
    },
    usecases::guided_auth::AuthBackendError,
};

use super::{ConnectivityMonitorFactory, TelegramAdapterFactory};

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
    fn from_config(&self, _config: &TelegramConfig) -> Result<TelegramAdapter, AuthBackendError> {
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
        updates_tx: Sender<crate::domain::events::ChatUpdate>,
    ) -> Result<TelegramChatUpdatesMonitor, ChatUpdatesMonitorStartError> {
        if self.chat_updates_should_fail {
            return Err(ChatUpdatesMonitorStartError::StartupRejected);
        }

        updates_tx
            .send(crate::domain::events::ChatUpdate::ChatMetadataChanged { chat_id: 1 })
            .expect("chat update signal should be sent");

        Ok(TelegramChatUpdatesMonitor::inert())
    }
}
