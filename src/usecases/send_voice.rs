//! Use case for sending a voice note to a chat.

use super::send_message::SendMessageSourceError;

/// Trait for sending voice notes to chats.
pub trait VoiceNoteSender {
    fn send_voice_note(
        &self,
        chat_id: i64,
        file_path: &str,
        duration: i32,
        waveform: &str,
    ) -> Result<(), SendMessageSourceError>;
}

impl<T: VoiceNoteSender + ?Sized> VoiceNoteSender for std::sync::Arc<T> {
    fn send_voice_note(
        &self,
        chat_id: i64,
        file_path: &str,
        duration: i32,
        waveform: &str,
    ) -> Result<(), SendMessageSourceError> {
        (**self).send_voice_note(chat_id, file_path, duration, waveform)
    }
}
