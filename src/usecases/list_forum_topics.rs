use crate::domain::forum_topic::ForumTopicSummary;

/// Default page size for the topic list. Telegram forums rarely have more
/// than a few dozen topics, so a single page suffices in v1.
pub const DEFAULT_FORUM_TOPIC_PAGE_SIZE: usize = 100;

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListForumTopicsQuery {
    pub chat_id: i64,
    pub limit: usize,
}

impl ListForumTopicsQuery {
    pub fn new(chat_id: i64) -> Self {
        Self {
            chat_id,
            limit: DEFAULT_FORUM_TOPIC_PAGE_SIZE,
        }
    }

    fn normalized_limit(&self) -> usize {
        match self.limit {
            0 => DEFAULT_FORUM_TOPIC_PAGE_SIZE,
            value => value,
        }
    }
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListForumTopicsOutput {
    pub topics: Vec<ForumTopicSummary>,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListForumTopicsSourceError {
    Unauthorized,
    Unavailable,
    ChatNotFound,
    InvalidData,
    Unknown,
}

#[cfg_attr(not(test), allow(dead_code))]
pub trait ForumTopicsSource {
    fn list_forum_topics(
        &self,
        chat_id: i64,
        limit: usize,
    ) -> Result<Vec<ForumTopicSummary>, ListForumTopicsSourceError>;
}

impl<T> ForumTopicsSource for &T
where
    T: ForumTopicsSource + ?Sized,
{
    fn list_forum_topics(
        &self,
        chat_id: i64,
        limit: usize,
    ) -> Result<Vec<ForumTopicSummary>, ListForumTopicsSourceError> {
        (*self).list_forum_topics(chat_id, limit)
    }
}

impl<T> ForumTopicsSource for std::sync::Arc<T>
where
    T: ForumTopicsSource + ?Sized,
{
    fn list_forum_topics(
        &self,
        chat_id: i64,
        limit: usize,
    ) -> Result<Vec<ForumTopicSummary>, ListForumTopicsSourceError> {
        (**self).list_forum_topics(chat_id, limit)
    }
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListForumTopicsError {
    Unauthorized,
    TemporarilyUnavailable,
    ChatNotFound,
    DataContractViolation,
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn list_forum_topics(
    source: &dyn ForumTopicsSource,
    query: ListForumTopicsQuery,
) -> Result<ListForumTopicsOutput, ListForumTopicsError> {
    let limit = query.normalized_limit();
    let topics = source
        .list_forum_topics(query.chat_id, limit)
        .map_err(map_source_error)?;

    Ok(ListForumTopicsOutput { topics })
}

fn map_source_error(error: ListForumTopicsSourceError) -> ListForumTopicsError {
    match error {
        ListForumTopicsSourceError::Unauthorized => ListForumTopicsError::Unauthorized,
        ListForumTopicsSourceError::ChatNotFound => ListForumTopicsError::ChatNotFound,
        ListForumTopicsSourceError::Unavailable | ListForumTopicsSourceError::Unknown => {
            ListForumTopicsError::TemporarilyUnavailable
        }
        ListForumTopicsSourceError::InvalidData => ListForumTopicsError::DataContractViolation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubSource {
        result: Result<Vec<ForumTopicSummary>, ListForumTopicsSourceError>,
        captured: std::sync::Mutex<Option<(i64, usize)>>,
    }

    impl StubSource {
        fn with_result(result: Result<Vec<ForumTopicSummary>, ListForumTopicsSourceError>) -> Self {
            Self {
                result,
                captured: std::sync::Mutex::new(None),
            }
        }
    }

    impl ForumTopicsSource for StubSource {
        fn list_forum_topics(
            &self,
            chat_id: i64,
            limit: usize,
        ) -> Result<Vec<ForumTopicSummary>, ListForumTopicsSourceError> {
            *self.captured.lock().unwrap() = Some((chat_id, limit));
            self.result.clone()
        }
    }

    fn sample_topic() -> ForumTopicSummary {
        ForumTopicSummary {
            chat_id: 100,
            topic_id: 1,
            name: "General".to_owned(),
            is_general: true,
            is_closed: false,
            is_hidden: false,
            is_pinned: false,
            unread_count: 0,
            last_message_preview: None,
            last_message_unix_ms: None,
            last_message_id: None,
            order: 1000,
        }
    }

    #[test]
    fn uses_default_limit_when_query_limit_is_zero() {
        let source = StubSource::with_result(Ok(vec![]));

        let _ = list_forum_topics(
            &source,
            ListForumTopicsQuery {
                chat_id: 100,
                limit: 0,
            },
        )
        .expect("list should succeed");

        assert_eq!(
            *source.captured.lock().unwrap(),
            Some((100, DEFAULT_FORUM_TOPIC_PAGE_SIZE))
        );
    }

    #[test]
    fn passes_custom_limit_and_chat_id_through() {
        let source = StubSource::with_result(Ok(vec![]));

        let _ = list_forum_topics(
            &source,
            ListForumTopicsQuery {
                chat_id: 42,
                limit: 7,
            },
        )
        .expect("list should succeed");

        assert_eq!(*source.captured.lock().unwrap(), Some((42, 7)));
    }

    #[test]
    fn keeps_source_payload_without_mutation() {
        let topics = vec![sample_topic()];
        let source = StubSource::with_result(Ok(topics.clone()));

        let output = list_forum_topics(&source, ListForumTopicsQuery::new(100))
            .expect("list should succeed");

        assert_eq!(output.topics, topics);
    }

    #[test]
    fn maps_unauthorized_error() {
        let source = StubSource::with_result(Err(ListForumTopicsSourceError::Unauthorized));

        let err = list_forum_topics(&source, ListForumTopicsQuery::new(1)).expect_err("must fail");

        assert_eq!(err, ListForumTopicsError::Unauthorized);
    }

    #[test]
    fn maps_chat_not_found_error() {
        let source = StubSource::with_result(Err(ListForumTopicsSourceError::ChatNotFound));

        let err = list_forum_topics(&source, ListForumTopicsQuery::new(1)).expect_err("must fail");

        assert_eq!(err, ListForumTopicsError::ChatNotFound);
    }

    #[test]
    fn maps_unavailable_error_to_temporarily_unavailable() {
        let source = StubSource::with_result(Err(ListForumTopicsSourceError::Unavailable));

        let err = list_forum_topics(&source, ListForumTopicsQuery::new(1)).expect_err("must fail");

        assert_eq!(err, ListForumTopicsError::TemporarilyUnavailable);
    }

    #[test]
    fn maps_unknown_error_to_temporarily_unavailable() {
        let source = StubSource::with_result(Err(ListForumTopicsSourceError::Unknown));

        let err = list_forum_topics(&source, ListForumTopicsQuery::new(1)).expect_err("must fail");

        assert_eq!(err, ListForumTopicsError::TemporarilyUnavailable);
    }

    #[test]
    fn maps_invalid_data_error_to_contract_violation() {
        let source = StubSource::with_result(Err(ListForumTopicsSourceError::InvalidData));

        let err = list_forum_topics(&source, ListForumTopicsQuery::new(1)).expect_err("must fail");

        assert_eq!(err, ListForumTopicsError::DataContractViolation);
    }
}
