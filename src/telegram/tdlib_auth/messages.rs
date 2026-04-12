use std::collections::HashMap;

use crate::domain::message::Message;
use crate::usecases::edit_message::EditMessageSourceError;
use crate::usecases::load_messages::MessagesSourceError;
use crate::usecases::send_message::SendMessageSourceError;

use super::error_mapping::{map_edit_message_error, map_messages_error, map_send_message_error};
use super::TdLibAuthBackend;
use crate::telegram::tdlib_cache::TdLibCache;
use crate::telegram::tdlib_client::TdLibClient;
use crate::telegram::tdlib_mappers;

impl TdLibAuthBackend {
    /// Lists messages from TDLib's local cache only.
    ///
    /// Does **not** call `openChat`/`closeChat` or trigger any network requests.
    /// Returns whatever messages TDLib has cached locally for this chat.
    /// Used for instant chat display before a full background refresh.
    ///
    /// Returns messages in chronological order (oldest first).
    pub fn list_cached_messages(
        &self,
        chat_id: i64,
        limit: usize,
    ) -> Result<Vec<Message>, MessagesSourceError> {
        let limit_i32 = i32::try_from(limit).unwrap_or(i32::MAX);

        let td_messages = self
            .client
            .get_cached_chat_history(chat_id, 0, 0, limit_i32)
            .map_err(map_messages_error)?;

        tracing::debug!(
            chat_id,
            count = td_messages.len(),
            "fetched cached messages from TDLib"
        );

        let mut messages: Vec<Message> = td_messages
            .iter()
            .map(|msg| {
                let sender_name = self.resolve_message_sender_name(msg);
                let reply_to = self.resolve_reply_info(msg);
                let forward_info = self.resolve_forward_info(msg);
                tdlib_mappers::map_tdlib_message_to_domain(msg, sender_name, reply_to, forward_info)
            })
            .collect();

        enrich_same_chat_reply_info(&td_messages, &mut messages);

        // TDLib returns newest-first; UI expects chronological (oldest-first)
        messages.reverse();

        Ok(messages)
    }

    /// Lists messages from a chat.
    ///
    /// Returns messages in chronological order (oldest first).
    ///
    /// **Note:** The caller is responsible for the TDLib `openChat`/`closeChat`
    /// lifecycle. This method only fetches messages via paginated
    /// `getChatHistory` calls.
    pub fn list_messages(
        &self,
        chat_id: i64,
        limit: usize,
    ) -> Result<Vec<Message>, MessagesSourceError> {
        self.fetch_messages_paginated(chat_id, limit)
    }

    /// Informs TDLib that the user has opened a chat.
    pub fn open_chat(&self, chat_id: i64) -> Result<(), MessagesSourceError> {
        self.client.open_chat(chat_id).map_err(map_messages_error)
    }

    /// Informs TDLib that the user has closed a chat.
    pub fn close_chat(&self, chat_id: i64) -> Result<(), MessagesSourceError> {
        self.client.close_chat(chat_id).map_err(map_messages_error)
    }

    /// Triggers an asynchronous file download.
    pub fn download_file(&self, file_id: i32) -> Result<(), MessagesSourceError> {
        self.client
            .download_file(file_id)
            .map_err(map_messages_error)
    }

    /// Marks the given messages as viewed/read in a chat.
    pub fn view_messages(
        &self,
        chat_id: i64,
        message_ids: Vec<i64>,
    ) -> Result<(), MessagesSourceError> {
        self.client
            .view_messages(chat_id, message_ids)
            .map_err(map_messages_error)
    }

    pub fn delete_messages(
        &self,
        chat_id: i64,
        message_ids: Vec<i64>,
        revoke: bool,
    ) -> Result<(), MessagesSourceError> {
        self.client
            .delete_messages(chat_id, message_ids, revoke)
            .map_err(map_messages_error)
    }

    /// Fetches up to `limit` messages using paginated `getChatHistory` calls.
    fn fetch_messages_paginated(
        &self,
        chat_id: i64,
        limit: usize,
    ) -> Result<Vec<Message>, MessagesSourceError> {
        use crate::telegram::message_pagination::{fetch_paginated, PageResult};

        let td_messages = fetch_paginated(
            limit,
            |from_message_id, page_limit| {
                let batch = self
                    .client
                    .get_chat_history(chat_id, from_message_id, 0, page_limit)
                    .map_err(map_messages_error)?;

                tracing::debug!(
                    chat_id,
                    batch_len = batch.len(),
                    "getChatHistory page fetched"
                );

                Ok(PageResult { messages: batch })
            },
            |msg| msg.id,
        )?;

        tracing::debug!(
            chat_id,
            total = td_messages.len(),
            "message pagination complete"
        );

        // Convert to domain messages (accumulated is newest-first)
        let mut messages: Vec<Message> = td_messages
            .iter()
            .map(|msg| {
                let sender_name = self.resolve_message_sender_name(msg);
                let reply_to = self.resolve_reply_info(msg);
                let forward_info = self.resolve_forward_info(msg);
                tdlib_mappers::map_tdlib_message_to_domain(msg, sender_name, reply_to, forward_info)
            })
            .collect();

        enrich_same_chat_reply_info(&td_messages, &mut messages);

        // Reverse to get oldest first (UI expects chronological order)
        messages.reverse();

        Ok(messages)
    }

