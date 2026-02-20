//! Use case for sending a message to a chat.
//!
//! This module provides the `MessageSender` trait and `send_message` function
//! for sending text messages through the Telegram API.

/// Command to send a message to a specific chat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendMessageCommand {
    pub chat_id: i64,
    pub text: String,
}

/// Errors that can occur at the source level (Telegram API).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendMessageSourceError {
    /// User is not authorized.
    Unauthorized,
    /// Target chat was not found or is not accessible.
    ChatNotFound,
    /// Service is temporarily unavailable.
    Unavailable,
}

/// Domain-level errors for send message operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendMessageError {
    /// Message text is empty after trimming whitespace.
    EmptyMessage,
    /// User is not authorized to send messages.
    Unauthorized,
    /// Target chat was not found.
    ChatNotFound,
    /// Service is temporarily unavailable.
    TemporarilyUnavailable,
}

/// Trait for sending messages to chats.
pub trait MessageSender {
    /// Sends a text message to the specified chat.
    ///
    /// # Arguments
    /// * `chat_id` - The ID of the chat to send the message to
    /// * `text` - The message text to send
    ///
    /// # Errors
    /// Returns `SendMessageSourceError` if the message could not be sent.
    fn send_message(&self, chat_id: i64, text: &str) -> Result<(), SendMessageSourceError>;
}

impl<T: MessageSender + ?Sized> MessageSender for &T {
    fn send_message(&self, chat_id: i64, text: &str) -> Result<(), SendMessageSourceError> {
        (*self).send_message(chat_id, text)
    }
}

/// Sends a message to the specified chat.
///
/// Validates the message text (must not be empty after trimming) and delegates
/// to the `MessageSender` implementation.
///
/// # Arguments
/// * `sender` - The message sender implementation
/// * `command` - The send message command containing chat_id and text
///
/// # Errors
/// Returns `SendMessageError::EmptyMessage` if text is empty/whitespace.
/// Maps source errors to domain errors for other failure cases.
pub fn send_message(
    sender: &dyn MessageSender,
    command: SendMessageCommand,
) -> Result<(), SendMessageError> {
    let text = command.text.trim();
    if text.is_empty() {
        return Err(SendMessageError::EmptyMessage);
    }

    sender
        .send_message(command.chat_id, text)
        .map_err(map_source_error)
}

fn map_source_error(error: SendMessageSourceError) -> SendMessageError {
    match error {
        SendMessageSourceError::Unauthorized => SendMessageError::Unauthorized,
        SendMessageSourceError::ChatNotFound => SendMessageError::ChatNotFound,
        SendMessageSourceError::Unavailable => SendMessageError::TemporarilyUnavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct StubSender {
        result: Result<(), SendMessageSourceError>,
        captured_chat_id: RefCell<Option<i64>>,
        captured_text: RefCell<Option<String>>,
    }

    impl StubSender {
        fn with_result(result: Result<(), SendMessageSourceError>) -> Self {
            Self {
                result,
                captured_chat_id: RefCell::new(None),
                captured_text: RefCell::new(None),
            }
        }
    }

    impl MessageSender for StubSender {
        fn send_message(&self, chat_id: i64, text: &str) -> Result<(), SendMessageSourceError> {
            *self.captured_chat_id.borrow_mut() = Some(chat_id);
            *self.captured_text.borrow_mut() = Some(text.to_owned());
            self.result.clone()
        }
    }

    #[test]
    fn rejects_empty_message_text() {
        let sender = StubSender::with_result(Ok(()));

        let result = send_message(
            &sender,
            SendMessageCommand {
                chat_id: 1,
                text: String::new(),
            },
        );

        assert_eq!(result, Err(SendMessageError::EmptyMessage));
        assert!(sender.captured_chat_id.borrow().is_none());
    }

    #[test]
    fn rejects_whitespace_only_message() {
        let sender = StubSender::with_result(Ok(()));

        let result = send_message(
            &sender,
            SendMessageCommand {
                chat_id: 1,
                text: "   \n\t  ".to_owned(),
            },
        );

        assert_eq!(result, Err(SendMessageError::EmptyMessage));
    }

    #[test]
    fn trims_whitespace_before_sending() {
        let sender = StubSender::with_result(Ok(()));

        let _ = send_message(
            &sender,
            SendMessageCommand {
                chat_id: 42,
                text: "  hello world  ".to_owned(),
            },
        );

        assert_eq!(
            *sender.captured_text.borrow(),
            Some("hello world".to_owned())
        );
    }

    #[test]
    fn passes_chat_id_to_sender() {
        let sender = StubSender::with_result(Ok(()));

        let _ = send_message(
            &sender,
            SendMessageCommand {
                chat_id: 123,
                text: "test".to_owned(),
            },
        );

        assert_eq!(*sender.captured_chat_id.borrow(), Some(123));
    }

    #[test]
    fn returns_ok_on_successful_send() {
        let sender = StubSender::with_result(Ok(()));

        let result = send_message(
            &sender,
            SendMessageCommand {
                chat_id: 1,
                text: "hello".to_owned(),
            },
        );

        assert_eq!(result, Ok(()));
    }

    #[test]
    fn maps_unauthorized_error() {
        let sender = StubSender::with_result(Err(SendMessageSourceError::Unauthorized));

        let result = send_message(
            &sender,
            SendMessageCommand {
                chat_id: 1,
                text: "hello".to_owned(),
            },
        );

        assert_eq!(result, Err(SendMessageError::Unauthorized));
    }

    #[test]
    fn maps_chat_not_found_error() {
        let sender = StubSender::with_result(Err(SendMessageSourceError::ChatNotFound));

        let result = send_message(
            &sender,
            SendMessageCommand {
                chat_id: 1,
                text: "hello".to_owned(),
            },
        );

        assert_eq!(result, Err(SendMessageError::ChatNotFound));
    }

    #[test]
    fn maps_unavailable_error() {
        let sender = StubSender::with_result(Err(SendMessageSourceError::Unavailable));

        let result = send_message(
            &sender,
            SendMessageCommand {
                chat_id: 1,
                text: "hello".to_owned(),
            },
        );

        assert_eq!(result, Err(SendMessageError::TemporarilyUnavailable));
    }
}
