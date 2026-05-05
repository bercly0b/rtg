use crate::domain::chat::{ChatSummary, ChatType, OutgoingReadStatus};

use super::{
    super::chat_list_item, channel_chat_outgoing, chat, group_chat, group_chat_outgoing,
    line_to_string, private_chat_online, private_chat_outgoing,
};

const TEST_WIDTH: usize = 50;

#[test]
fn chat_list_item_includes_title_and_preview() {
    let line =
        chat_list_item::chat_list_item_line(&chat(1, "General", 0, Some("Hello")), TEST_WIDTH);
    let text = line_to_string(&line);

    assert!(text.contains("General"));
    assert!(text.contains("Hello"));
}

#[test]
fn chat_list_item_includes_unread_counter() {
    let line =
        chat_list_item::chat_list_item_line(&chat(1, "General", 3, Some("Hello")), TEST_WIDTH);
    let text = line_to_string(&line);

    assert!(text.contains("[3]"));
}

#[test]
fn chat_list_item_omits_counter_when_zero() {
    let line =
        chat_list_item::chat_list_item_line(&chat(1, "General", 0, Some("Hello")), TEST_WIDTH);
    let text = line_to_string(&line);

    assert!(!text.contains("[0]"));
    assert!(!text.contains("[]"));
}

#[test]
fn chat_list_item_falls_back_to_placeholder_preview() {
    let line =
        chat_list_item::chat_list_item_line(&chat(1, "General", 0, Some("  \n\t  ")), TEST_WIDTH);
    let text = line_to_string(&line);

    assert!(text.contains("No messages yet"));
}

#[test]
fn chat_list_item_normalizes_whitespace() {
    let line = chat_list_item::chat_list_item_line(
        &chat(1, "General", 0, Some("  Hello\n\n  from\t\tRTG   ")),
        TEST_WIDTH,
    );
    let text = line_to_string(&line);

    assert!(text.contains("Hello from RTG"));
}

#[test]
fn group_chat_shows_sender_name_before_preview() {
    let line = chat_list_item::chat_list_item_line(
        &group_chat(1, "Dev Team", Some("Fixed the bug"), Some("Alex")),
        TEST_WIDTH,
    );
    let text = line_to_string(&line);

    assert!(text.contains("Dev Team"));
    assert!(text.contains("Alex: "));
    assert!(text.contains("Fixed the bug"));
}

#[test]
fn group_chat_without_sender_shows_plain_preview() {
    let line = chat_list_item::chat_list_item_line(
        &group_chat(1, "Dev Team", Some("Hello everyone"), None),
        TEST_WIDTH,
    );
    let text = line_to_string(&line);

    assert!(text.contains("Hello everyone"));
    assert!(!text.contains(": "));
}

#[test]
fn group_chat_outgoing_delivered_shows_single_check_after_preview() {
    let line = chat_list_item::chat_list_item_line(
        &group_chat_outgoing(1, "Dev Team", Some("I fixed it"), Some("You"), false),
        70,
    );
    let text = line_to_string(&line);

    assert!(text.contains("Dev Team"));
    assert!(text.contains("You: "));
    assert!(text.contains(" \u{2713}"));
    assert!(!text.contains("\u{2713}\u{2713}"));
    assert!(text.contains("I fixed it"));
    let preview_pos = text.find("I fixed it").unwrap();
    let check_pos = text.find("\u{2713}").unwrap();
    assert!(
        preview_pos < check_pos,
        "Preview should come before status indicator"
    );
}

#[test]
fn group_chat_outgoing_read_shows_double_check_after_preview() {
    let line = chat_list_item::chat_list_item_line(
        &group_chat_outgoing(1, "Dev Team", Some("Done"), Some("You"), true),
        70,
    );
    let text = line_to_string(&line);

    assert!(text.contains("Dev Team"));
    assert!(text.contains("You: "));
    assert!(text.contains(" \u{2713}\u{2713}"));
    assert!(text.contains("Done"));
    let preview_pos = text.find("Done").unwrap();
    let check_pos = text.find("\u{2713}").unwrap();
    assert!(
        preview_pos < check_pos,
        "Preview should come before status indicator"
    );
}

