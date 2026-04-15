use std::sync::{mpsc::Sender, Arc};

use crate::{
    domain::events::{BackgroundError, BackgroundTaskResult},
    usecases::{
        edit_message::{edit_message, EditMessageCommand, MessageEditor},
        load_messages::{load_messages, LoadMessagesQuery, MessagesSource},
        send_message::{send_message, MessageSender, SendMessageCommand},
        send_voice::VoiceNoteSender,
    },
};

use super::error_mapping::{
    map_edit_message_error, map_load_messages_error, map_send_message_error,
};

pub(super) fn dispatch_load_messages<M: MessagesSource + Send + Sync + 'static>(
    source: &Arc<M>,
    tx: &Sender<BackgroundTaskResult>,
    chat_id: i64,
) {
    let source = Arc::clone(source);
    let tx = tx.clone();
    let tx_fallback = tx.clone();

    let spawn_result = std::thread::Builder::new()
        .name("rtg-bg-messages".into())
        .spawn(move || {
            tracing::debug!(chat_id, "background: fetching messages");
            let result = load_messages(source.as_ref(), LoadMessagesQuery::new(chat_id))
                .map(|output| output.messages)
                .map_err(|error| {
                    tracing::warn!(chat_id, error = ?error, "background: messages fetch failed");
                    BackgroundError::new(map_load_messages_error(&error))
                });

            let _ = tx.send(BackgroundTaskResult::MessagesLoaded { chat_id, result });
        });

    if let Err(error) = spawn_result {
        tracing::error!(error = %error, "failed to spawn messages background thread");
        let _ = tx_fallback.send(BackgroundTaskResult::MessagesLoaded {
            chat_id,
            result: Err(BackgroundError::new("THREAD_SPAWN_FAILED")),
        });
    }
}

pub(super) fn dispatch_load_older_messages<M: MessagesSource + Send + Sync + 'static>(
    source: &Arc<M>,
    tx: &Sender<BackgroundTaskResult>,
    chat_id: i64,
    from_message_id: i64,
) {
    let source = Arc::clone(source);
    let tx = tx.clone();
    let tx_fallback = tx.clone();

    let spawn_result = std::thread::Builder::new()
        .name("rtg-bg-older-msgs".into())
        .spawn(move || {
            tracing::debug!(
                chat_id,
                from_message_id,
                "background: fetching older messages"
            );
            let result = load_messages(
                source.as_ref(),
                LoadMessagesQuery::older_than(chat_id, from_message_id),
            )
            .map(|output| output.messages)
            .map_err(|error| {
                tracing::warn!(chat_id, error = ?error, "background: older messages fetch failed");
                BackgroundError::new(map_load_messages_error(&error))
            });

            let _ = tx.send(BackgroundTaskResult::OlderMessagesLoaded { chat_id, result });
        });

    if let Err(error) = spawn_result {
        tracing::error!(error = %error, "failed to spawn older messages background thread");
        let _ = tx_fallback.send(BackgroundTaskResult::OlderMessagesLoaded {
            chat_id,
            result: Err(BackgroundError::new("THREAD_SPAWN_FAILED")),
        });
    }
}

pub(super) fn dispatch_send_message<
    MS: MessageSender + Send + Sync + 'static,
    M: MessagesSource + Send + Sync + 'static,
>(
    sender: &Arc<MS>,
    messages_source: &Arc<M>,
    tx: &Sender<BackgroundTaskResult>,
    chat_id: i64,
    text: String,
    reply_to_message_id: Option<i64>,
) {
    let sender = Arc::clone(sender);
    let messages_source = Arc::clone(messages_source);
    let tx = tx.clone();
    let tx_fallback = tx.clone();
    let original_text = text.clone();

    let fallback_text = original_text.clone();
    let spawn_result = std::thread::Builder::new()
        .name("rtg-bg-send-msg".into())
        .spawn(move || {
            tracing::debug!(chat_id, "background: sending message");
            let command = SendMessageCommand {
                chat_id,
                text,
                reply_to_message_id,
            };
            let send_result = send_message(sender.as_ref(), command).map_err(|error| {
                tracing::warn!(chat_id, error = ?error, "background: send message failed");
                BackgroundError::new(map_send_message_error(&error))
            });

            let is_ok = send_result.is_ok();

            let _ = tx.send(BackgroundTaskResult::MessageSent {
                chat_id,
                original_text,
                result: send_result,
            });

            // If send succeeded, automatically refresh messages
            if is_ok {
                refresh_messages_after_send(&messages_source, &tx, chat_id);
            }
        });

    if let Err(error) = spawn_result {
        tracing::error!(error = %error, "failed to spawn send message background thread");
        let _ = tx_fallback.send(BackgroundTaskResult::MessageSent {
            chat_id,
            original_text: fallback_text,
            result: Err(BackgroundError::new("THREAD_SPAWN_FAILED")),
        });
    }
}

