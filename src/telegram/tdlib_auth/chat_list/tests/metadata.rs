use super::*;

// ---------------------------------------------------------------------------
// User metadata resolution
// ---------------------------------------------------------------------------

#[test]
fn private_chat_resolves_user_from_cache() {
    let resolver = FakeResolver::new()
        .with_cached_chat(make_test_chat(1, "Alice"))
        .with_cached_user(make_online_user(1, "Alice"));

    let summaries = build_summaries_from_ids(&resolver, vec![1], NO_FORCE);

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].is_online, Some(true));
    assert_eq!(
        resolver.get_user_call_count(),
        0,
        "user resolved from cache"
    );
}

#[test]
fn private_chat_falls_back_to_get_user_on_cache_miss() {
    let resolver = FakeResolver::new()
        .with_cached_chat(make_test_chat(1, "Bob"))
        .with_tdlib_user(make_online_user(1, "Bob"));

    let summaries = build_summaries_from_ids(&resolver, vec![1], NO_FORCE);

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].is_online, Some(true));
    assert_eq!(resolver.get_user_call_count(), 1);
}

#[test]
fn private_chat_user_fallback_populates_cache() {
    let resolver = FakeResolver::new()
        .with_cached_chat(make_test_chat(1, "Charlie"))
        .with_tdlib_user(make_online_user(1, "Charlie"));

    let _ = build_summaries_from_ids(&resolver, vec![1], NO_FORCE);

    assert!(
        resolver.cache.get_user(1).is_some(),
        "user should be cached after get_user fallback"
    );
}

#[test]
fn private_chat_missing_user_returns_none_online() {
    let resolver = FakeResolver::new().with_cached_chat(make_test_chat(1, "Unknown User"));

    let summaries = build_summaries_from_ids(&resolver, vec![1], NO_FORCE);

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].is_online, None);
}

// ---------------------------------------------------------------------------
// Group chat sender name resolution
// ---------------------------------------------------------------------------

#[test]
fn group_chat_resolves_sender_name() {
    let mut group_chat = make_group_chat(100, "Dev Team");
    group_chat.last_message = Some(make_message_from_user(42));

    let resolver = FakeResolver::new()
        .with_cached_chat(group_chat)
        .with_cached_user(make_test_user(42, "Sender"));

    let summaries = build_summaries_from_ids(&resolver, vec![100], NO_FORCE);

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].last_message_sender.as_deref(), Some("Sender"));
    assert_eq!(summaries[0].chat_type, ChatType::Group);
}

#[test]
fn group_chat_missing_sender_returns_none() {
    let mut group_chat = make_group_chat(100, "Dev Team");
    group_chat.last_message = Some(make_message_from_user(999));

    let resolver = FakeResolver::new().with_cached_chat(group_chat);

    let summaries = build_summaries_from_ids(&resolver, vec![100], NO_FORCE);

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].last_message_sender, None);
}

// ---------------------------------------------------------------------------
// Bot detection
// ---------------------------------------------------------------------------

#[test]
fn private_chat_detects_bot() {
    let resolver = FakeResolver::new()
        .with_cached_chat(make_test_chat(1, "Bot Chat"))
        .with_cached_user(make_bot_user(1, "HelpBot"));

    let summaries = build_summaries_from_ids(&resolver, vec![1], NO_FORCE);

    assert_eq!(summaries.len(), 1);
    assert!(summaries[0].is_bot);
}

// ---------------------------------------------------------------------------
// Supergroup forum flag
// ---------------------------------------------------------------------------

#[test]
fn supergroup_chat_carries_is_forum_from_cache() {
    use crate::telegram::tdlib_cache::tests::make_test_supergroup;

    let chat = make_supergroup_chat(10, 100, "Topics");
    let resolver = FakeResolver::new().with_cached_chat(chat);
    resolver
        .cache
        .upsert_supergroup(make_test_supergroup(100, true));

    let summaries = build_summaries_from_ids(&resolver, vec![10], NO_FORCE);

    assert_eq!(summaries.len(), 1);
    assert!(summaries[0].is_forum);
}

