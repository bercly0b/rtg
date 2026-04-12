pub struct EditMessageCommand {
    pub chat_id: i64,
    pub message_id: i64,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditMessageSourceError {
    Unauthorized,
    MessageNotFound,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditMessageError {
    EmptyMessage,
    Unauthorized,
    MessageNotFound,
    TemporarilyUnavailable,
}

pub trait MessageEditor {
    fn edit_message(
        &self,
        chat_id: i64,
        message_id: i64,
        text: &str,
    ) -> Result<(), EditMessageSourceError>;
}

impl<T: MessageEditor + ?Sized> MessageEditor for &T {
    fn edit_message(
        &self,
        chat_id: i64,
        message_id: i64,
        text: &str,
    ) -> Result<(), EditMessageSourceError> {
        (*self).edit_message(chat_id, message_id, text)
    }
}

impl<T: MessageEditor + ?Sized> MessageEditor for std::sync::Arc<T> {
    fn edit_message(
        &self,
        chat_id: i64,
        message_id: i64,
        text: &str,
    ) -> Result<(), EditMessageSourceError> {
        (**self).edit_message(chat_id, message_id, text)
    }
}

pub fn edit_message(
    editor: &dyn MessageEditor,
    command: EditMessageCommand,
) -> Result<(), EditMessageError> {
    let text = command.text.trim();
    if text.is_empty() {
        return Err(EditMessageError::EmptyMessage);
    }

    editor
        .edit_message(command.chat_id, command.message_id, text)
        .map_err(map_source_error)
}

fn map_source_error(error: EditMessageSourceError) -> EditMessageError {
    match error {
        EditMessageSourceError::Unauthorized => EditMessageError::Unauthorized,
        EditMessageSourceError::MessageNotFound => EditMessageError::MessageNotFound,
        EditMessageSourceError::Unavailable => EditMessageError::TemporarilyUnavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct StubEditor {
        result: Result<(), EditMessageSourceError>,
        captured_text: RefCell<Option<String>>,
    }

    impl StubEditor {
        fn with_result(result: Result<(), EditMessageSourceError>) -> Self {
            Self {
                result,
                captured_text: RefCell::new(None),
            }
        }
    }

    impl MessageEditor for StubEditor {
        fn edit_message(
            &self,
            _chat_id: i64,
            _message_id: i64,
            text: &str,
        ) -> Result<(), EditMessageSourceError> {
            *self.captured_text.borrow_mut() = Some(text.to_owned());
            self.result.clone()
        }
    }

    #[test]
    fn rejects_empty_text() {
        let editor = StubEditor::with_result(Ok(()));
        let result = edit_message(
            &editor,
            EditMessageCommand {
                chat_id: 1,
                message_id: 1,
                text: String::new(),
            },
        );
        assert_eq!(result, Err(EditMessageError::EmptyMessage));
    }

    #[test]
    fn rejects_whitespace_only() {
        let editor = StubEditor::with_result(Ok(()));
        let result = edit_message(
            &editor,
            EditMessageCommand {
                chat_id: 1,
                message_id: 1,
                text: "   \n\t  ".to_owned(),
            },
        );
        assert_eq!(result, Err(EditMessageError::EmptyMessage));
    }

    #[test]
    fn trims_whitespace_before_editing() {
        let editor = StubEditor::with_result(Ok(()));
        let _ = edit_message(
            &editor,
            EditMessageCommand {
                chat_id: 1,
                message_id: 1,
                text: "  hello  ".to_owned(),
            },
        );
        assert_eq!(*editor.captured_text.borrow(), Some("hello".to_owned()));
    }

    #[test]
    fn returns_ok_on_success() {
        let editor = StubEditor::with_result(Ok(()));
        let result = edit_message(
            &editor,
            EditMessageCommand {
                chat_id: 1,
                message_id: 1,
                text: "updated".to_owned(),
            },
        );
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn maps_unauthorized_error() {
        let editor = StubEditor::with_result(Err(EditMessageSourceError::Unauthorized));
        let result = edit_message(
            &editor,
            EditMessageCommand {
                chat_id: 1,
                message_id: 1,
                text: "updated".to_owned(),
            },
        );
        assert_eq!(result, Err(EditMessageError::Unauthorized));
    }

    #[test]
    fn maps_message_not_found_error() {
        let editor = StubEditor::with_result(Err(EditMessageSourceError::MessageNotFound));
        let result = edit_message(
            &editor,
            EditMessageCommand {
                chat_id: 1,
                message_id: 1,
                text: "updated".to_owned(),
            },
        );
        assert_eq!(result, Err(EditMessageError::MessageNotFound));
    }

    #[test]
    fn maps_unavailable_error() {
        let editor = StubEditor::with_result(Err(EditMessageSourceError::Unavailable));
        let result = edit_message(
            &editor,
            EditMessageCommand {
                chat_id: 1,
                message_id: 1,
                text: "updated".to_owned(),
            },
        );
        assert_eq!(result, Err(EditMessageError::TemporarilyUnavailable));
    }
}
