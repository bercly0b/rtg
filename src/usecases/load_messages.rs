use crate::domain::message::Message;

const DEFAULT_MESSAGES_PAGE_SIZE: usize = 50;
const MAX_MESSAGES_PAGE_SIZE: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadMessagesQuery {
    pub chat_id: i64,
    pub limit: usize,
}

impl LoadMessagesQuery {
    pub fn new(chat_id: i64) -> Self {
        Self {
            chat_id,
            limit: DEFAULT_MESSAGES_PAGE_SIZE,
        }
    }

    fn normalized_limit(&self) -> usize {
        match self.limit {
            0 => DEFAULT_MESSAGES_PAGE_SIZE,
            value if value > MAX_MESSAGES_PAGE_SIZE => MAX_MESSAGES_PAGE_SIZE,
            value => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadMessagesOutput {
    pub messages: Vec<Message>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessagesSourceError {
    Unauthorized,
    Unavailable,
    InvalidData,
    ChatNotFound,
}

pub trait MessagesSource {
    fn list_messages(
        &self,
        chat_id: i64,
        limit: usize,
    ) -> Result<Vec<Message>, MessagesSourceError>;
}

impl<T> MessagesSource for &T
where
    T: MessagesSource + ?Sized,
{
    fn list_messages(
        &self,
        chat_id: i64,
        limit: usize,
    ) -> Result<Vec<Message>, MessagesSourceError> {
        (*self).list_messages(chat_id, limit)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadMessagesError {
    Unauthorized,
    TemporarilyUnavailable,
    DataContractViolation,
    ChatNotFound,
}

pub fn load_messages(
    source: &dyn MessagesSource,
    query: LoadMessagesQuery,
) -> Result<LoadMessagesOutput, LoadMessagesError> {
    let limit = query.normalized_limit();
    let messages = source
        .list_messages(query.chat_id, limit)
        .map_err(map_source_error)?;

    Ok(LoadMessagesOutput { messages })
}

fn map_source_error(error: MessagesSourceError) -> LoadMessagesError {
    match error {
        MessagesSourceError::Unauthorized => LoadMessagesError::Unauthorized,
        MessagesSourceError::Unavailable => LoadMessagesError::TemporarilyUnavailable,
        MessagesSourceError::InvalidData => LoadMessagesError::DataContractViolation,
        MessagesSourceError::ChatNotFound => LoadMessagesError::ChatNotFound,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubSource {
        result: Result<Vec<Message>, MessagesSourceError>,
        captured_chat_id: std::sync::Mutex<Option<i64>>,
        captured_limit: std::sync::Mutex<Option<usize>>,
    }

    impl StubSource {
        fn with_result(result: Result<Vec<Message>, MessagesSourceError>) -> Self {
            Self {
                result,
                captured_chat_id: std::sync::Mutex::new(None),
                captured_limit: std::sync::Mutex::new(None),
            }
        }
    }

    impl MessagesSource for StubSource {
        fn list_messages(
            &self,
            chat_id: i64,
            limit: usize,
        ) -> Result<Vec<Message>, MessagesSourceError> {
            *self.captured_chat_id.lock().expect("chat_id lock") = Some(chat_id);
            *self.captured_limit.lock().expect("limit lock") = Some(limit);
            self.result.clone()
        }
    }

    fn sample_message() -> Message {
        Message {
            id: 1,
            sender_name: "User".to_owned(),
            text: "Hello".to_owned(),
            timestamp_ms: 1000,
            is_outgoing: false,
            media: crate::domain::message::MessageMedia::None,
        }
    }

    #[test]
    fn uses_default_limit_when_query_limit_is_zero() {
        let source = StubSource::with_result(Ok(vec![]));

        let _ = load_messages(
            &source,
            LoadMessagesQuery {
                chat_id: 1,
                limit: 0,
            },
        )
        .expect("load should succeed");

        assert_eq!(*source.captured_limit.lock().expect("limit lock"), Some(50));
    }

    #[test]
    fn caps_limit_to_maximum_boundary() {
        let source = StubSource::with_result(Ok(vec![]));

        let _ = load_messages(
            &source,
            LoadMessagesQuery {
                chat_id: 1,
                limit: 999,
            },
        )
        .expect("load should succeed");

        assert_eq!(
            *source.captured_limit.lock().expect("limit lock"),
            Some(200)
        );
    }

    #[test]
    fn passes_chat_id_to_source() {
        let source = StubSource::with_result(Ok(vec![]));

        let _ = load_messages(&source, LoadMessagesQuery::new(42)).expect("load should succeed");

        assert_eq!(
            *source.captured_chat_id.lock().expect("chat_id lock"),
            Some(42)
        );
    }

    #[test]
    fn keeps_source_payload_without_mutation() {
        let messages = vec![sample_message()];
        let source = StubSource::with_result(Ok(messages.clone()));

        let output =
            load_messages(&source, LoadMessagesQuery::new(1)).expect("load should succeed");

        assert_eq!(output.messages, messages);
    }

    #[test]
    fn maps_unauthorized_error() {
        let source = StubSource::with_result(Err(MessagesSourceError::Unauthorized));

        let err = load_messages(&source, LoadMessagesQuery::new(1)).expect_err("must fail");

        assert_eq!(err, LoadMessagesError::Unauthorized);
    }

    #[test]
    fn maps_chat_not_found_error() {
        let source = StubSource::with_result(Err(MessagesSourceError::ChatNotFound));

        let err = load_messages(&source, LoadMessagesQuery::new(1)).expect_err("must fail");

        assert_eq!(err, LoadMessagesError::ChatNotFound);
    }
}
