//! Chat subtitle and chat info resolution.
//!
//! Subtitle: status line shown below the chat title in the messages panel.
//! Chat info: detailed information shown in the chat info popup (I key).

use crate::domain::chat::ChatType;
use crate::domain::chat_info_state::ChatInfo;
use crate::domain::chat_subtitle::ChatSubtitle;

/// Query for resolving a chat's subtitle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatSubtitleQuery {
    pub chat_id: i64,
    pub chat_type: ChatType,
}

/// Query for resolving full chat info (for the info popup).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatInfoQuery {
    pub chat_id: i64,
    pub chat_type: ChatType,
    pub title: String,
}

/// Error when subtitle or chat info resolution fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatSubtitleError {
    Unavailable,
}

/// Source that resolves chat subtitles and chat info from the Telegram backend.
pub trait ChatSubtitleSource: Send + Sync {
    fn resolve_chat_subtitle(
        &self,
        query: &ChatSubtitleQuery,
    ) -> Result<ChatSubtitle, ChatSubtitleError>;

    fn resolve_chat_info(&self, query: &ChatInfoQuery) -> Result<ChatInfo, ChatSubtitleError>;
}

// Blanket impl for Arc<T> to match the pattern used by other sources.
impl<T: ChatSubtitleSource> ChatSubtitleSource for std::sync::Arc<T> {
    fn resolve_chat_subtitle(
        &self,
        query: &ChatSubtitleQuery,
    ) -> Result<ChatSubtitle, ChatSubtitleError> {
        (**self).resolve_chat_subtitle(query)
    }

    fn resolve_chat_info(&self, query: &ChatInfoQuery) -> Result<ChatInfo, ChatSubtitleError> {
        (**self).resolve_chat_info(query)
    }
}