#[test]
fn supergroup_chat_is_forum_defaults_false_when_supergroup_missing() {
    let chat = make_supergroup_chat(10, 100, "No cache");
    let resolver = FakeResolver::new().with_cached_chat(chat);

    let summaries = build_summaries_from_ids(&resolver, vec![10], NO_FORCE);

    assert_eq!(summaries.len(), 1);
    assert!(!summaries[0].is_forum);
}

#[test]
fn non_supergroup_chat_is_never_forum() {
    let resolver = FakeResolver::new().with_cached_chat(make_group_chat(11, "Plain group"));

    let summaries = build_summaries_from_ids(&resolver, vec![11], NO_FORCE);

    assert_eq!(summaries.len(), 1);
    assert!(!summaries[0].is_forum);
}

// ---------------------------------------------------------------------------
// Forum unread-topic count: the badge counts unread topics, not messages,
// and is derived from the seeded topic cache — never fetched on this path
// ---------------------------------------------------------------------------

#[test]
fn forum_badge_counts_unread_topics_from_cache() {
    use crate::telegram::tdlib_cache::tests::make_test_supergroup;

    // chat-level unread_count (42) is the stale message total; the real unread
    // is two topics with 1 and 7 messages — the badge must read 2.
    let resolver = FakeResolver::new().with_cached_chat(make_unread_forum_chat(10, 100, 42));
    resolver
        .cache
        .upsert_supergroup(make_test_supergroup(100, true));
    resolver.cache.seed_forum_topics(
        10,
        &[
            make_test_forum_topic(1, 1),
            make_test_forum_topic(2, 7),
            make_test_forum_topic(3, 0),
        ],
    );

    let summaries = build_summaries_from_ids(&resolver, vec![10], NO_FORCE);

    assert_eq!(summaries.len(), 1);
    assert!(summaries[0].is_forum);
    assert_eq!(summaries[0].unread_count, 42);
    assert_eq!(summaries[0].unread_topic_count, Some(2));
}

#[test]
fn unseeded_forum_leaves_count_none() {
    use crate::telegram::tdlib_cache::tests::make_test_supergroup;

    // Topic cache not seeded yet: the badge stays unknown (hidden) instead of
    // falling back to the misleading chat-level message count; the background
    // warm-up delivers the real value later.
    let resolver = FakeResolver::new().with_cached_chat(make_unread_forum_chat(10, 100, 42));
    resolver
        .cache
        .upsert_supergroup(make_test_supergroup(100, true));

    let summaries = build_summaries_from_ids(&resolver, vec![10], NO_FORCE);

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].unread_topic_count, None);
}

#[test]
fn seeded_read_forum_reports_zero_unread_topics() {
    use crate::telegram::tdlib_cache::tests::make_test_supergroup;

    let resolver = FakeResolver::new().with_cached_chat(make_unread_forum_chat(10, 100, 0));
    resolver
        .cache
        .upsert_supergroup(make_test_supergroup(100, true));
    resolver
        .cache
        .seed_forum_topics(10, &[make_test_forum_topic(1, 0)]);

    let summaries = build_summaries_from_ids(&resolver, vec![10], NO_FORCE);

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].unread_topic_count, Some(0));
}

#[test]
fn non_forum_chat_has_no_unread_topic_count() {
    let mut chat = make_test_chat(1, "Plain");
    chat.unread_count = 9;
    let resolver = FakeResolver::new().with_cached_chat(chat);
    // Even a (bogus) seeded entry must not leak onto non-forum chats.
    resolver
        .cache
        .seed_forum_topics(1, &[make_test_forum_topic(1, 1)]);

    let summaries = build_summaries_from_ids(&resolver, vec![1], NO_FORCE);

    assert_eq!(summaries[0].unread_count, 9);
    assert_eq!(summaries[0].unread_topic_count, None);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_bot_user(id: i64, name: &str) -> User {
    let mut user = make_test_user(id, name);
    user.r#type = tdlib_rs::enums::UserType::Bot(tdlib_rs::types::UserTypeBot {
        can_be_edited: false,
        can_join_groups: false,
        can_read_all_group_messages: false,
        has_main_web_app: false,
        has_topics: false,
        allows_users_to_create_topics: false,
        is_inline: false,
        inline_query_placeholder: String::new(),
        need_location: false,
        can_connect_to_business: false,
        can_be_added_to_attachment_menu: false,
        active_user_count: 0,
    });
    user
}
