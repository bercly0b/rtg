//! Chat subtitle resolution — fetches the status line for the open chat header.

use crate::domain::chat::ChatType;
use crate::domain::chat_subtitle::ChatSubtitle;

/// Query for resolving a chat's subtitle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatSubtitleQuery {
    pub chat_id: i64,
    pub chat_type: ChatType,
}

/// Error when subtitle resolution fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatSubtitleError {
    Unavailable,
}

/// Source that resolves chat subtitles from the Telegram backend.
pub trait ChatSubtitleSource: Send + Sync {
    fn resolve_chat_subtitle(
        &self,
        query: &ChatSubtitleQuery,
    ) -> Result<ChatSubtitle, ChatSubtitleError>;
}

// Blanket impl for Arc<T> to match the pattern used by other sources.
impl<T: ChatSubtitleSource> ChatSubtitleSource for std::sync::Arc<T> {
    fn resolve_chat_subtitle(
        &self,
        query: &ChatSubtitleQuery,
    ) -> Result<ChatSubtitle, ChatSubtitleError> {
        (**self).resolve_chat_subtitle(query)
    }
}
