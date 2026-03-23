#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvent {
    Tick,
    QuitRequested,
    InputKey(KeyInput),
    ConnectivityChanged(ConnectivityStatus),
    ChatUpdateReceived {
        updates: Vec<ChatUpdate>,
    },
    BackgroundTaskCompleted(BackgroundTaskResult),
    /// A line of output from a running external command (e.g. ffmpeg).
    ///
    /// `replace_last=true` means the line came from a carriage-return update
    /// and should replace the previously displayed line (progress-style output).
    CommandOutputLine {
        text: String,
        replace_last: bool,
    },
    /// The external command process has exited.
    CommandExited {
        success: bool,
    },
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
    /// Background prefetch of messages for a chat the user is hovering.
    /// Results go only into `MessageCache`, not `OpenChatState`.
    MessagesPrefetched {
        chat_id: i64,
        result: Result<Vec<super::message::Message>, BackgroundError>,
    },
    /// Voice note send failed; the optimistic pending message should be removed.
    VoiceSendFailed { chat_id: i64 },
    /// Chat subtitle (user status / member count) resolved.
    ChatSubtitleLoaded {
        chat_id: i64,
        result: Result<super::chat_subtitle::ChatSubtitle, BackgroundError>,
    },
    /// Chat info (title, status, description) resolved for the info popup.
    ChatInfoLoaded {
        chat_id: i64,
        result: Result<super::chat_info_state::ChatInfo, BackgroundError>,
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

/// A granular update from Telegram about a specific chat.
///
/// Produced by the chat updates monitor from TDLib push updates.
/// Carries enough data for the orchestrator to warm the message cache
/// without dispatching additional TDLib calls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatUpdate {
    /// A new message arrived in a chat.
    NewMessage {
        chat_id: i64,
        message: super::message::Message,
    },
    /// Messages were deleted from a chat.
    MessagesDeleted { chat_id: i64, message_ids: Vec<i64> },
    /// Chat metadata changed (last message, position, read state, etc.).
    /// The orchestrator should refresh the chat list.
    ChatMetadataChanged { chat_id: i64 },
    /// File download progress or completion update from TDLib.
    FileUpdated {
        file_id: i32,
        size: u64,
        local_path: String,
        is_downloading_active: bool,
        is_downloading_completed: bool,
        downloaded_size: u64,
    },
}

impl ChatUpdate {
    /// Returns the chat_id for chat-scoped updates, or `None` for file updates
    /// (which are cross-chat and resolved via the downloads tracker).
    pub fn chat_id(&self) -> Option<i64> {
        match self {
            ChatUpdate::NewMessage { chat_id, .. }
            | ChatUpdate::MessagesDeleted { chat_id, .. }
            | ChatUpdate::ChatMetadataChanged { chat_id } => Some(*chat_id),
            ChatUpdate::FileUpdated { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectivityStatus {
    Connected,
    Connecting,
    Disconnected,
}

impl ConnectivityStatus {
    #[cfg_attr(not(test), allow(dead_code))]
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

/// Events produced by a running external command (e.g. ffmpeg).
///
/// Defined in the domain layer so both usecases and ui can reference it
/// without creating circular dependencies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandEvent {
    /// A line of combined stdout/stderr output.
    ///
    /// `replace_last=true` means the line terminated with `\r` and should
    /// replace the previously displayed line.
    OutputLine { text: String, replace_last: bool },
    /// The process has exited.
    Exited { success: bool },
}
