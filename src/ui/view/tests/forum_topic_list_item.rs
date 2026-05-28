use crate::domain::forum_topic::ForumTopicSummary;
use crate::ui::view::forum_topic_list_item;

use super::line_to_string;

const TEST_WIDTH: usize = 50;

fn topic(name: &str) -> ForumTopicSummary {
    ForumTopicSummary {
        chat_id: 100,
        topic_id: 7,
        name: name.to_owned(),
        is_general: false,
        is_closed: false,
        is_hidden: false,
        is_pinned: false,
        unread_count: 0,
        last_message_preview: Some("hello".to_owned()),
        last_message_unix_ms: None,
        last_message_id: None,
        order: 100,
    }
}

#[test]
fn renders_topic_name_and_preview() {
    let t = topic("Backend");

    let line = forum_topic_list_item::forum_topic_list_item_line(&t, TEST_WIDTH);
    let text = line_to_string(&line);

    assert!(text.contains("Backend"));
    assert!(text.contains("hello"));
}

#[test]
fn renders_unread_badge_when_unread() {
    let mut t = topic("Backend");
    t.unread_count = 4;

    let line = forum_topic_list_item::forum_topic_list_item_line(&t, TEST_WIDTH);
    let text = line_to_string(&line);

    assert!(text.contains("[4]"));
}

#[test]
fn omits_unread_badge_when_zero() {
    let t = topic("Backend");

    let line = forum_topic_list_item::forum_topic_list_item_line(&t, TEST_WIDTH);
    let text = line_to_string(&line);

    assert!(!text.contains('['));
}

#[test]
fn renders_closed_marker() {
    let mut t = topic("Old discussion");
    t.is_closed = true;

    let line = forum_topic_list_item::forum_topic_list_item_line(&t, TEST_WIDTH);
    let text = line_to_string(&line);

    assert!(text.contains("[closed]"));
}

#[test]
fn renders_hidden_marker_in_priority_over_closed() {
    let mut t = topic("General");
    t.is_general = true;
    t.is_closed = true;
    t.is_hidden = true;

    let line = forum_topic_list_item::forum_topic_list_item_line(&t, TEST_WIDTH);
    let text = line_to_string(&line);

    assert!(text.contains("[hidden]"));
    assert!(!text.contains("[closed]"));
}

#[test]
fn renders_no_messages_placeholder_when_preview_absent() {
    let mut t = topic("Empty");
    t.last_message_preview = None;

    let line = forum_topic_list_item::forum_topic_list_item_line(&t, TEST_WIDTH);
    let text = line_to_string(&line);

    assert!(text.contains("No messages yet"));
}
