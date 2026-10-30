use crate::domain::{
    chat::ChatSummary, chat_folder::ChatListKind, chat_info_state::ChatInfo,
    chat_subtitle::ChatSubtitle, forum_topic::ForumTopicSummary, message::Message,
    message_info_state::MessageInfo, reaction_picker_state::AvailableReaction,
};

/// Result of an asynchronous background operation dispatched from the UI thread.
///
/// These variants carry the outcome of Telegram API calls that were executed
/// on a background thread to avoid blocking the TUI event loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackgroundTaskResult {
    /// Chat list fetch completed. `list` mirrors the request scope so the
    /// handler can drop results for a list the user has switched away from.
    ChatListLoaded {
        list: ChatListKind,
        result: Result<Vec<ChatSummary>, BackgroundError>,
        all_loaded: bool,
    },
    /// Forum topic list fetch completed for a specific forum chat.
    ForumTopicsLoaded {
        chat_id: i64,
        result: Result<Vec<ForumTopicSummary>, BackgroundError>,
    },
    /// Unread-topic counts resolved for forum chats whose chat-list badge was
    /// unknown. Chats whose fetch failed are absent; the next chat-list load
    /// re-dispatches the warm-up for them.
    ForumUnreadCountsLoaded { counts: Vec<(i64, u32)> },
    /// Messages fetch for a specific chat completed. `topic_id` mirrors the
    /// request scope so handlers can drop results from a topic the user is
    /// no longer viewing.
    MessagesLoaded {
        chat_id: i64,
        topic_id: Option<i32>,
        result: Result<Vec<Message>, BackgroundError>,
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
        topic_id: Option<i32>,
        result: Result<Vec<Message>, BackgroundError>,
    },
    /// Background prefetch of messages for a chat the user is hovering.
    /// Results go only into `MessageCache`, not `OpenChatState`.
    MessagesPrefetched {
        chat_id: i64,
        result: Result<Vec<Message>, BackgroundError>,
    },
    /// Older messages loaded for scroll-up pagination.
    OlderMessagesLoaded {
        chat_id: i64,
        topic_id: Option<i32>,
        result: Result<Vec<Message>, BackgroundError>,
    },
    /// Message edit operation completed.
    MessageEdited {
        chat_id: i64,
        message_id: i64,
        original_text: String,
        result: Result<(), BackgroundError>,
    },
    /// Voice note send failed; the optimistic pending message should be removed.
    VoiceSendFailed { chat_id: i64 },
    /// Chat subtitle (user status / member count) resolved.
    ChatSubtitleLoaded {
        chat_id: i64,
        result: Result<ChatSubtitle, BackgroundError>,
    },
    /// Chat info (title, status, description) resolved for the info popup.
    ChatInfoLoaded {
        chat_id: i64,
        result: Result<ChatInfo, BackgroundError>,
    },
    /// Message info (reactions, viewers, read/edit dates) resolved for the info popup.
    MessageInfoLoaded {
        chat_id: i64,
        message_id: i64,
        result: Result<MessageInfo, BackgroundError>,
    },
    /// Available reactions loaded for the reaction picker popup.
    AvailableReactionsLoaded {
        chat_id: i64,
        message_id: i64,
        result: Result<Vec<AvailableReaction>, BackgroundError>,
    },
    /// Platform default file opener failed (non-zero exit code).
    /// The orchestrator should display the error as a notification.
    OpenFileFailed { stderr: String },
    /// File was successfully saved to the downloads directory.
    FileSaved { file_id: i32, file_name: String },
    /// File save to downloads directory failed.
    FileSaveFailed { file_id: i32, error: String },
}

/// Lightweight error type for background task failures.
///
/// Carries only a non-secret error code: the dispatcher maps usecase-level
/// errors into it, so results can be cloned and compared freely.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{code}")]
pub struct BackgroundError {
    pub code: &'static str,
}

impl BackgroundError {
    pub fn new(code: &'static str) -> Self {
        Self { code }
    }
}
