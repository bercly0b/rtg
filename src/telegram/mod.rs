//! Telegram integration layer: API clients and event mapping.

mod connectivity;

use std::sync::mpsc::Sender;

pub use connectivity::{ConnectivityMonitorStartError, TelegramConnectivityMonitor};

use crate::{
    domain::events::ConnectivityStatus,
    usecases::guided_auth::{AuthBackendError, AuthCodeToken, SignInOutcome, TelegramAuthClient},
};

#[derive(Debug, Clone, Default)]
pub struct TelegramAdapter;

impl TelegramAdapter {
    pub fn stub() -> Self {
        Self
    }

    pub fn start_connectivity_monitor(
        &self,
        status_tx: Sender<ConnectivityStatus>,
    ) -> Result<TelegramConnectivityMonitor, ConnectivityMonitorStartError> {
        TelegramConnectivityMonitor::start(status_tx)
    }
}

impl TelegramAuthClient for TelegramAdapter {
    fn request_login_code(&mut self, _phone: &str) -> Result<AuthCodeToken, AuthBackendError> {
        Err(AuthBackendError::Transient {
            code: "AUTH_BACKEND_UNAVAILABLE",
            message: "Telegram auth backend is not connected yet".into(),
        })
    }

    fn sign_in_with_code(
        &mut self,
        _token: &AuthCodeToken,
        _code: &str,
    ) -> Result<SignInOutcome, AuthBackendError> {
        Err(AuthBackendError::Transient {
            code: "AUTH_BACKEND_UNAVAILABLE",
            message: "Telegram auth backend is not connected yet".into(),
        })
    }

    fn verify_password(&mut self, _password: &str) -> Result<(), AuthBackendError> {
        Err(AuthBackendError::Transient {
            code: "AUTH_BACKEND_UNAVAILABLE",
            message: "Telegram auth backend is not connected yet".into(),
        })
    }
}

/// Returns the telegram module name for smoke checks.
pub fn module_name() -> &'static str {
    "telegram"
}