    /// Sends a text message to a chat.
    pub fn send_message(
        &self,
        chat_id: i64,
        text: &str,
        reply_to_message_id: Option<i64>,
    ) -> Result<(), SendMessageSourceError> {
        self.client
            .send_message(chat_id, text, reply_to_message_id)
            .map_err(map_send_message_error)?;

        tracing::debug!(chat_id, text_len = text.len(), "Message sent via TDLib");
        Ok(())
    }

    pub fn edit_message(
        &self,
        chat_id: i64,
        message_id: i64,
        text: &str,
    ) -> Result<(), EditMessageSourceError> {
        self.client
            .edit_message_text(chat_id, message_id, text)
            .map_err(map_edit_message_error)?;

        tracing::debug!(
            chat_id,
            message_id,
            text_len = text.len(),
            "Message edited via TDLib"
        );
        Ok(())
    }

    /// Sends a voice note to a chat.
    pub fn send_voice_note(
        &self,
        chat_id: i64,
        file_path: &str,
        duration: i32,
        waveform: &str,
    ) -> Result<(), SendMessageSourceError> {
        self.client
            .send_voice_note(chat_id, file_path, duration, waveform)
            .map_err(map_send_message_error)?;

        tracing::debug!(chat_id, file_path, duration, "Voice note sent via TDLib");
        Ok(())
    }

    /// Resolves the sender name for a message.
    fn resolve_message_sender_name(&self, msg: &tdlib_rs::types::Message) -> String {
        resolve_sender_name(self.client.cache(), &self.client, msg)
    }

    /// Resolves reply information for a message using the TDLib cache.
    fn resolve_reply_info(
        &self,
        msg: &tdlib_rs::types::Message,
    ) -> Option<crate::domain::message::ReplyInfo> {
        let cache = self.client.cache();
        let my_user_id = cache.my_user_id();
        tdlib_mappers::extract_reply_info(
            msg,
            |user_id| {
                cache
                    .get_user(user_id)
                    .map(|u| tdlib_mappers::format_user_name(&u))
            },
            |chat_id| cache.get_chat(chat_id).map(|c| c.title.clone()),
            my_user_id,
        )
    }

    fn resolve_forward_info(
        &self,
        msg: &tdlib_rs::types::Message,
    ) -> Option<crate::domain::message::ForwardInfo> {
        let cache = self.client.cache();
        tdlib_mappers::extract_forward_info(
            msg,
            |user_id| {
                cache
                    .get_user(user_id)
                    .map(|u| tdlib_mappers::format_user_name(&u))
            },
            |chat_id| cache.get_chat(chat_id).map(|c| c.title.clone()),
        )
    }

    /// Creates a `MessageMapper` that can be shared with the chat updates monitor.
    pub fn create_message_mapper(
        &self,
    ) -> std::sync::Arc<dyn crate::telegram::chat_updates::MessageMapper> {
        std::sync::Arc::new(TdLibMessageMapper {
            cache: self.client.cache().clone(),
            rt: self.client.runtime().clone(),
            client_id: self.client.client_id(),
        })
    }
}

/// Resolves the sender name for a TDLib message using the cache with
/// TDLib client fallback.
fn resolve_sender_name(
    cache: &TdLibCache,
    client: &TdLibClient,
    msg: &tdlib_rs::types::Message,
) -> String {
    match &msg.sender_id {
        tdlib_rs::enums::MessageSender::User(u) => cache
            .get_user(u.user_id)
            .or_else(|| {
                let user = client.get_user(u.user_id).ok()?;
                cache.upsert_user(user.clone());
                Some(user)
            })
            .map(|user| tdlib_mappers::format_user_name(&user))
            .unwrap_or_else(|| "Unknown".to_owned()),
        tdlib_rs::enums::MessageSender::Chat(c) => cache
            .get_chat(c.chat_id)
            .or_else(|| {
                let chat = client.get_chat(c.chat_id).ok()?;
                cache.upsert_chat(chat.clone());
                Some(chat)
            })
            .map(|chat| chat.title.clone())
            .unwrap_or_else(|| "Channel".to_owned()),
    }
}

