use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthStatus {
    NotStarted,
    InProgress,
    Requires2fa,
    Success,
    TransientFailure,
    FatalFailure,
}

impl AuthStatus {
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn as_label(self) -> &'static str {
        match self {
            Self::NotStarted => "AUTH_NOT_STARTED",
            Self::InProgress => "AUTH_IN_PROGRESS",
            Self::Requires2fa => "AUTH_REQUIRES_2FA",
            Self::Success => "AUTH_SUCCESS",
            Self::TransientFailure => "AUTH_TRANSIENT_FAILURE",
            Self::FatalFailure => "AUTH_FATAL_FAILURE",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectivityHealth {
    Unknown,
    Ok,
    Degraded,
    Unavailable,
}

impl ConnectivityHealth {
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn as_label(self) -> &'static str {
        match self {
            Self::Unknown => "CONNECTIVITY_UNKNOWN",
            Self::Ok => "CONNECTIVITY_OK",
            Self::Degraded => "CONNECTIVITY_DEGRADED",
            Self::Unavailable => "CONNECTIVITY_UNAVAILABLE",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusError {
    pub code: String,
    pub at_unix_ms: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthConnectivityStatus {
    pub auth: AuthStatus,
    pub connectivity: ConnectivityHealth,
    pub updated_at_unix_ms: u128,
    pub last_error: Option<StatusError>,
}

impl Default for AuthConnectivityStatus {
    fn default() -> Self {
        Self {
            auth: AuthStatus::NotStarted,
            connectivity: ConnectivityHealth::Unknown,
            updated_at_unix_ms: now_unix_ms(),
            last_error: None,
        }
    }
}

pub fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
