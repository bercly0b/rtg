use super::types::TdLibError;
use super::TdLibClient;

impl TdLibClient {
    /// Triggers an asynchronous file download in TDLib.
    ///
    /// Progress updates are delivered via `Update::File` events.
    /// The file will be stored in the TDLib files directory.
    pub fn download_file(&self, file_id: i32) -> Result<(), TdLibError> {
        let client_id = self.client_id;

        self.rt.block_on(async {
            tdlib_rs::functions::download_file(
                file_id, 16,    // priority (1-32, 16 = medium-high)
                0,     // offset (from start)
                0,     // limit (0 = entire file)
                false, // synchronous = false (async, progress via updateFile)
                client_id,
            )
            .await
            .map(|_| ())
            .map_err(|e| TdLibError::Request {
                code: e.code,
                message: e.message,
            })
        })
    }

    /// Gets message history from TDLib's local database only.
    ///
    /// Unlike [`get_chat_history`](Self::get_chat_history), this uses
    /// `only_local: true`, so it never triggers a network request. Returns
    /// whatever messages TDLib has cached locally from previous fetches.
    ///
    /// Useful for instant chat display: show cached messages immediately,
    /// then refresh from the server in the background.
    pub fn get_cached_chat_history(
        &self,
        chat_id: i64,
        from_message_id: i64,
        offset: i32,
        limit: i32,
    ) -> Result<Vec<tdlib_rs::types::Message>, TdLibError> {
        let client_id = self.client_id;

        self.rt.block_on(async {
            let messages = tdlib_rs::functions::get_chat_history(
                chat_id,
                from_message_id,
                offset,
                limit,
                true, // only_local: read from cache only, no network
                client_id,
            )
            .await
            .map_err(|e| TdLibError::Request {
                code: e.code,
                message: e.message,
            })?;

            match messages {
                tdlib_rs::enums::Messages::Messages(m) => {
                    Ok(m.messages.into_iter().flatten().collect())
                }
            }
        })
    }

    /// Gets message history for a chat.
    ///
    /// Returns messages in reverse chronological order (newest first).
    /// Use `from_message_id: 0` to get the most recent messages.
    pub fn get_chat_history(
        &self,
        chat_id: i64,
        from_message_id: i64,
        offset: i32,
        limit: i32,
    ) -> Result<Vec<tdlib_rs::types::Message>, TdLibError> {
        let client_id = self.client_id;

        self.rt.block_on(async {
            let messages = tdlib_rs::functions::get_chat_history(
                chat_id,
                from_message_id,
                offset,
                limit,
                false, // only_local: fetch from server if needed
                client_id,
            )
            .await
            .map_err(|e| TdLibError::Request {
                code: e.code,
                message: e.message,
            })?;

            match messages {
                tdlib_rs::enums::Messages::Messages(m) => {
                    // Filter out None values (deleted messages)
                    Ok(m.messages.into_iter().flatten().collect())
                }
            }
        })
    }

    /// Sends a text message to a chat.
    ///
    /// Returns the sent message (which may have a temporary ID until confirmed).
    pub fn send_message(
        &self,
        chat_id: i64,
        text: &str,
        reply_to_message_id: Option<i64>,
    ) -> Result<tdlib_rs::types::Message, TdLibError> {
        let client_id = self.client_id;
        let text = text.to_owned();

        self.rt.block_on(async {
            let formatted_text = tdlib_rs::types::FormattedText {
                text,
                entities: vec![],
            };

            let input_content = tdlib_rs::enums::InputMessageContent::InputMessageText(
                tdlib_rs::types::InputMessageText {
                    text: formatted_text,
                    link_preview_options: None,
                    clear_draft: true,
                },
            );

            let reply_to = reply_to_message_id.map(|msg_id| {
                tdlib_rs::enums::InputMessageReplyTo::Message(
                    tdlib_rs::types::InputMessageReplyToMessage {
                        message_id: msg_id,
                        quote: None,
                        checklist_task_id: 0,
                    },
                )
            });

            let message = tdlib_rs::functions::send_message(
                chat_id,
                None, // topic_id
                reply_to,
                None, // options
                input_content,
                client_id,
            )
            .await
            .map_err(|e| TdLibError::Request {
                code: e.code,
                message: e.message,
            })?;

            match message {
                tdlib_rs::enums::Message::Message(m) => Ok(m),
            }
        })
    }

    /// Sends a voice note to a chat.
    ///
    /// The voice note file must be Opus-encoded in an OGG container.
    pub fn send_voice_note(
        &self,
        chat_id: i64,
        file_path: &str,
        duration: i32,
        waveform: &str,
    ) -> Result<tdlib_rs::types::Message, TdLibError> {
        let client_id = self.client_id;
        let file_path = file_path.to_owned();
        let waveform = waveform.to_owned();

        self.rt.block_on(async {
            let voice_note = tdlib_rs::enums::InputFile::Local(tdlib_rs::types::InputFileLocal {
                path: file_path,
            });

            let input_content = tdlib_rs::enums::InputMessageContent::InputMessageVoiceNote(
                tdlib_rs::types::InputMessageVoiceNote {
                    voice_note,
                    duration,
                    waveform,
                    caption: None,
                    self_destruct_type: None,
                },
            );

            let message = tdlib_rs::functions::send_message(
                chat_id,
                None, // topic_id
                None, // reply_to
                None, // options
                input_content,
                client_id,
            )
            .await
            .map_err(|e| TdLibError::Request {
                code: e.code,
                message: e.message,
            })?;

            match message {
                tdlib_rs::enums::Message::Message(m) => Ok(m),
            }
        })
    }

    /// Deletes messages from a chat.
    ///
    /// When `revoke` is true, the messages are deleted for all participants
    /// (if Telegram allows it). When false, only for the current user.
    pub fn delete_messages(
        &self,
        chat_id: i64,
        message_ids: Vec<i64>,
        revoke: bool,
    ) -> Result<(), TdLibError> {
        let client_id = self.client_id;

        self.rt.block_on(async {
            tdlib_rs::functions::delete_messages(chat_id, message_ids, revoke, client_id)
                .await
                .map_err(|e| TdLibError::Request {
                    code: e.code,
                    message: e.message,
                })
        })
    }
}
