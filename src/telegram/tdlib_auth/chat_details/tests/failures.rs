//! Only the requests behind the header fail the popup.

use super::*;

#[test]
fn get_chat_failure_returns_error() {
    let resolver = FakeResolver::default();

    let result = resolve(&resolver, 1, ChatType::Private);

    assert_eq!(result, Err(ChatSubtitleError::Unavailable));
}

#[test]
fn private_user_failure_returns_error() {
    let mut resolver = FakeResolver::default();
    resolver.chats.insert(1, private_chat(1, 1));

    let result = resolve(&resolver, 1, ChatType::Private);

    assert_eq!(result, Err(ChatSubtitleError::Unavailable));
}

#[test]
fn basic_group_full_info_failure_returns_error() {
    let mut resolver = FakeResolver::default();
    resolver.chats.insert(5, basic_group_chat(5, 50));
    resolver
        .basic_groups
        .insert(50, basic_group(50, member_status()));

    let result = resolve(&resolver, 5, ChatType::Group);

    assert_eq!(result, Err(ChatSubtitleError::Unavailable));
}

#[test]
fn supergroup_full_info_failure_returns_error() {
    let mut resolver = FakeResolver::default();
    resolver.chats.insert(6, supergroup_chat(6, 60, false));
    resolver.supergroups.insert(60, supergroup(60, false));

    let result = resolve(&resolver, 6, ChatType::Group);

    assert_eq!(result, Err(ChatSubtitleError::Unavailable));
}

#[test]
fn channel_full_info_failure_returns_error() {
    let mut resolver = FakeResolver::default();
    resolver.chats.insert(7, supergroup_chat(7, 70, true));
    resolver.supergroups.insert(70, supergroup(70, true));

    let result = resolve(&resolver, 7, ChatType::Channel);

    assert_eq!(result, Err(ChatSubtitleError::Unavailable));
}

#[test]
fn chat_type_mismatch_returns_error() {
    let mut resolver = FakeResolver::default();
    resolver.chats.insert(1, private_chat(1, 1));

    let result = resolve(&resolver, 1, ChatType::Group);

    assert_eq!(result, Err(ChatSubtitleError::Unavailable));
}

#[test]
fn optional_request_failures_still_resolve_every_template() {
    let mut resolver = FakeResolver::default();
    let mut user_chat = private_chat(1, 1);
    user_chat.notification_settings.use_default_mute_for = true;
    resolver.chats.insert(1, user_chat);
    resolver.users.insert(1, user(1));
    resolver.chats.insert(2, private_chat(2, 2));
    resolver.users.insert(2, bot_user(2, bot_type()));
    resolver.chats.insert(5, basic_group_chat(5, 50));
    resolver
        .basic_group_full_infos
        .insert(50, basic_group_full_info(vec![], ""));
    resolver.chats.insert(6, supergroup_chat(6, 60, false));
    resolver
        .supergroup_full_infos
        .insert(60, supergroup_full_info(3, ""));
    resolver.chats.insert(7, supergroup_chat(7, 70, true));
    resolver
        .supergroup_full_infos
        .insert(70, supergroup_full_info(9, ""));

    assert!(resolve(&resolver, 1, ChatType::Private).is_ok());
    assert!(resolve(&resolver, 2, ChatType::Private).is_ok());
    assert!(resolve(&resolver, 5, ChatType::Group).is_ok());
    assert!(resolve(&resolver, 6, ChatType::Group).is_ok());
    assert!(resolve(&resolver, 7, ChatType::Channel).is_ok());
}