pub(super) fn dispatch_edit_message<ME: MessageEditor + Send + Sync + 'static>(
    editor: &Arc<ME>,
    tx: &Sender<BackgroundTaskResult>,
    chat_id: i64,
    message_id: i64,
    text: String,
) {
    let editor = Arc::clone(editor);
    let tx = tx.clone();
    let tx_fallback = tx.clone();
    let original_text = text.clone();
    let fallback_text = original_text.clone();

    let spawn_result = std::thread::Builder::new()
        .name("rtg-bg-edit-msg".into())
        .spawn(move || {
            tracing::debug!(chat_id, message_id, "background: editing message");
            let command = EditMessageCommand {
                chat_id,
                message_id,
                text,
            };
            let result = edit_message(editor.as_ref(), command).map_err(|error| {
                tracing::warn!(chat_id, message_id, error = ?error, "background: edit message failed");
                BackgroundError::new(map_edit_message_error(&error))
            });

            let _ = tx.send(BackgroundTaskResult::MessageEdited {
                chat_id,
                message_id,
                original_text,
                result,
            });
        });

    if let Err(error) = spawn_result {
        tracing::error!(error = %error, "failed to spawn edit message background thread");
        let _ = tx_fallback.send(BackgroundTaskResult::MessageEdited {
            chat_id,
            message_id,
            original_text: fallback_text,
            result: Err(BackgroundError::new("THREAD_SPAWN_FAILED")),
        });
    }
}

pub(super) fn dispatch_prefetch_messages<M: MessagesSource + Send + Sync + 'static>(
    source: &Arc<M>,
    tx: &Sender<BackgroundTaskResult>,
    chat_id: i64,
) {
    let source = Arc::clone(source);
    let tx = tx.clone();
    let tx_fallback = tx.clone();

    let spawn_result = std::thread::Builder::new()
        .name("rtg-bg-prefetch".into())
        .spawn(move || {
            tracing::debug!(chat_id, "background: prefetching messages");
            let result = load_messages(source.as_ref(), LoadMessagesQuery::new(chat_id))
                .map(|output| output.messages)
                .map_err(|error| {
                    tracing::warn!(chat_id, error = ?error, "background: prefetch failed");
                    BackgroundError::new(map_load_messages_error(&error))
                });

            let _ = tx.send(BackgroundTaskResult::MessagesPrefetched { chat_id, result });
        });

    if let Err(error) = spawn_result {
        tracing::error!(error = %error, "failed to spawn prefetch background thread");
        let _ = tx_fallback.send(BackgroundTaskResult::MessagesPrefetched {
            chat_id,
            result: Err(BackgroundError::new("THREAD_SPAWN_FAILED")),
        });
    }
}

pub(super) fn dispatch_send_voice<
    MS: VoiceNoteSender + Send + Sync + 'static,
    M: MessagesSource + Send + Sync + 'static,
>(
    sender: &Arc<MS>,
    messages_source: &Arc<M>,
    tx: &Sender<BackgroundTaskResult>,
    chat_id: i64,
    file_path: String,
) {
    let sender = Arc::clone(sender);
    let messages_source = Arc::clone(messages_source);
    let tx = tx.clone();
    let tx_fallback = tx.clone();

    let spawn_result = std::thread::Builder::new()
        .name("rtg-bg-send-voice".into())
        .spawn(move || {
            use crate::usecases::voice_recording;

            tracing::info!(chat_id, file_path, "background: sending voice note");

            let duration = voice_recording::get_audio_duration(&file_path).unwrap_or(0);
            let waveform = voice_recording::generate_waveform_stub();

            let result = sender
                .send_voice_note(chat_id, &file_path, duration, &waveform)
                .map_err(|error| {
                    tracing::warn!(
                        chat_id,
                        error = ?error,
                        "background: send voice note failed"
                    );
                    BackgroundError::new("SEND_VOICE_FAILED")
                });

            if result.is_err() {
                let _ = tx.send(BackgroundTaskResult::VoiceSendFailed { chat_id });
                return;
            }

            refresh_messages_after_send(&messages_source, &tx, chat_id);
        });

    if let Err(error) = spawn_result {
        tracing::error!(error = %error, "failed to spawn send-voice background thread");
        let _ = tx_fallback.send(BackgroundTaskResult::VoiceSendFailed { chat_id });
    }
}

fn refresh_messages_after_send<M: MessagesSource>(
    messages_source: &M,
    tx: &Sender<BackgroundTaskResult>,
    chat_id: i64,
) {
    tracing::debug!(chat_id, "background: refreshing messages after send");
    let refresh_result = load_messages(messages_source, LoadMessagesQuery::new(chat_id))
        .map(|output| output.messages)
        .map_err(|error| {
            tracing::warn!(
                chat_id,
                error = ?error,
                "background: messages refresh after send failed"
            );
            BackgroundError::new(map_load_messages_error(&error))
        });

    let _ = tx.send(BackgroundTaskResult::MessageSentRefreshCompleted {
        chat_id,
        result: refresh_result,
    });
}
