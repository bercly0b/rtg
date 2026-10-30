use super::*;

// ---------------------------------------------------------------------------
// Cache-first lookup order
// ---------------------------------------------------------------------------

#[test]
fn cache_hit_skips_get_chat() {
    let resolver = FakeResolver::new().with_cached_chat(make_test_chat(1, "Cached Chat"));

    let summaries = build_summaries_from_ids(&resolver, vec![1], NO_FORCE);

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].chat_id, 1);
    assert_eq!(summaries[0].title, "Cached Chat");
    assert_eq!(
        resolver.get_chat_call_count(),
        0,
        "get_chat should not be called on cache hit"
    );
}

#[test]
fn cache_miss_falls_back_to_get_chat() {
    let resolver = FakeResolver::new().with_tdlib_chat(make_test_chat(2, "TDLib Chat"));

    let summaries = build_summaries_from_ids(&resolver, vec![2], NO_FORCE);

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].chat_id, 2);
    assert_eq!(summaries[0].title, "TDLib Chat");
    assert_eq!(resolver.get_chat_call_count(), 1);
}

#[test]
fn cache_miss_fallback_populates_cache() {
    let resolver = FakeResolver::new().with_tdlib_chat(make_test_chat(3, "Backfilled"));

    let _ = build_summaries_from_ids(&resolver, vec![3], NO_FORCE);

    let cached = resolver.cache.get_chat(3);
    assert!(
        cached.is_some(),
        "cache should be populated after get_chat fallback"
    );
    assert_eq!(cached.unwrap().title, "Backfilled");
}

#[test]
fn mixed_cache_hit_and_miss() {
    let resolver = FakeResolver::new()
        .with_cached_chat(make_test_chat(1, "From Cache"))
        .with_tdlib_chat(make_test_chat(2, "From TDLib"));

    let summaries = build_summaries_from_ids(&resolver, vec![1, 2], NO_FORCE);

    assert_eq!(summaries.len(), 2);
    assert_eq!(summaries[0].title, "From Cache");
    assert_eq!(summaries[1].title, "From TDLib");
    assert_eq!(
        resolver.get_chat_call_count(),
        1,
        "only cache-miss chat triggers get_chat"
    );
}

// ---------------------------------------------------------------------------
// Stale cache scenario (the original bug context)
// ---------------------------------------------------------------------------

#[test]
fn stale_cache_serves_cached_data_without_tdlib_call() {
    // Simulates the core scenario: update loop populated cache with stale
    // data (e.g. updateChatLastMessage dropped by tdlib-rs for messageGift).
    // build_summaries_from_ids reads cached (stale) data instantly without
    // calling get_chat — the fast path that keeps startup quick.
    let mut stale_chat = make_test_chat(1, "Chat With Gift");
    stale_chat.unread_count = 5;

    let resolver = FakeResolver::new().with_cached_chat(stale_chat);

    let summaries = build_summaries_from_ids(&resolver, vec![1], NO_FORCE);

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].unread_count, 5);
    assert_eq!(
        resolver.get_chat_call_count(),
        0,
        "cache hit means no TDLib call — fast path preserved"
    );
}

// ---------------------------------------------------------------------------
// Ordering preservation
// ---------------------------------------------------------------------------

#[test]
fn preserves_input_order() {
    let resolver = FakeResolver::new()
        .with_cached_chat(make_test_chat(3, "Third"))
        .with_cached_chat(make_test_chat(1, "First"))
        .with_cached_chat(make_test_chat(2, "Second"));

    let summaries = build_summaries_from_ids(&resolver, vec![1, 2, 3], NO_FORCE);

    let ids: Vec<i64> = summaries.iter().map(|s| s.chat_id).collect();
    assert_eq!(ids, vec![1, 2, 3]);
}

// ---------------------------------------------------------------------------
// Force refresh: bypasses cache, reads from TDLib directly
// ---------------------------------------------------------------------------

