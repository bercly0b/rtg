/// Generic over the background task result `B`: those results belong to
/// `usecases` (see `usecases::contracts::AppEvent`), which the domain must
/// not depend on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvent<B> {
    Tick,
    QuitRequested,
    InputKey(KeyInput),
    ConnectivityChanged(ConnectivityStatus),
    ChatUpdateReceived {
        updates: Vec<ChatUpdate>,
    },
    /// All background task results that accumulated since the last event,
    /// in completion order.
    BackgroundTasksCompleted(Vec<B>),
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
        message: Box<super::message::Message>,
    },
    /// Messages were deleted from a chat.
    MessagesDeleted { chat_id: i64, message_ids: Vec<i64> },
    /// Chat metadata changed (last message, position, read state, etc.).
    /// The orchestrator should refresh the chat list.
    ChatMetadataChanged { chat_id: i64 },
    /// Reaction count changed on a specific message.
    MessageReactionsChanged {
        chat_id: i64,
        message_id: i64,
        reaction_count: u32,
    },
    /// User online/offline status changed.
    /// The orchestrator should update the open chat subtitle if it belongs to this user.
    UserStatusChanged { user_id: i64 },
    /// A user started or stopped a chat action (typing, recording, etc.).
    ChatActionChanged {
        chat_id: i64,
        sender_user_id: i64,
        sender_name: String,
        action_label: String,
        is_cancel: bool,
    },
    /// File download progress or completion update from TDLib.
    /// `local_path` is set only once the download has completed.
    FileUpdated {
        file_id: i32,
        size: u64,
        local_path: Option<String>,
        download_status: super::message::DownloadStatus,
    },
    /// A forum topic's metadata or unread state changed. The orchestrator
    /// should refresh the topic list when this matches the open forum.
    /// `unread_topic_count` carries the chat's recomputed unread-topic count
    /// for the chat-list badge (`None` when unknown — e.g. metadata-only
    /// updates or a not-yet-seeded topic cache).
    ForumTopicChanged {
        chat_id: i64,
        topic_id: i32,
        unread_topic_count: Option<u32>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectivityStatus {
    Connected,
    Connecting,
    Updating,
    Disconnected,
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
