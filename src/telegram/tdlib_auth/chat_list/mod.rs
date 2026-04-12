use tdlib_rs::types::Chat;

use crate::domain::chat::ChatSummary;
use crate::telegram::tdlib_cache::TdLibCache;
use crate::telegram::tdlib_client::TdLibError;
use crate::usecases::list_chats::ListChatsSourceError;

use super::error_mapping::map_list_chats_error;
use super::TdLibAuthBackend;

/// Abstraction over TDLib data access for chat list resolution.
///
/// Separates cache reads from TDLib API calls, enabling unit tests
/// with a fake implementation that never touches real TDLib.
pub(super) trait ChatDataResolver {
    fn cache(&self) -> &TdLibCache;
    fn get_chat(&self, chat_id: i64) -> Result<Chat, TdLibError>;
    fn get_user(&self, user_id: i64) -> Result<tdlib_rs::types::User, TdLibError>;
}

impl ChatDataResolver for crate::telegram::tdlib_client::TdLibClient {
    fn cache(&self) -> &TdLibCache {
        self.cache()
    }

    fn get_chat(&self, chat_id: i64) -> Result<Chat, TdLibError> {
        self.get_chat(chat_id)
    }

    fn get_user(&self, user_id: i64) -> Result<tdlib_rs::types::User, TdLibError> {
        self.get_user(user_id)
    }
}

impl TdLibAuthBackend {
    /// Lists chat summaries from TDLib.
    ///
    /// Fetches chats from the main chat list and maps them to domain `ChatSummary`.
    /// When `force` is `true`, bypasses the in-memory cache and reads every chat
    /// directly from TDLib's SQLite via `get_chat()` — guarantees fresh data at
    /// the cost of ~1-2ms per chat.
    pub fn list_chat_summaries(
        &self,
        limit: usize,
        force: bool,
    ) -> Result<(Vec<ChatSummary>, bool), ListChatsSourceError> {
        let limit_i32 = i32::try_from(limit).unwrap_or(i32::MAX);

        let result = self
            .client
            .get_chats(limit_i32)
            .map_err(map_list_chats_error)?;

        let requested_count = result.chat_ids.len();
        tracing::debug!(
            count = requested_count,
            all_loaded = result.all_loaded,
            force,
            "Fetched chat IDs from TDLib"
        );

        let summaries = build_summaries_from_ids(&self.client, result.chat_ids, force);

        if requested_count > 0 && summaries.is_empty() {
            tracing::warn!(
                requested_count,
                "all chats failed to resolve from TDLib and cache"
            );
            return Err(ListChatsSourceError::Unavailable);
        }

        if requested_count == 0 && !result.all_loaded {
            tracing::warn!("TDLib returned zero chat IDs without all-loaded signal");
            return Err(ListChatsSourceError::Unavailable);
        }

        Ok((summaries, result.all_loaded))
    }
}

/// Builds domain `ChatSummary` list from raw TDLib chat IDs.
///
/// When `force` is `false` (default), uses the update-driven cache for
/// lookups — fast in-memory reads. Falls back to `get_chat` (TDLib SQLite)
/// on cache miss and populates the cache from the result.
///
/// When `force` is `true`, bypasses the cache entirely and calls `get_chat`
/// for every chat. This guarantees fresh data from TDLib's SQLite, which is
/// always up-to-date after `loadChats()`. Used for user-initiated refreshes
/// where stale data is unacceptable.
pub(super) fn build_summaries_from_ids(
    resolver: &dyn ChatDataResolver,
    chat_ids: Vec<i64>,
    force: bool,
) -> Vec<ChatSummary> {
    let cache = resolver.cache();
    let mut summaries = Vec::with_capacity(chat_ids.len());

    for chat_id in chat_ids {
        let chat = if force {
            match resolver.get_chat(chat_id) {
                Ok(c) => {
                    cache.upsert_chat(c.clone());
                    c
                }
                Err(e) => {
                    tracing::warn!(chat_id, error = %e, "chat missing from TDLib (force refresh)");
                    continue;
                }
            }
        } else {
            match cache.get_chat(chat_id) {
                Some(c) => c,
                None => match resolver.get_chat(chat_id) {
                    Ok(c) => {
                        cache.upsert_chat(c.clone());
                        c
                    }
                    Err(e) => {
                        tracing::warn!(chat_id, error = %e, "chat missing from cache and TDLib");
                        continue;
                    }
                },
            }
        };

        let (sender_name, is_online, is_bot) = resolve_chat_metadata(resolver, &chat, cache);
        let summary = crate::telegram::tdlib_mappers::map_chat_to_summary(
            &chat,
            sender_name,
            is_online,
            is_bot,
        );
        summaries.push(summary);
    }

    summaries
}

/// Resolves additional metadata for a chat (sender name, online status).
///
/// Uses the cache for user lookups. Falls back to `get_user` on miss.
fn resolve_chat_metadata(
    resolver: &dyn ChatDataResolver,
    chat: &Chat,
    cache: &TdLibCache,
) -> (Option<String>, Option<bool>, bool) {
    use crate::domain::chat::ChatType;
    use crate::telegram::tdlib_mappers;

    let chat_type = tdlib_mappers::map_chat_type(&chat.r#type);

    let (is_online, is_bot) = if matches!(chat_type, ChatType::Private) {
        if let Some(user_id) = tdlib_mappers::get_private_chat_user_id(&chat.r#type) {
            match resolve_user(resolver, cache, user_id) {
                Some(u) => (
                    Some(tdlib_mappers::is_user_online(&u.status)),
                    matches!(u.r#type, tdlib_rs::enums::UserType::Bot(_)),
                ),
                None => (None, false),
            }
        } else {
            (None, false)
        }
    } else {
        (None, false)
    };

    let sender_name = if matches!(chat_type, ChatType::Group | ChatType::Channel) {
        chat.last_message.as_ref().and_then(|msg| {
            let user_id = tdlib_mappers::get_sender_user_id(&msg.sender_id)?;
            let user = resolve_user(resolver, cache, user_id)?;
            Some(tdlib_mappers::format_user_name(&user))
        })
    } else {
        None
    };

    (sender_name, is_online, is_bot)
}

/// Resolves a user from cache, falling back to TDLib on miss.
///
/// Populates the cache on successful TDLib fetch. Logs a warning
/// when the user cannot be resolved from either source.
fn resolve_user(
    resolver: &dyn ChatDataResolver,
    cache: &TdLibCache,
    user_id: i64,
) -> Option<tdlib_rs::types::User> {
    if let Some(u) = cache.get_user(user_id) {
        return Some(u);
    }
    match resolver.get_user(user_id) {
        Ok(u) => {
            cache.upsert_user(u.clone());
            Some(u)
        }
        Err(e) => {
            tracing::debug!(user_id, error = %e, "user missing from cache and TDLib");
            None
        }
    }
}

#[cfg(test)]
mod tests;
