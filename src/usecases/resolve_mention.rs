use crate::domain::chat::ChatSummary;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MentionSourceError {
    Unavailable,
}

/// Finds the chat a mention points at.
pub trait MentionSource {
    /// Resolves `username` (without the leading `@`); `Ok(None)` when nobody
    /// holds it.
    fn resolve_mention(&self, username: &str) -> Result<Option<ChatSummary>, MentionSourceError>;
}

impl<T> MentionSource for std::sync::Arc<T>
where
    T: MentionSource + ?Sized,
{
    fn resolve_mention(&self, username: &str) -> Result<Option<ChatSummary>, MentionSourceError> {
        (**self).resolve_mention(username)
    }
}
