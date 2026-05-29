use crate::domain::forum_topic::ForumTopicSummary;
use crate::telegram::tdlib_mappers::map_forum_topic_to_summary;
use crate::usecases::list_forum_topics::ListForumTopicsSourceError;

use super::error_mapping::map_forum_topics_error;
use super::TdLibAuthBackend;

impl TdLibAuthBackend {
    /// Lists forum topic summaries for a forum supergroup chat.
    pub fn list_forum_topic_summaries(
        &self,
        chat_id: i64,
        limit: usize,
    ) -> Result<Vec<ForumTopicSummary>, ListForumTopicsSourceError> {
        let limit_i32 = i32::try_from(limit).unwrap_or(i32::MAX);

        let topics = self
            .client
            .get_forum_topics(chat_id, limit_i32)
            .map_err(map_forum_topics_error)?;

        tracing::debug!(chat_id, count = topics.len(), "Fetched forum topics");

        Ok(topics.iter().map(map_forum_topic_to_summary).collect())
    }
}
