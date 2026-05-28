//! Use case for sending a voice note to a chat.

use super::send_message::SendMessageSourceError;

/// Trait for sending voice notes to chats.
pub trait VoiceNoteSender {
    /// Sends a voice note to the chat, or to a specific forum topic when
    /// `topic_id` is `Some`.
    fn send_voice_note(
        &self,
        chat_id: i64,
        topic_id: Option<i32>,
        file_path: &str,
        duration: i32,
        waveform: &str,
    ) -> Result<(), SendMessageSourceError>;
}

impl<T: VoiceNoteSender + ?Sized> VoiceNoteSender for std::sync::Arc<T> {
    fn send_voice_note(
        &self,
        chat_id: i64,
        topic_id: Option<i32>,
        file_path: &str,
        duration: i32,
        waveform: &str,
    ) -> Result<(), SendMessageSourceError> {
        (**self).send_voice_note(chat_id, topic_id, file_path, duration, waveform)
    }
}