#[test]
fn group_chat_outgoing_narrow_width_still_shows_status() {
    let line = chat_list_item::chat_list_item_line(
        &group_chat_outgoing(1, "Dev Team", Some("I fixed the bug"), Some("Alex"), true),
        34,
    );
    let text = line_to_string(&line);

    assert!(text.contains("Dev Team"));
    assert!(text.contains("Alex: "));
    assert!(
        text.contains("\u{2713}"),
        "Status indicator must be present even at narrow width. Got: '{}'",
        text
    );
}

#[test]
fn group_chat_emoji_in_sender_name_shows_status_indicator() {
    let line = chat_list_item::chat_list_item_line(
        &group_chat_outgoing(1, "Group", Some("hello"), Some("\u{1F680} vlad"), true),
        40,
    );
    let text = line_to_string(&line);

    assert!(
        text.contains("\u{2713}"),
        "Status indicator must be present with emoji sender. Got: '{}'",
        text
    );
}

#[test]
fn group_chat_emoji_in_title_shows_status_indicator() {
    let line = chat_list_item::chat_list_item_line(
        &group_chat_outgoing(1, "\u{1F525} Fire Chat", Some("done"), Some("Alex"), true),
        50,
    );
    let text = line_to_string(&line);

    assert!(
        text.contains("\u{2713}"),
        "Status indicator must be present with emoji title. Got: '{}'",
        text
    );
}

#[test]
fn channel_does_not_render_sender_prefix() {
    let c = ChatSummary {
        chat_id: 1,
        title: "My Channel".to_owned(),
        unread_count: 0,
        last_message_preview: Some("Post".to_owned()),
        last_message_unix_ms: None,
        is_pinned: false,
        chat_type: ChatType::Channel,
        last_message_sender: Some("Author".to_owned()),
        is_online: None,
        is_bot: false,
        outgoing_status: OutgoingReadStatus::default(),
        last_message_id: None,
        unread_reaction_count: 0,
    };

    let line = chat_list_item::chat_list_item_line(&c, TEST_WIDTH);
    let text = line_to_string(&line);

    assert!(text.contains("Post"));
    assert!(
        !text.contains("Author:"),
        "channel chats must not render sender prefix; got: '{}'",
        text
    );
}

#[test]
fn channel_outgoing_shows_read_indicator() {
    let line = chat_list_item::chat_list_item_line(
        &channel_chat_outgoing(1, "My Channel", Some("New post"), true),
        TEST_WIDTH,
    );
    let text = line_to_string(&line);

    assert!(text.contains("My Channel"));
    assert!(text.contains(" \u{2713}\u{2713}"));
    assert!(text.contains("New post"));
}

#[test]
fn channel_outgoing_delivered_shows_single_check() {
    let line = chat_list_item::chat_list_item_line(
        &channel_chat_outgoing(1, "My Channel", Some("Draft post"), false),
        TEST_WIDTH,
    );
    let text = line_to_string(&line);

    assert!(text.contains("My Channel"));
    assert!(text.contains(" \u{2713}"));
    assert!(!text.contains("\u{2713}\u{2713}"));
    assert!(text.contains("Draft post"));
}

#[test]
fn private_chat_online_shows_bullet() {
    let line = chat_list_item::chat_list_item_line(
        &private_chat_online(1, "John", Some("Hey there"), true),
        TEST_WIDTH,
    );
    let text = line_to_string(&line);

    assert!(text.contains("John"));
    assert!(text.contains("Hey there"));
    assert!(text.contains("\u{2022}"));
}

#[test]
fn private_chat_offline_no_bullet() {
    let line = chat_list_item::chat_list_item_line(
        &private_chat_online(1, "John", Some("Hey there"), false),
        TEST_WIDTH,
    );
    let text = line_to_string(&line);

    assert!(text.contains("John"));
    assert!(!text.contains("\u{2022}"));
}

#[test]
fn private_chat_outgoing_delivered_shows_single_check() {
    let line = chat_list_item::chat_list_item_line(
        &private_chat_outgoing(1, "Jane", Some("See you tomorrow"), false),
        TEST_WIDTH,
    );
    let text = line_to_string(&line);

    assert!(text.contains("Jane"));
    assert!(text.contains(" \u{2713}"));
    assert!(!text.contains("\u{2713}\u{2713}"));
    assert!(text.contains("See you tomorrow"));
}

