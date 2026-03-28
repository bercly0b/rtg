use crate::domain::chat::ChatSummary;
use crate::usecases::list_chats::ListChatsSourceError;

use super::error_mapping::map_list_chats_error;
use super::TdLibAuthBackend;
use crate::telegram::tdlib_mappers;

impl TdLibAuthBackend {
    /// Lists chat summaries from TDLib.
    ///
    /// Fetches chats from the main chat list and maps them to domain `ChatSummary`.
    pub fn list_chat_summaries(
        &self,
        limit: usize,
    ) -> Result<Vec<ChatSummary>, ListChatsSourceError> {
        let limit_i32 = i32::try_from(limit).unwrap_or(i32::MAX);

        let chat_ids = self
            .client
            .get_chats(limit_i32)
            .map_err(map_list_chats_error)?;

        tracing::debug!(count = chat_ids.len(), "Fetched chat IDs from TDLib");

        Ok(self.build_summaries_from_ids(chat_ids))
    }

    /// Builds domain `ChatSummary` list from raw TDLib chat IDs.
    ///
    /// Uses the update-driven cache for lookups instead of per-item TDLib
    /// calls. Falls back to `get_chat`/`get_user` if the cache misses
    /// (should be rare — TDLib guarantees updates arrive before IDs).
    pub(super) fn build_summaries_from_ids(&self, chat_ids: Vec<i64>) -> Vec<ChatSummary> {
        let cache = self.client.cache();
        let mut summaries = Vec::with_capacity(chat_ids.len());

        for chat_id in chat_ids {
            let chat = match cache.get_chat(chat_id) {
                Some(c) => c,
                None => match self.client.get_chat(chat_id) {
                    Ok(c) => {
                        cache.upsert_chat(c.clone());
                        c
                    }
                    Err(e) => {
                        tracing::warn!(chat_id, error = %e, "chat missing from cache and TDLib");
                        continue;
                    }
                },
            };

            let (sender_name, is_online, is_bot) = self.resolve_chat_metadata(&chat, cache);
            let summary = tdlib_mappers::map_chat_to_summary(&chat, sender_name, is_online, is_bot);
            summaries.push(summary);
        }

        summaries
    }

    /// Resolves additional metadata for a chat (sender name, online status).
    ///
    /// Uses the cache for user lookups. Falls back to `get_user` on miss.
    fn resolve_chat_metadata(
        &self,
        chat: &tdlib_rs::types::Chat,
        cache: &crate::telegram::tdlib_cache::TdLibCache,
    ) -> (Option<String>, Option<bool>, bool) {
        let chat_type = tdlib_mappers::map_chat_type(&chat.r#type);

        let (is_online, is_bot) = if matches!(chat_type, crate::domain::chat::ChatType::Private) {
            if let Some(user_id) = tdlib_mappers::get_private_chat_user_id(&chat.r#type) {
                let user = cache.get_user(user_id).or_else(|| {
                    let u = self.client.get_user(user_id).ok()?;
                    cache.upsert_user(u.clone());
                    Some(u)
                });
                match user {
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

        let sender_name = if matches!(
            chat_type,
            crate::domain::chat::ChatType::Group | crate::domain::chat::ChatType::Channel
        ) {
            chat.last_message.as_ref().and_then(|msg| {
                if let Some(user_id) = tdlib_mappers::get_sender_user_id(&msg.sender_id) {
                    let user = cache.get_user(user_id).or_else(|| {
                        let u = self.client.get_user(user_id).ok()?;
                        cache.upsert_user(u.clone());
                        Some(u)
                    });
                    user.map(|u| tdlib_mappers::format_user_name(&u))
                } else {
                    None
                }
            })
        } else {
            None
        };

        (sender_name, is_online, is_bot)
    }
}
