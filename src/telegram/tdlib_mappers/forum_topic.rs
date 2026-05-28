use tdlib_rs::types::ForumTopic as TdForumTopic;

use crate::domain::forum_topic::ForumTopicSummary;

use super::extract_message_preview;

/// Maps a TDLib ForumTopic to a domain ForumTopicSummary.
///
/// Names default to "General" for the implicit General topic if TDLib
/// returns an empty string.
pub fn map_forum_topic_to_summary(topic: &TdForumTopic) -> ForumTopicSummary {
    let (last_message_preview, last_message_unix_ms, last_message_id) =
        extract_last_message_info(topic);

    let name = if topic.info.name.trim().is_empty() {
        if topic.info.is_general {
            "General".to_owned()
        } else {
            "Unnamed topic".to_owned()
        }
    } else {
        topic.info.name.clone()
    };

    ForumTopicSummary {
        chat_id: topic.info.chat_id,
        topic_id: topic.info.forum_topic_id,
        name,
        is_general: topic.info.is_general,
        is_closed: topic.info.is_closed,
        is_hidden: topic.info.is_hidden,
        is_pinned: topic.is_pinned,
        unread_count: topic.unread_count.max(0) as u32,
        last_message_preview,
        last_message_unix_ms,
        last_message_id,
        order: topic.order,
    }
}

fn extract_last_message_info(topic: &TdForumTopic) -> (Option<String>, Option<i64>, Option<i64>) {
    let Some(ref msg) = topic.last_message else {
        return (None, None, None);
    };

    let preview = extract_message_preview(&msg.content);
    let timestamp_ms = i64::from(msg.date) * 1000;
    (preview, Some(timestamp_ms), Some(msg.id))
}
