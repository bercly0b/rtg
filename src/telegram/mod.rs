//! Telegram integration layer: API clients and event mapping.

mod auth;
mod connectivity;

use std::sync::mpsc::Sender;

use auth::GrammersAuthBackend;

pub use connectivity::{ConnectivityMonitorStartError, TelegramConnectivityMonitor};

use crate::{
    domain::events::ConnectivityStatus,
    infra::{config::TelegramConfig, storage_layout::StorageLayout},
    usecases::guided_auth::{AuthBackendError, AuthCodeToken, SignInOutcome, TelegramAuthClient},
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum BackendKind {
    Stub,
    Grammers,
}

pub struct TelegramAdapter {
    backend_kind: BackendKind,
    auth_backend: Option<GrammersAuthBackend>,
}

impl std::fmt::Debug for TelegramAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TelegramAdapter")
            .field("backend_kind", &self.backend_kind)
            .finish()
    }
}

impl TelegramAdapter {
    pub fn stub() -> Self {
        Self {
            backend_kind: BackendKind::Stub,
            auth_backend: None,
        }
    }

    pub fn from_config(config: &TelegramConfig) -> Result<Self, AuthBackendError> {
        if !config.is_configured() {
            return Ok(Self::stub());
        }

        let session_path = StorageLayout::resolve()
            .map_err(|error| AuthBackendError::Transient {
                code: "AUTH_SESSION_STORE_UNAVAILABLE",
                message: format!("failed to resolve storage layout: {error}"),
            })?
            .session_file();

        let backend = GrammersAuthBackend::new(config, &session_path)?;
        Ok(Self {
            backend_kind: BackendKind::Grammers,
            auth_backend: Some(backend),
        })
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn uses_real_backend(&self) -> bool {
        matches!(self.backend_kind, BackendKind::Grammers)
    }

    pub fn start_connectivity_monitor(
        &self,
        status_tx: Sender<ConnectivityStatus>,
    ) -> Result<TelegramConnectivityMonitor, ConnectivityMonitorStartError> {
        TelegramConnectivityMonitor::start(status_tx)
    }
}

impl TelegramAuthClient for TelegramAdapter {
    fn request_login_code(&mut self, phone: &str) -> Result<AuthCodeToken, AuthBackendError> {
        match self.auth_backend.as_mut() {
            Some(backend) => backend.request_login_code(phone),
            None => Err(AuthBackendError::Transient {
                code: "AUTH_BACKEND_UNAVAILABLE",
                message: "Telegram auth backend is not configured".into(),
            }),
        }
    }

    fn sign_in_with_code(
        &mut self,
        token: &AuthCodeToken,
        code: &str,
    ) -> Result<SignInOutcome, AuthBackendError> {
        match self.auth_backend.as_mut() {
            Some(backend) => backend.sign_in_with_code(token, code),
            None => Err(AuthBackendError::Transient {
                code: "AUTH_BACKEND_UNAVAILABLE",
                message: "Telegram auth backend is not configured".into(),
            }),
        }
    }

    fn verify_password(&mut self, password: &str) -> Result<(), AuthBackendError> {
        match self.auth_backend.as_mut() {
            Some(backend) => backend.verify_password(password),
            None => Err(AuthBackendError::Transient {
                code: "AUTH_BACKEND_UNAVAILABLE",
                message: "Telegram auth backend is not configured".into(),
            }),
        }
    }

    fn persist_authorized_session(
        &mut self,
        session_path: &std::path::Path,
    ) -> Result<(), AuthBackendError> {
        match self.auth_backend.as_mut() {
            Some(backend) => backend.persist_authorized_session(session_path),
            None => Err(AuthBackendError::Transient {
                code: "AUTH_BACKEND_UNAVAILABLE",
                message: "Telegram auth backend is not configured".into(),
            }),
        }
    }
}

/// Returns the telegram module name for smoke checks.
pub fn module_name() -> &'static str {
    "telegram"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_stub_backend_when_config_is_not_set() {
        let adapter =
            TelegramAdapter::from_config(&TelegramConfig::default()).expect("stub adapter");
        assert!(!adapter.uses_real_backend());
    }
}
