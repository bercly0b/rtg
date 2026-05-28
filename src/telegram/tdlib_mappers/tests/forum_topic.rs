use tdlib_rs::types::{ForumTopic, ForumTopicIcon, ForumTopicInfo};

use crate::telegram::tdlib_mappers::map_forum_topic_to_summary;

fn topic_info(forum_topic_id: i32, name: &str, is_general: bool) -> ForumTopicInfo {
    ForumTopicInfo {
        chat_id: 100,
        forum_topic_id,
        name: name.to_owned(),
        icon: ForumTopicIcon {
            color: 0,
            custom_emoji_id: 0,
        },
        creation_date: 0,
        creator_id: tdlib_rs::enums::MessageSender::User(tdlib_rs::types::MessageSenderUser {
            user_id: 0,
        }),
        is_general,
        is_outgoing: false,
        is_closed: false,
        is_hidden: false,
        is_name_implicit: false,
    }
}

fn forum_topic(info: ForumTopicInfo, order: i64) -> ForumTopic {
    ForumTopic {
        info,
        last_message: None,
        order,
        is_pinned: false,
        unread_count: 0,
        last_read_inbox_message_id: 0,
        last_read_outbox_message_id: 0,
        unread_mention_count: 0,
        unread_reaction_count: 0,
        notification_settings: tdlib_rs::types::ChatNotificationSettings {
            use_default_mute_for: true,
            mute_for: 0,
            use_default_sound: true,
            sound_id: 0,
            use_default_show_preview: true,
            show_preview: false,
            use_default_mute_stories: true,
            mute_stories: false,
            use_default_story_sound: true,
            story_sound_id: 0,
            use_default_show_story_poster: true,
            show_story_poster: false,
            use_default_disable_pinned_message_notifications: true,
            disable_pinned_message_notifications: false,
            use_default_disable_mention_notifications: true,
            disable_mention_notifications: false,
        },
        draft_message: None,
    }
}

#[test]
fn maps_basic_fields() {
    let topic = forum_topic(topic_info(7, "Backend", false), 500);

    let summary = map_forum_topic_to_summary(&topic);

    assert_eq!(summary.chat_id, 100);
    assert_eq!(summary.topic_id, 7);
    assert_eq!(summary.name, "Backend");
    assert!(!summary.is_general);
    assert!(!summary.is_closed);
    assert_eq!(summary.order, 500);
}

#[test]
fn falls_back_to_general_name_when_topic_is_general_and_name_empty() {
    let topic = forum_topic(topic_info(1, "", true), 1_000_000);

    let summary = map_forum_topic_to_summary(&topic);

    assert_eq!(summary.name, "General");
    assert!(summary.is_general);
}

#[test]
fn falls_back_to_unnamed_placeholder_for_non_general_empty_name() {
    let topic = forum_topic(topic_info(2, "", false), 100);

    let summary = map_forum_topic_to_summary(&topic);

    assert_eq!(summary.name, "Unnamed topic");
}

#[test]
fn carries_closed_and_hidden_flags() {
    let mut info = topic_info(3, "Old", false);
    info.is_closed = true;
    let mut topic = forum_topic(info, 50);
    topic.unread_count = 4;

    let summary = map_forum_topic_to_summary(&topic);

    assert!(summary.is_closed);
    assert_eq!(summary.unread_count, 4);
}
