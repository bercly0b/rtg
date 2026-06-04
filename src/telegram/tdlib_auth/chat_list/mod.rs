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
    fn get_forum_topics(
        &self,
        chat_id: i64,
        limit: i32,
    ) -> Result<Vec<tdlib_rs::types::ForumTopic>, TdLibError>;
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

    fn get_forum_topics(
        &self,
        chat_id: i64,
        limit: i32,
    ) -> Result<Vec<tdlib_rs::types::ForumTopic>, TdLibError> {
        self.get_forum_topics(chat_id, limit)
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

        let (sender_name, is_online, is_bot, is_forum) =
            resolve_chat_metadata(resolver, &chat, cache);
        let mut summary = crate::telegram::tdlib_mappers::map_chat_to_summary(
            &chat,
            sender_name,
            is_online,
            is_bot,
            is_forum,
        );
        // TDLib's chat-level `unread_count` is unreliable for forums, so the
        // badge instead shows the number of topics with unread messages. Only
        // resolved when the chat reports unread (cheap gate) to avoid an extra
        // server round-trip for fully-read forums.
        if is_forum && summary.unread_count > 0 {
            summary.unread_topic_count = Some(count_unread_forum_topics(resolver, chat_id));
        }
        summaries.push(summary);
    }

    summaries
}

/// `getForumTopics` page size used for the unread-topic count. TDLib caps this
/// at 100 and may return fewer; unread topics sort to the top by `order`, so a
/// single page covers the realistic unread set.
const FORUM_TOPICS_PAGE_LIMIT: i32 = 100;

/// Counts forum topics with unread messages for the chat-list badge.
///
/// On fetch failure returns 0 (badge hidden) rather than falling back to the
/// misleading chat-level message count — the next chat-list refresh retries.
fn count_unread_forum_topics(resolver: &dyn ChatDataResolver, chat_id: i64) -> u32 {
    match resolver.get_forum_topics(chat_id, FORUM_TOPICS_PAGE_LIMIT) {
        Ok(topics) => topics.iter().filter(|t| t.unread_count > 0).count() as u32,
        Err(e) => {
            tracing::warn!(chat_id, error = %e, "failed to fetch forum topics for unread count");
            0
        }
    }
}

/// Resolves additional metadata for a chat (sender name, online status, forum flag).
///
/// Uses the cache for user lookups. Falls back to `get_user` on miss.
/// `is_forum` is read from the cached `Supergroup` — TDLib guarantees that
/// `updateSupergroup` arrives before the supergroup ID surfaces in any
/// response, so for supergroup chats the cache lookup is race-free. Missing
/// supergroup data defaults to `false`.
fn resolve_chat_metadata(
    resolver: &dyn ChatDataResolver,
    chat: &Chat,
    cache: &TdLibCache,
) -> (Option<String>, Option<bool>, bool, bool) {
    use crate::domain::chat::ChatType;
    use crate::telegram::tdlib_mappers;

    let chat_type = tdlib_mappers::map_chat_type(&chat.r#type);

    let is_forum = match &chat.r#type {
        tdlib_rs::enums::ChatType::Supergroup(sg) => cache
            .get_supergroup(sg.supergroup_id)
            .map(|s| s.is_forum)
            .unwrap_or(false),
        _ => false,
    };

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

    (sender_name, is_online, is_bot, is_forum)
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
