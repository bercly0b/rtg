use crate::domain::chat::ChatSummary;

pub const DEFAULT_CHAT_PAGE_SIZE: usize = 50;

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListChatsQuery {
    pub limit: usize,
    /// When `true`, bypasses the in-memory cache and reads directly from
    /// TDLib's SQLite — guarantees fresh data for user-initiated refreshes.
    pub force: bool,
}

impl Default for ListChatsQuery {
    fn default() -> Self {
        Self {
            limit: DEFAULT_CHAT_PAGE_SIZE,
            force: false,
        }
    }
}

impl ListChatsQuery {
    fn normalized_limit(&self) -> usize {
        match self.limit {
            0 => DEFAULT_CHAT_PAGE_SIZE,
            value => value,
        }
    }
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListChatsOutput {
    pub chats: Vec<ChatSummary>,
    pub all_loaded: bool,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListChatsSourceError {
    Unauthorized,
    Unavailable,
    InvalidData,
    Unknown,
}

#[cfg_attr(not(test), allow(dead_code))]
pub trait ListChatsSource {
    fn list_chats(
        &self,
        limit: usize,
        force: bool,
    ) -> Result<(Vec<ChatSummary>, bool), ListChatsSourceError>;
}

impl<T> ListChatsSource for &T
where
    T: ListChatsSource + ?Sized,
{
    fn list_chats(
        &self,
        limit: usize,
        force: bool,
    ) -> Result<(Vec<ChatSummary>, bool), ListChatsSourceError> {
        (*self).list_chats(limit, force)
    }
}

impl<T> ListChatsSource for std::sync::Arc<T>
where
    T: ListChatsSource + ?Sized,
{
    fn list_chats(
        &self,
        limit: usize,
        force: bool,
    ) -> Result<(Vec<ChatSummary>, bool), ListChatsSourceError> {
        (**self).list_chats(limit, force)
    }
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListChatsError {
    Unauthorized,
    TemporarilyUnavailable,
    DataContractViolation,
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn list_chats(
    source: &dyn ListChatsSource,
    query: ListChatsQuery,
) -> Result<ListChatsOutput, ListChatsError> {
    let limit = query.normalized_limit();
    let (chats, all_loaded) = source
        .list_chats(limit, query.force)
        .map_err(map_source_error)?;

    Ok(ListChatsOutput { chats, all_loaded })
}

fn map_source_error(error: ListChatsSourceError) -> ListChatsError {
    match error {
        ListChatsSourceError::Unauthorized => ListChatsError::Unauthorized,
        ListChatsSourceError::Unavailable | ListChatsSourceError::Unknown => {
            ListChatsError::TemporarilyUnavailable
        }
        ListChatsSourceError::InvalidData => ListChatsError::DataContractViolation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubSource {
        result: Result<Vec<ChatSummary>, ListChatsSourceError>,
        captured_limit: std::sync::Mutex<Option<usize>>,
    }

    impl StubSource {
        fn with_result(result: Result<Vec<ChatSummary>, ListChatsSourceError>) -> Self {
            Self {
                result,
                captured_limit: std::sync::Mutex::new(None),
            }
        }
    }

    impl ListChatsSource for StubSource {
        fn list_chats(
            &self,
            limit: usize,
            _force: bool,
        ) -> Result<(Vec<ChatSummary>, bool), ListChatsSourceError> {
            *self.captured_limit.lock().expect("limit lock") = Some(limit);
            self.result.clone().map(|chats| (chats, false))
        }
    }

    fn sample_chat() -> ChatSummary {
        use crate::domain::chat::{ChatType, OutgoingReadStatus};
        ChatSummary {
            chat_id: 42,
            title: "rtg".to_owned(),
            unread_count: 3,
            last_message_preview: Some("hello".to_owned()),
            last_message_unix_ms: Some(1_700_000_000_000),
            is_pinned: false,
            chat_type: ChatType::Private,
            last_message_sender: None,
            is_online: None,
            is_bot: false,
            outgoing_status: OutgoingReadStatus::default(),
            last_message_id: None,
            unread_reaction_count: 0,
        }
    }

    #[test]
    fn uses_default_limit_when_query_limit_is_zero() {
        let source = StubSource::with_result(Ok(vec![]));

        let _ = list_chats(
            &source,
            ListChatsQuery {
                limit: 0,
                ..ListChatsQuery::default()
            },
        )
        .expect("list should succeed");

        assert_eq!(*source.captured_limit.lock().expect("limit lock"), Some(50));
    }

    #[test]
    fn passes_custom_limit_through() {
        let source = StubSource::with_result(Ok(vec![]));

        let _ = list_chats(
            &source,
            ListChatsQuery {
                limit: 999,
                ..ListChatsQuery::default()
            },
        )
        .expect("list should succeed");

        assert_eq!(
            *source.captured_limit.lock().expect("limit lock"),
            Some(999)
        );
    }

    #[test]
    fn keeps_source_payload_without_mutation() {
        let chats = vec![sample_chat()];
        let source = StubSource::with_result(Ok(chats.clone()));

        let output = list_chats(&source, ListChatsQuery::default()).expect("list should succeed");

        assert_eq!(output.chats, chats);
    }

    #[test]
    fn maps_unauthorized_error() {
        let source = StubSource::with_result(Err(ListChatsSourceError::Unauthorized));

        let err = list_chats(&source, ListChatsQuery::default()).expect_err("must fail");

        assert_eq!(err, ListChatsError::Unauthorized);
    }

    #[test]
    fn maps_unavailable_error_to_temporarily_unavailable() {
        let source = StubSource::with_result(Err(ListChatsSourceError::Unavailable));

        let err = list_chats(&source, ListChatsQuery::default()).expect_err("must fail");

        assert_eq!(err, ListChatsError::TemporarilyUnavailable);
    }

    #[test]
    fn maps_invalid_data_error_to_contract_violation() {
        let source = StubSource::with_result(Err(ListChatsSourceError::InvalidData));

        let err = list_chats(&source, ListChatsQuery::default()).expect_err("must fail");

        assert_eq!(err, ListChatsError::DataContractViolation);
    }

    #[test]
    fn maps_unknown_error_to_temporarily_unavailable() {
        let source = StubSource::with_result(Err(ListChatsSourceError::Unknown));

        let err = list_chats(&source, ListChatsQuery::default()).expect_err("must fail");

        assert_eq!(err, ListChatsError::TemporarilyUnavailable);
    }
}
