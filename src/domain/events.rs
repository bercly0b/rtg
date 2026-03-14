#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvent {
    Tick,
    QuitRequested,
    InputKey(KeyInput),
    ConnectivityChanged(ConnectivityStatus),
    ChatListUpdateRequested,
    BackgroundTaskCompleted(BackgroundTaskResult),
}

/// Result of an asynchronous background operation dispatched from the UI thread.
///
/// These variants carry the outcome of Telegram API calls that were executed
/// on a background thread to avoid blocking the TUI event loop.
///
/// Uses domain-level types only; error details are represented as simple strings
/// to keep the domain layer independent of the usecases layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackgroundTaskResult {
    /// Chat list fetch completed.
    ChatListLoaded {
        result: Result<Vec<super::chat::ChatSummary>, BackgroundError>,
    },
    /// Messages fetch for a specific chat completed.
    MessagesLoaded {
        chat_id: i64,
        result: Result<Vec<super::message::Message>, BackgroundError>,
    },
    /// Message send operation completed; `chat_id` identifies the target chat,
    /// `original_text` is kept for re-population on failure.
    MessageSent {
        chat_id: i64,
        original_text: String,
        result: Result<(), BackgroundError>,
    },
    /// Messages refresh after a successful send completed.
    MessageSentRefreshCompleted {
        chat_id: i64,
        result: Result<Vec<super::message::Message>, BackgroundError>,
    },
}

/// Lightweight error type for background task failures.
///
/// Kept simple to avoid domain→usecases dependency; the orchestrator maps
/// usecase-level errors into this type when dispatching background work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundError {
    pub code: &'static str,
}

impl BackgroundError {
    pub fn new(code: &'static str) -> Self {
        Self { code }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectivityStatus {
    Connected,
    Connecting,
    Disconnected,
}

impl ConnectivityStatus {
    pub fn as_label(self) -> &'static str {
        match self {
            Self::Connected => "connected",
            Self::Connecting => "connecting",
            Self::Disconnected => "disconnected",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyInput {
    pub key: String,
    pub ctrl: bool,
}

impl KeyInput {
    pub fn new(key: impl Into<String>, ctrl: bool) -> Self {
        Self {
            key: key.into(),
            ctrl,
        }
    }
}