fn enrich_same_chat_reply_info(
    raw_messages: &[tdlib_rs::types::Message],
    messages: &mut [Message],
) {
    use tdlib_rs::enums::MessageReplyTo;

    let by_id: HashMap<i64, (String, String, bool)> = messages
        .iter()
        .map(|m| {
            (
                m.id,
                (
                    reply_sender_name_for_message(m),
                    m.display_content(),
                    m.is_outgoing,
                ),
            )
        })
        .collect();

    for (raw, mapped) in raw_messages.iter().zip(messages.iter_mut()) {
        let Some(reply) = mapped.reply_to.as_mut() else {
            continue;
        };

        let Some(MessageReplyTo::Message(info)) = raw.reply_to.as_ref() else {
            continue;
        };

        if let Some((sender_name, text, is_outgoing)) = by_id.get(&info.message_id) {
            if reply.sender_name.is_empty() {
                reply.sender_name = sender_name.clone();
            }
            if reply.text.is_empty() {
                reply.text = text.clone();
            }
            reply.is_outgoing = *is_outgoing;
        }
    }
}

pub(super) fn reply_sender_name_for_message(message: &Message) -> String {
    if message.is_outgoing {
        "You".to_owned()
    } else {
        message.sender_name.clone()
    }
}

/// Resolves reply info from the TDLib cache (used by `TdLibMessageMapper`).
fn extract_reply_info_from_cache(
    msg: &tdlib_rs::types::Message,
    cache: &TdLibCache,
    _rt: &std::sync::Arc<tokio::runtime::Runtime>,
    _client_id: i32,
) -> Option<crate::domain::message::ReplyInfo> {
    let my_user_id = cache.my_user_id();
    tdlib_mappers::extract_reply_info(
        msg,
        |user_id| {
            cache
                .get_user(user_id)
                .map(|u| tdlib_mappers::format_user_name(&u))
        },
        |chat_id| cache.get_chat(chat_id).map(|c| c.title.clone()),
        my_user_id,
    )
}

fn extract_forward_info_from_cache(
    msg: &tdlib_rs::types::Message,
    cache: &TdLibCache,
) -> Option<crate::domain::message::ForwardInfo> {
    tdlib_mappers::extract_forward_info(
        msg,
        |user_id| {
            cache
                .get_user(user_id)
                .map(|u| tdlib_mappers::format_user_name(&u))
        },
        |chat_id| cache.get_chat(chat_id).map(|c| c.title.clone()),
    )
}

/// Maps raw TDLib messages to domain `Message` types.
///
/// Uses the shared TDLib cache for sender name resolution (fast path).
/// Falls back to `rt.block_on(get_user/get_chat)` on cache misses and
/// warms the cache on success.
struct TdLibMessageMapper {
    cache: TdLibCache,
    rt: std::sync::Arc<tokio::runtime::Runtime>,
    client_id: i32,
}

impl crate::telegram::chat_updates::MessageMapper for TdLibMessageMapper {
    fn map_message(&self, raw: &tdlib_rs::types::Message) -> Message {
        let sender_name = match &raw.sender_id {
            tdlib_rs::enums::MessageSender::User(u) => {
                if let Some(user) = self.cache.get_user(u.user_id) {
                    tdlib_mappers::format_user_name(&user)
                } else if let Ok(tdlib_rs::enums::User::User(user)) = self.rt.block_on(async {
                    tdlib_rs::functions::get_user(u.user_id, self.client_id).await
                }) {
                    self.cache.upsert_user(user.clone());
                    tdlib_mappers::format_user_name(&user)
                } else {
                    "Unknown".to_owned()
                }
            }
            tdlib_rs::enums::MessageSender::Chat(c) => {
                if let Some(chat) = self.cache.get_chat(c.chat_id) {
                    chat.title.clone()
                } else if let Ok(tdlib_rs::enums::Chat::Chat(chat)) = self.rt.block_on(async {
                    tdlib_rs::functions::get_chat(c.chat_id, self.client_id).await
                }) {
                    let title = chat.title.clone();
                    self.cache.upsert_chat(chat);
                    title
                } else {
                    "Channel".to_owned()
                }
            }
        };
        let reply_to = extract_reply_info_from_cache(raw, &self.cache, &self.rt, self.client_id);
        let forward_info = extract_forward_info_from_cache(raw, &self.cache);
        tdlib_mappers::map_tdlib_message_to_domain(raw, sender_name, reply_to, forward_info)
    }
}