#[test]
fn private_chat_outgoing_read_shows_double_check() {
    let line = chat_list_item::chat_list_item_line(
        &private_chat_outgoing(1, "Jane", Some("Got it"), true),
        TEST_WIDTH,
    );
    let text = line_to_string(&line);

    assert!(text.contains("Jane"));
    assert!(text.contains(" \u{2713}\u{2713}"));
    assert!(text.contains("Got it"));
}

#[test]
fn private_chat_incoming_message_no_indicator() {
    let line = chat_list_item::chat_list_item_line(&chat(1, "Bob", 0, Some("Hello!")), TEST_WIDTH);
    let text = line_to_string(&line);

    assert!(text.contains("Hello!"));
    assert!(!text.contains("\u{2713}"));
}

#[test]
fn chat_with_unread_and_online_shows_both() {
    let c = ChatSummary {
        chat_id: 1,
        title: "Alice".to_owned(),
        unread_count: 5,
        last_message_preview: Some("New message".to_owned()),
        last_message_unix_ms: None,
        is_pinned: false,
        chat_type: ChatType::Private,
        last_message_sender: None,
        is_online: Some(true),
        is_bot: false,
        outgoing_status: OutgoingReadStatus::default(),
        last_message_id: None,
        unread_reaction_count: 0,
    };

    let line = chat_list_item::chat_list_item_line(&c, 70);
    let text = line_to_string(&line);

    assert!(text.contains("[5]"));
    assert!(text.contains("\u{2022}"));
}

#[test]
fn bot_chat_online_does_not_show_online_indicator() {
    let c = ChatSummary {
        chat_id: 1,
        title: "BotName".to_owned(),
        unread_count: 0,
        last_message_preview: Some("Hello".to_owned()),
        last_message_unix_ms: None,
        is_pinned: false,
        chat_type: ChatType::Private,
        last_message_sender: None,
        is_online: Some(true),
        is_bot: true,
        outgoing_status: OutgoingReadStatus::default(),
        last_message_id: None,
        unread_reaction_count: 0,
    };

    let line = chat_list_item::chat_list_item_line(&c, 70);
    let text = line_to_string(&line);

    assert!(
        !text.contains("\u{2022}"),
        "online bullet must not appear for bots"
    );
}

// -- reaction badge tests --

#[test]
fn chat_with_unread_reactions_shows_heart_badge() {
    let c = ChatSummary {
        chat_id: 1,
        title: "Alice".to_owned(),
        unread_count: 0,
        last_message_preview: Some("Hello".to_owned()),
        last_message_unix_ms: None,
        is_pinned: false,
        chat_type: ChatType::Private,
        last_message_sender: None,
        is_online: None,
        is_bot: false,
        outgoing_status: OutgoingReadStatus::default(),
        last_message_id: None,
        unread_reaction_count: 2,
    };

    let line = chat_list_item::chat_list_item_line(&c, 70);
    let text = line_to_string(&line);

    assert!(
        text.contains("[\u{2661}]"),
        "expected heart badge in: {text}"
    );
}

#[test]
fn chat_without_unread_reactions_has_no_heart_badge() {
    let c = chat(1, "Bob", 0, Some("Hi"));
    let line = chat_list_item::chat_list_item_line(&c, 70);
    let text = line_to_string(&line);

    assert!(
        !text.contains("\u{2661}"),
        "heart badge must not appear when unread_reaction_count is 0: {text}"
    );
}

#[test]
fn chat_with_reactions_and_unread_shows_both_badges() {
    let c = ChatSummary {
        chat_id: 1,
        title: "Alice".to_owned(),
        unread_count: 3,
        last_message_preview: Some("Hello".to_owned()),
        last_message_unix_ms: None,
        is_pinned: false,
        chat_type: ChatType::Private,
        last_message_sender: None,
        is_online: None,
        is_bot: false,
        outgoing_status: OutgoingReadStatus::default(),
        last_message_id: None,
        unread_reaction_count: 1,
    };

    let line = chat_list_item::chat_list_item_line(&c, 80);
    let text = line_to_string(&line);

    assert!(
        text.contains("[\u{2661}]"),
        "expected heart badge in: {text}"
    );
    assert!(text.contains("[3]"), "expected unread badge in: {text}");
}
