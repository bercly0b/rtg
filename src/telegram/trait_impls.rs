use crate::{
    domain::message::Message,
    usecases::{
        chat_lifecycle::{
            ChatLifecycle, ChatLifecycleError, ChatReadMarker, FileDownloader, MessageDeleter,
        },
        chat_subtitle::{ChatInfoQuery, ChatSubtitleError, ChatSubtitleQuery, ChatSubtitleSource},
        list_chats::{ListChatsSource, ListChatsSourceError},
        load_messages::{CachedMessagesSource, MessagesSource, MessagesSourceError},
        send_message::{MessageSender, SendMessageSourceError},
        send_voice::VoiceNoteSender,
    },
};

use super::TelegramAdapter;

impl ListChatsSource for TelegramAdapter {
    fn list_chats(
        &self,
        limit: usize,
        force: bool,
    ) -> Result<Vec<crate::domain::chat::ChatSummary>, ListChatsSourceError> {
        match self.tdlib_backend.as_ref() {
            Some(backend) => backend.list_chat_summaries(limit, force),
            None => Err(ListChatsSourceError::Unavailable),
        }
    }
}

impl MessagesSource for TelegramAdapter {
    fn list_messages(
        &self,
        chat_id: i64,
        limit: usize,
    ) -> Result<Vec<Message>, MessagesSourceError> {
        match self.tdlib_backend.as_ref() {
            Some(backend) => backend.list_messages(chat_id, limit),
            None => Err(MessagesSourceError::Unavailable),
        }
    }
}

impl CachedMessagesSource for TelegramAdapter {
    fn list_cached_messages(
        &self,
        chat_id: i64,
        limit: usize,
    ) -> Result<Vec<Message>, MessagesSourceError> {
        match self.tdlib_backend.as_ref() {
            Some(backend) => backend.list_cached_messages(chat_id, limit),
            None => Ok(Vec::new()),
        }
    }
}

impl MessageSender for TelegramAdapter {
    fn send_message(
        &self,
        chat_id: i64,
        text: &str,
        reply_to_message_id: Option<i64>,
    ) -> Result<(), SendMessageSourceError> {
        match self.tdlib_backend.as_ref() {
            Some(backend) => backend.send_message(chat_id, text, reply_to_message_id),
            None => Err(SendMessageSourceError::Unauthorized),
        }
    }
}

impl VoiceNoteSender for TelegramAdapter {
    fn send_voice_note(
        &self,
        chat_id: i64,
        file_path: &str,
        duration: i32,
        waveform: &str,
    ) -> Result<(), SendMessageSourceError> {
        match self.tdlib_backend.as_ref() {
            Some(backend) => backend.send_voice_note(chat_id, file_path, duration, waveform),
            None => Err(SendMessageSourceError::Unauthorized),
        }
    }
}

impl ChatLifecycle for TelegramAdapter {
    fn open_chat(&self, chat_id: i64) -> Result<(), ChatLifecycleError> {
        match self.tdlib_backend.as_ref() {
            Some(backend) => backend.open_chat(chat_id).map_err(|e| {
                tracing::debug!(chat_id, error = ?e, "open_chat mapped to lifecycle error");
                ChatLifecycleError::Unavailable
            }),
            None => Err(ChatLifecycleError::Unavailable),
        }
    }

    fn close_chat(&self, chat_id: i64) -> Result<(), ChatLifecycleError> {
        match self.tdlib_backend.as_ref() {
            Some(backend) => backend.close_chat(chat_id).map_err(|e| {
                tracing::debug!(chat_id, error = ?e, "close_chat mapped to lifecycle error");
                ChatLifecycleError::Unavailable
            }),
            None => Err(ChatLifecycleError::Unavailable),
        }
    }
}

impl FileDownloader for TelegramAdapter {
    fn download_file(&self, file_id: i32) -> Result<(), ChatLifecycleError> {
        match self.tdlib_backend.as_ref() {
            Some(backend) => backend.download_file(file_id).map_err(|e| {
                tracing::debug!(file_id, error = ?e, "download_file mapped to lifecycle error");
                ChatLifecycleError::Unavailable
            }),
            None => Err(ChatLifecycleError::Unavailable),
        }
    }
}

impl ChatReadMarker for TelegramAdapter {
    fn mark_messages_read(
        &self,
        chat_id: i64,
        message_ids: Vec<i64>,
    ) -> Result<(), ChatLifecycleError> {
        match self.tdlib_backend.as_ref() {
            Some(backend) => backend.view_messages(chat_id, message_ids).map_err(|e| {
                tracing::debug!(chat_id, error = ?e, "view_messages mapped to lifecycle error");
                ChatLifecycleError::Unavailable
            }),
            None => Err(ChatLifecycleError::Unavailable),
        }
    }
}

impl MessageDeleter for TelegramAdapter {
    fn delete_messages(
        &self,
        chat_id: i64,
        message_ids: Vec<i64>,
        revoke: bool,
    ) -> Result<(), ChatLifecycleError> {
        match self.tdlib_backend.as_ref() {
            Some(backend) => backend.delete_messages(chat_id, message_ids, revoke).map_err(|e| {
                tracing::debug!(chat_id, error = ?e, "delete_messages mapped to lifecycle error");
                ChatLifecycleError::Unavailable
            }),
            None => Err(ChatLifecycleError::Unavailable),
        }
    }
}

impl ChatSubtitleSource for TelegramAdapter {
    fn resolve_chat_subtitle(
        &self,
        query: &ChatSubtitleQuery,
    ) -> Result<crate::domain::chat_subtitle::ChatSubtitle, ChatSubtitleError> {
        match self.tdlib_backend.as_ref() {
            Some(backend) => Ok(backend.resolve_subtitle(query.chat_id)),
            None => Err(ChatSubtitleError::Unavailable),
        }
    }

    fn resolve_chat_info(
        &self,
        query: &ChatInfoQuery,
    ) -> Result<crate::domain::chat_info_state::ChatInfo, ChatSubtitleError> {
        match self.tdlib_backend.as_ref() {
            Some(backend) => {
                Ok(backend.resolve_chat_info(query.chat_id, query.chat_type, query.title.clone()))
            }
            None => Err(ChatSubtitleError::Unavailable),
        }
    }
}