#[test]
fn force_bypasses_cache_and_calls_get_chat() {
    let resolver = FakeResolver::new()
        .with_cached_chat(make_test_chat(1, "Stale Cache"))
        .with_tdlib_chat(make_test_chat(1, "Fresh TDLib"));

    let summaries = build_summaries_from_ids(&resolver, vec![1], true);

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].title, "Fresh TDLib");
    assert_eq!(
        resolver.get_chat_call_count(),
        1,
        "force=true must call get_chat even when cache has data"
    );
}

#[test]
fn force_populates_cache_from_tdlib() {
    let resolver = FakeResolver::new().with_tdlib_chat(make_test_chat(1, "From TDLib"));

    let _ = build_summaries_from_ids(&resolver, vec![1], true);

    let cached = resolver.cache.get_chat(1);
    assert!(cached.is_some(), "force path should populate cache");
    assert_eq!(cached.unwrap().title, "From TDLib");
}

#[test]
fn force_skips_chat_on_tdlib_failure() {
    let resolver = FakeResolver::new().with_cached_chat(make_test_chat(1, "Cached Only"));

    let summaries = build_summaries_from_ids(&resolver, vec![1], true);

    // Chat is in cache but NOT in tdlib_chats HashMap — force path fails
    assert!(
        summaries.is_empty(),
        "force=true should not fall back to cache on TDLib failure"
    );
    assert_eq!(resolver.get_chat_call_count(), 1);
}

#[test]
fn no_force_uses_cache_first() {
    let resolver = FakeResolver::new()
        .with_cached_chat(make_test_chat(1, "From Cache"))
        .with_tdlib_chat(make_test_chat(1, "From TDLib"));

    let summaries = build_summaries_from_ids(&resolver, vec![1], false);

    assert_eq!(summaries[0].title, "From Cache");
    assert_eq!(
        resolver.get_chat_call_count(),
        0,
        "force=false should use cache"
    );
}

// ---------------------------------------------------------------------------
// First-paint critical path invariant
// ---------------------------------------------------------------------------

/// Guards the chat-list fast path: with a warm cache, building summaries makes
/// no TDLib calls at all. This code runs before the first paint of the chat
/// list, so any call here is at best an SQLite read and at worst a server
/// round-trip — the `getForumTopics` regression of PR #156 kept the startup
/// loader on screen for seconds exactly this way. When a new method is added
/// to `ChatDataResolver`, wire its call counter into this test and keep every
/// expected count at zero.
#[test]
fn warm_cache_first_load_makes_no_tdlib_calls() {
    use crate::telegram::tdlib_cache::tests::make_test_supergroup;

    // One chat of every shape that triggers extra resolution: a private chat
    // (online status + bot flag), a group with a last-message sender, and an
    // unread forum (unread-topic badge).
    let private_chat = make_test_chat(1, "Alice");
    let mut group_chat = make_group_chat(2, "Dev Team");
    group_chat.last_message = Some(make_message_from_user(42));
    let forum_chat = make_unread_forum_chat(3, 100, 42);

    let resolver = FakeResolver::new()
        .with_cached_chat(private_chat)
        .with_cached_chat(group_chat)
        .with_cached_chat(forum_chat)
        .with_cached_user(make_online_user(1, "Alice"))
        .with_cached_user(make_test_user(42, "Sender"));
    resolver
        .cache
        .upsert_supergroup(make_test_supergroup(100, true));
    resolver
        .cache
        .seed_forum_topics(3, &[make_test_forum_topic(1, 5)]);

    let summaries = build_summaries_from_ids(&resolver, vec![1, 2, 3], NO_FORCE);

    assert_eq!(summaries.len(), 3, "all chats resolved from cache");
    assert_eq!(summaries[2].unread_topic_count, Some(1));
    assert_eq!(
        resolver.get_chat_call_count(),
        0,
        "warm first load must not call get_chat"
    );
    assert_eq!(
        resolver.get_user_call_count(),
        0,
        "warm first load must not call get_user"
    );
}
