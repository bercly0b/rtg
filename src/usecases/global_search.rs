use crate::domain::global_search_state::{ChatScope, FoundChat, FoundMessagesPage};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GlobalSearchError {
    Unavailable,
}

/// Looks through the account and Telegram itself for global search.
pub trait GlobalSearchSource {
    /// Finds the chats of `scope` that match `query`.
    fn search_chats(
        &self,
        query: &str,
        scope: ChatScope,
    ) -> Result<Vec<FoundChat>, GlobalSearchError>;

    /// Finds a page of messages that match `query` in every chat, newest
    /// first.
    /// `offset` is empty for the first page, or the `next_offset` of the
    /// page before.
    fn search_messages(
        &self,
        query: &str,
        offset: &str,
    ) -> Result<FoundMessagesPage, GlobalSearchError>;
}
