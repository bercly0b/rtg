//! Telegram integration layer: API clients and event mapping.

mod auth;
mod connectivity;
mod status_tracker;

use std::sync::mpsc::{Receiver, Sender};

use auth::GrammersAuthBackend;
use status_tracker::StatusTracker;

pub use connectivity::{ConnectivityMonitorStartError, TelegramConnectivityMonitor};

use crate::{
    domain::{events::ConnectivityStatus, status::AuthConnectivityStatus},
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
    status_tracker: StatusTracker,
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
            status_tracker: StatusTracker::new(),
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
            status_tracker: StatusTracker::new(),
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
        let tracker = self.status_tracker.clone();
        TelegramConnectivityMonitor::start(status_tx, move |status| {
            tracker.on_connectivity_changed(status);
        })
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn subscribe_status(&self) -> Receiver<AuthConnectivityStatus> {
        self.status_tracker.subscribe()
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn status_snapshot(&self) -> AuthConnectivityStatus {
        self.status_tracker.snapshot()
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn record_connectivity_status(&self, status: ConnectivityStatus) {
        self.status_tracker.on_connectivity_changed(status);
    }
}

impl TelegramAuthClient for TelegramAdapter {
    fn request_login_code(&mut self, phone: &str) -> Result<AuthCodeToken, AuthBackendError> {
        self.status_tracker.on_auth_start();

        let result = match self.auth_backend.as_mut() {
            Some(backend) => backend.request_login_code(phone),
            None => Err(AuthBackendError::Transient {
                code: "AUTH_BACKEND_UNAVAILABLE",
                message: "Telegram auth backend is not configured".into(),
            }),
        };

        if let Err(error) = &result {
            self.status_tracker.on_auth_error(error);
        }

        result
    }

    fn sign_in_with_code(
        &mut self,
        token: &AuthCodeToken,
        code: &str,
    ) -> Result<SignInOutcome, AuthBackendError> {
        let result = match self.auth_backend.as_mut() {
            Some(backend) => backend.sign_in_with_code(token, code),
            None => Err(AuthBackendError::Transient {
                code: "AUTH_BACKEND_UNAVAILABLE",
                message: "Telegram auth backend is not configured".into(),
            }),
        };

        match &result {
            Ok(SignInOutcome::Authorized) => self.status_tracker.on_auth_success(),
            Ok(SignInOutcome::PasswordRequired) => self.status_tracker.on_auth_password_required(),
            Err(error) => self.status_tracker.on_auth_error(error),
        }

        result
    }

    fn verify_password(&mut self, password: &str) -> Result<(), AuthBackendError> {
        let result = match self.auth_backend.as_mut() {
            Some(backend) => backend.verify_password(password),
            None => Err(AuthBackendError::Transient {
                code: "AUTH_BACKEND_UNAVAILABLE",
                message: "Telegram auth backend is not configured".into(),
            }),
        };

        match &result {
            Ok(()) => self.status_tracker.on_auth_success(),
            Err(error) => self.status_tracker.on_auth_error(error),
        }

        result
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

    #[test]
    fn status_stream_emits_initial_payload_contract() {
        let adapter = TelegramAdapter::stub();
        let rx = adapter.subscribe_status();

        let initial = rx.recv().expect("initial status snapshot");
        assert_eq!(initial.auth.as_label(), "AUTH_NOT_STARTED");
        assert_eq!(initial.connectivity.as_label(), "CONNECTIVITY_UNKNOWN");
        assert_eq!(initial.last_error, None);
        assert!(initial.updated_at_unix_ms > 0);
    }

    #[test]
    fn status_stream_tracks_auth_error_and_connectivity_transition() {
        let mut adapter = TelegramAdapter::stub();
        let rx = adapter.subscribe_status();
        let _ = rx.recv().expect("initial status snapshot");

        let result = adapter.request_login_code("+15551234567");
        assert!(result.is_err());

        adapter.record_connectivity_status(ConnectivityStatus::Connecting);

        let mut snapshots = Vec::new();
        while let Ok(snapshot) = rx.try_recv() {
            snapshots.push(snapshot);
        }

        assert!(snapshots
            .iter()
            .any(|item| item.auth.as_label() == "AUTH_IN_PROGRESS"));
        assert!(snapshots.iter().any(|item| {
            item.auth.as_label() == "AUTH_FATAL_FAILURE"
                && item.last_error.as_ref().map(|err| err.code.as_str())
                    == Some("AUTH_BACKEND_UNAVAILABLE")
        }));
        assert!(snapshots
            .iter()
            .any(|item| item.connectivity.as_label() == "CONNECTIVITY_DEGRADED"));
    }
}
