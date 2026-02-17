use std::sync::{mpsc, Arc, Mutex};

use crate::{
    domain::{
        events::ConnectivityStatus,
        status::{
            now_unix_ms, AuthConnectivityStatus, AuthStatus, ConnectivityHealth, StatusError,
        },
    },
    infra::secrets::sanitize_error_code,
    usecases::guided_auth::AuthBackendError,
};

#[derive(Clone, Debug)]
pub struct StatusTracker {
    inner: Arc<Mutex<StatusTrackerState>>,
}

#[derive(Debug, Default)]
struct StatusTrackerState {
    snapshot: AuthConnectivityStatus,
    subscribers: Vec<mpsc::Sender<AuthConnectivityStatus>>,
}

impl StatusTracker {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(StatusTrackerState::default())),
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn subscribe(&self) -> mpsc::Receiver<AuthConnectivityStatus> {
        let (tx, rx) = mpsc::channel();
        if let Ok(mut state) = self.inner.lock() {
            let _ = tx.send(state.snapshot.clone());
            state.subscribers.push(tx);
        }
        rx
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn snapshot(&self) -> AuthConnectivityStatus {
        self.inner
            .lock()
            .map(|state| state.snapshot.clone())
            .unwrap_or_default()
    }

    pub fn on_connectivity_changed(&self, status: ConnectivityStatus) {
        self.mutate(|snapshot| {
            snapshot.connectivity = map_connectivity(status);
        });
    }

    pub fn on_auth_start(&self) {
        self.mutate(|snapshot| {
            snapshot.auth = AuthStatus::InProgress;
            snapshot.last_error = None;
        });
    }

    pub fn on_auth_password_required(&self) {
        self.mutate(|snapshot| {
            snapshot.auth = AuthStatus::Requires2fa;
            snapshot.last_error = None;
        });
    }

    pub fn on_auth_success(&self) {
        self.mutate(|snapshot| {
            snapshot.auth = AuthStatus::Success;
            snapshot.last_error = None;
        });
    }

    pub fn on_auth_error(&self, error: &AuthBackendError) {
        self.mutate(|snapshot| {
            snapshot.auth = map_auth_error(error);
            snapshot.last_error = Some(StatusError {
                code: auth_error_code(error).to_owned(),
                at_unix_ms: now_unix_ms(),
            });
        });
    }

    pub fn on_logout_reset(&self) {
        self.mutate(|snapshot| {
            snapshot.auth = AuthStatus::NotStarted;
            snapshot.connectivity = ConnectivityHealth::Unavailable;
            snapshot.last_error = None;
        });
    }

    fn mutate<F>(&self, mutator: F)
    where
        F: FnOnce(&mut AuthConnectivityStatus),
    {
        if let Ok(mut state) = self.inner.lock() {
            mutator(&mut state.snapshot);
            state.snapshot.updated_at_unix_ms = now_unix_ms();
            let payload = state.snapshot.clone();
            state
                .subscribers
                .retain(|sub| sub.send(payload.clone()).is_ok());
        }
    }
}

fn map_connectivity(status: ConnectivityStatus) -> ConnectivityHealth {
    match status {
        ConnectivityStatus::Connected => ConnectivityHealth::Ok,
        ConnectivityStatus::Connecting => ConnectivityHealth::Degraded,
        ConnectivityStatus::Disconnected => ConnectivityHealth::Unavailable,
    }
}

fn map_auth_error(error: &AuthBackendError) -> AuthStatus {
    if matches!(
        error,
        AuthBackendError::Transient {
            code: "AUTH_BACKEND_UNAVAILABLE",
            ..
        }
    ) {
        return AuthStatus::FatalFailure;
    }

    AuthStatus::TransientFailure
}

fn auth_error_code(error: &AuthBackendError) -> String {
    match error {
        AuthBackendError::InvalidPhone => "AUTH_INVALID_PHONE".to_owned(),
        AuthBackendError::InvalidCode => "AUTH_INVALID_CODE".to_owned(),
        AuthBackendError::WrongPassword => "AUTH_WRONG_2FA".to_owned(),
        AuthBackendError::Timeout => "AUTH_TIMEOUT".to_owned(),
        AuthBackendError::FloodWait { .. } => "AUTH_FLOOD_WAIT".to_owned(),
        AuthBackendError::Transient { code, .. } => sanitize_error_code(code),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_initial_snapshot_on_subscribe() {
        let tracker = StatusTracker::new();
        let rx = tracker.subscribe();
        let initial = rx.recv().expect("initial snapshot should be sent");

        assert_eq!(initial.auth, AuthStatus::NotStarted);
        assert_eq!(initial.connectivity, ConnectivityHealth::Unknown);
        assert_eq!(initial.last_error, None);
    }

    #[test]
    fn transitions_auth_and_error_contract() {
        let tracker = StatusTracker::new();
        tracker.on_auth_start();
        tracker.on_auth_error(&AuthBackendError::InvalidCode);

        let snapshot = tracker.snapshot();
        assert_eq!(snapshot.auth, AuthStatus::TransientFailure);
        assert_eq!(
            snapshot.last_error.as_ref().map(|e| e.code.as_str()),
            Some("AUTH_INVALID_CODE")
        );
        assert!(snapshot.updated_at_unix_ms > 0);
    }

    #[test]
    fn maps_connectivity_statuses_to_canonical_health() {
        let tracker = StatusTracker::new();
        tracker.on_connectivity_changed(ConnectivityStatus::Connecting);
        assert_eq!(
            tracker.snapshot().connectivity,
            ConnectivityHealth::Degraded
        );

        tracker.on_connectivity_changed(ConnectivityStatus::Connected);
        assert_eq!(tracker.snapshot().connectivity, ConnectivityHealth::Ok);

        tracker.on_connectivity_changed(ConnectivityStatus::Disconnected);
        assert_eq!(
            tracker.snapshot().connectivity,
            ConnectivityHealth::Unavailable
        );
    }

    #[test]
    fn successful_auth_clears_last_error() {
        let tracker = StatusTracker::new();
        tracker.on_auth_error(&AuthBackendError::WrongPassword);
        assert!(tracker.snapshot().last_error.is_some());

        tracker.on_auth_success();
        let snapshot = tracker.snapshot();
        assert_eq!(snapshot.auth, AuthStatus::Success);
        assert!(snapshot.last_error.is_none());
    }

    #[test]
    fn logout_reset_forces_disconnected_clean_state() {
        let tracker = StatusTracker::new();
        tracker.on_auth_success();
        tracker.on_connectivity_changed(ConnectivityStatus::Connected);

        tracker.on_logout_reset();

        let snapshot = tracker.snapshot();
        assert_eq!(snapshot.auth, AuthStatus::NotStarted);
        assert_eq!(snapshot.connectivity, ConnectivityHealth::Unavailable);
        assert!(snapshot.last_error.is_none());
    }
}
