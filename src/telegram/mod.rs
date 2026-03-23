//! Telegram integration layer: API clients and event mapping.

mod chat_updates;
mod connectivity;
mod message_pagination;
mod status_tracker;
mod tdlib_auth;
mod tdlib_client;
mod tdlib_mappers;
mod tdlib_updates;

// Re-export TDLib types for external use
#[allow(unused_imports)]
pub use tdlib_client::{TdLibClient, TdLibConfig, TdLibError};
#[allow(unused_imports)]
pub use tdlib_updates::TdLibUpdate;

use std::sync::mpsc::{Receiver, Sender};

use status_tracker::StatusTracker;
use tdlib_auth::TdLibAuthBackend;

pub use chat_updates::{ChatUpdatesMonitorStartError, TelegramChatUpdatesMonitor};
pub use connectivity::{ConnectivityMonitorStartError, TelegramConnectivityMonitor};

use crate::{
    domain::{events::ConnectivityStatus, message::Message, status::AuthConnectivityStatus},
    infra::{config::TelegramConfig, storage_layout::StorageLayout},
    usecases::{
        chat_lifecycle::{ChatLifecycle, ChatLifecycleError, ChatReadMarker, MessageDeleter},
        chat_subtitle::{ChatSubtitleError, ChatSubtitleQuery, ChatSubtitleSource},
        guided_auth::{AuthBackendError, AuthCodeToken, SignInOutcome, TelegramAuthClient},
        list_chats::{CachedChatsSource, ListChatsSource, ListChatsSourceError},
        load_messages::{CachedMessagesSource, MessagesSource, MessagesSourceError},
        send_message::{MessageSender, SendMessageSourceError},
        send_voice::VoiceNoteSender,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum BackendKind {
    Stub,
    /// Stub that reports as authorized (for testing startup flow).
    #[cfg(test)]
    StubAuthorized,
    TdLib,
}

pub struct TelegramAdapter {
    backend_kind: BackendKind,
    tdlib_backend: Option<TdLibAuthBackend>,
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
            tdlib_backend: None,
            status_tracker: StatusTracker::new(),
        }
    }

    /// Creates a stub adapter that reports as authorized.
    ///
    /// Used in tests to verify the `LaunchTui` startup path without
    /// requiring a real TDLib connection.
    #[cfg(test)]
    pub fn stub_authorized() -> Self {
        Self {
            backend_kind: BackendKind::StubAuthorized,
            tdlib_backend: None,
            status_tracker: StatusTracker::new(),
        }
    }

    pub fn from_config(config: &TelegramConfig) -> Result<Self, AuthBackendError> {
        if !config.is_configured() {
            return Ok(Self::stub());
        }

        let layout = StorageLayout::resolve().map_err(|error| AuthBackendError::Transient {
            code: "AUTH_SESSION_STORE_UNAVAILABLE",
            message: format!("failed to resolve storage layout: {error}"),
        })?;

        let backend = TdLibAuthBackend::new(config, &layout)?;
        Ok(Self {
            backend_kind: BackendKind::TdLib,
            tdlib_backend: Some(backend),
            status_tracker: StatusTracker::new(),
        })
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn uses_real_backend(&self) -> bool {
        matches!(self.backend_kind, BackendKind::TdLib)
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

    pub fn start_chat_updates_monitor(
        &self,
        updates_tx: Sender<crate::domain::events::ChatUpdate>,
    ) -> Result<TelegramChatUpdatesMonitor, ChatUpdatesMonitorStartError> {
        let backend = self
            .tdlib_backend
            .as_ref()
            .ok_or(ChatUpdatesMonitorStartError::StartupRejected)?;

        let update_rx = backend
            .take_update_receiver()
            .ok_or(ChatUpdatesMonitorStartError::StartupRejected)?;

        let mapper = backend.create_message_mapper();

        TelegramChatUpdatesMonitor::start(update_rx, updates_tx, mapper)
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

    /// Checks whether the TDLib client is fully authorized (session is valid).
    ///
    /// Returns `false` for stub backends or when TDLib is not yet authorized
    /// (e.g. waiting for phone number, code, or password).
    pub fn is_authorized(&mut self) -> Result<bool, AuthBackendError> {
        #[cfg(test)]
        if matches!(self.backend_kind, BackendKind::StubAuthorized) {
            return Ok(true);
        }

        match self.tdlib_backend.as_mut() {
            Some(backend) => backend.is_authorized(),
            None => Ok(false),
        }
    }

    pub fn disconnect_and_reset(&mut self) {
        if let Some(backend) = self.tdlib_backend.as_mut() {
            backend.disconnect_and_reset();
        }
        self.status_tracker.on_logout_reset();
    }
}

impl TelegramAuthClient for TelegramAdapter {
    fn auth_status_snapshot(&self) -> Option<AuthConnectivityStatus> {
        Some(self.status_snapshot())
    }

    fn request_login_code(&mut self, phone: &str) -> Result<AuthCodeToken, AuthBackendError> {
        self.status_tracker.on_auth_start();

        let result = match self.tdlib_backend.as_mut() {
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
        let result = match self.tdlib_backend.as_mut() {
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
        let result = match self.tdlib_backend.as_mut() {
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
}

impl ListChatsSource for TelegramAdapter {
    fn list_chats(
        &self,
        limit: usize,
    ) -> Result<Vec<crate::domain::chat::ChatSummary>, ListChatsSourceError> {
        match self.tdlib_backend.as_ref() {
            Some(backend) => backend.list_chat_summaries(limit),
            None => Err(ListChatsSourceError::Unavailable),
        }
    }
}

impl CachedChatsSource for TelegramAdapter {
    fn list_cached_chats(
        &self,
        limit: usize,
    ) -> Result<Vec<crate::domain::chat::ChatSummary>, ListChatsSourceError> {
        match self.tdlib_backend.as_ref() {
            Some(backend) => backend.list_cached_chat_summaries(limit),
            None => Ok(Vec::new()),
        }
    }
}

impl MessagesSource for TelegramAdapter {
    fn list_messages(
        &self,
        chat_id: i64,
        limit: usize,
    ) -> Result<Vec<Message>, MessagesSourceError> {
        match self.tdlib_backend.as_ref() {
            Some(backend) => backend.list_messages(chat_id, limit),
            None => Err(MessagesSourceError::Unavailable),
        }
    }
}

impl CachedMessagesSource for TelegramAdapter {
    fn list_cached_messages(
        &self,
        chat_id: i64,
        limit: usize,
    ) -> Result<Vec<Message>, MessagesSourceError> {
        match self.tdlib_backend.as_ref() {
            Some(backend) => backend.list_cached_messages(chat_id, limit),
            None => Ok(Vec::new()),
        }
    }
}

impl MessageSender for TelegramAdapter {
    fn send_message(&self, chat_id: i64, text: &str) -> Result<(), SendMessageSourceError> {
        match self.tdlib_backend.as_ref() {
            Some(backend) => backend.send_message(chat_id, text),
            None => Err(SendMessageSourceError::Unauthorized),
        }
    }
}

impl VoiceNoteSender for TelegramAdapter {
    fn send_voice_note(
        &self,
        chat_id: i64,
        file_path: &str,
        duration: i32,
        waveform: &str,
    ) -> Result<(), SendMessageSourceError> {
        match self.tdlib_backend.as_ref() {
            Some(backend) => backend.send_voice_note(chat_id, file_path, duration, waveform),
            None => Err(SendMessageSourceError::Unauthorized),
        }
    }
}

impl ChatLifecycle for TelegramAdapter {
    fn open_chat(&self, chat_id: i64) -> Result<(), ChatLifecycleError> {
        match self.tdlib_backend.as_ref() {
            Some(backend) => backend.open_chat(chat_id).map_err(|e| {
                tracing::debug!(chat_id, error = ?e, "open_chat mapped to lifecycle error");
                ChatLifecycleError::Unavailable
            }),
            None => Err(ChatLifecycleError::Unavailable),
        }
    }

    fn close_chat(&self, chat_id: i64) -> Result<(), ChatLifecycleError> {
        match self.tdlib_backend.as_ref() {
            Some(backend) => backend.close_chat(chat_id).map_err(|e| {
                tracing::debug!(chat_id, error = ?e, "close_chat mapped to lifecycle error");
                ChatLifecycleError::Unavailable
            }),
            None => Err(ChatLifecycleError::Unavailable),
        }
    }
}

impl ChatReadMarker for TelegramAdapter {
    fn mark_messages_read(
        &self,
        chat_id: i64,
        message_ids: Vec<i64>,
    ) -> Result<(), ChatLifecycleError> {
        match self.tdlib_backend.as_ref() {
            Some(backend) => backend.view_messages(chat_id, message_ids).map_err(|e| {
                tracing::debug!(chat_id, error = ?e, "view_messages mapped to lifecycle error");
                ChatLifecycleError::Unavailable
            }),
            None => Err(ChatLifecycleError::Unavailable),
        }
    }
}

impl MessageDeleter for TelegramAdapter {
    fn delete_messages(
        &self,
        chat_id: i64,
        message_ids: Vec<i64>,
        revoke: bool,
    ) -> Result<(), ChatLifecycleError> {
        match self.tdlib_backend.as_ref() {
            Some(backend) => backend.delete_messages(chat_id, message_ids, revoke).map_err(|e| {
                tracing::debug!(chat_id, error = ?e, "delete_messages mapped to lifecycle error");
                ChatLifecycleError::Unavailable
            }),
            None => Err(ChatLifecycleError::Unavailable),
        }
    }
}

impl ChatSubtitleSource for TelegramAdapter {
    fn resolve_chat_subtitle(
        &self,
        query: &ChatSubtitleQuery,
    ) -> Result<crate::domain::chat_subtitle::ChatSubtitle, ChatSubtitleError> {
        match self.tdlib_backend.as_ref() {
            Some(backend) => Ok(backend.resolve_subtitle(query.chat_id)),
            None => Err(ChatSubtitleError::Unavailable),
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

    #[test]
    fn disconnect_and_reset_sets_disconnected_snapshot() {
        let mut adapter = TelegramAdapter::stub();
        adapter.record_connectivity_status(ConnectivityStatus::Connected);

        adapter.disconnect_and_reset();

        let snapshot = adapter.status_snapshot();
        assert_eq!(snapshot.auth.as_label(), "AUTH_NOT_STARTED");
        assert_eq!(snapshot.connectivity.as_label(), "CONNECTIVITY_UNAVAILABLE");
        assert_eq!(snapshot.last_error, None);
    }

    #[test]
    fn list_chats_returns_unavailable_when_backend_is_not_configured() {
        let adapter = TelegramAdapter::stub();

        let error = adapter
            .list_chats(20)
            .expect_err("stub adapter should fail");

        assert_eq!(error, ListChatsSourceError::Unavailable);
    }

    #[test]
    fn list_messages_returns_unavailable_when_backend_is_not_configured() {
        let adapter = TelegramAdapter::stub();

        let error = adapter
            .list_messages(1, 20)
            .expect_err("stub adapter should fail");

        assert_eq!(error, MessagesSourceError::Unavailable);
    }

    #[test]
    fn send_message_returns_unauthorized_when_backend_is_not_configured() {
        let adapter = TelegramAdapter::stub();

        let error = adapter
            .send_message(1, "hello")
            .expect_err("stub adapter should fail");

        assert_eq!(error, SendMessageSourceError::Unauthorized);
    }

    #[test]
    fn open_chat_returns_unavailable_when_backend_is_not_configured() {
        let adapter = TelegramAdapter::stub();

        let error = adapter.open_chat(1).expect_err("stub adapter should fail");

        assert_eq!(error, ChatLifecycleError::Unavailable);
    }

    #[test]
    fn close_chat_returns_unavailable_when_backend_is_not_configured() {
        let adapter = TelegramAdapter::stub();

        let error = adapter.close_chat(1).expect_err("stub adapter should fail");

        assert_eq!(error, ChatLifecycleError::Unavailable);
    }

    #[test]
    fn mark_messages_read_returns_unavailable_when_backend_is_not_configured() {
        let adapter = TelegramAdapter::stub();

        let error = adapter
            .mark_messages_read(1, vec![1, 2, 3])
            .expect_err("stub adapter should fail");

        assert_eq!(error, ChatLifecycleError::Unavailable);
    }

    #[test]
    fn delete_messages_returns_unavailable_when_backend_is_not_configured() {
        let adapter = TelegramAdapter::stub();

        let error = adapter
            .delete_messages(1, vec![1, 2], true)
            .expect_err("stub adapter should fail");

        assert_eq!(error, ChatLifecycleError::Unavailable);
    }
}
