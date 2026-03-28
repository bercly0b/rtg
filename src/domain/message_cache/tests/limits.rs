use super::*;

// ── Per-chat message limit tests ──

#[test]
fn put_truncates_to_max_messages_per_chat() {
    let mut cache = MessageCache::new(50, 3);

    cache.put(
        1,
        vec![
            msg_with_ts(1, "oldest", 1000),
            msg_with_ts(2, "old", 2000),
            msg_with_ts(3, "mid", 3000),
            msg_with_ts(4, "new", 4000),
            msg_with_ts(5, "newest", 5000),
        ],
        true,
    );

    let cached = cache.get(1).unwrap();
    assert_eq!(cached.len(), 3, "should keep only 3 newest messages");
    assert_eq!(cached[0].text, "mid");
    assert_eq!(cached[1].text, "new");
    assert_eq!(cached[2].text, "newest");
}

#[test]
fn add_message_respects_max_messages_per_chat() {
    let mut cache = MessageCache::new(50, 3);

    cache.put(
        1,
        vec![
            msg_with_ts(1, "a", 1000),
            msg_with_ts(2, "b", 2000),
            msg_with_ts(3, "c", 3000),
        ],
        true,
    );

    // Adding a 4th should truncate to 3
    cache.add_message(1, msg_with_ts(4, "d", 4000));

    let cached = cache.get(1).unwrap();
    assert_eq!(cached.len(), 3);
    assert_eq!(cached[0].text, "b");
    assert_eq!(cached[1].text, "c");
    assert_eq!(cached[2].text, "d");
}

// ── fully_loaded tracking tests ──

#[test]
fn fully_loaded_false_by_default() {
    let cache = MessageCache::default();
    assert!(!cache.is_fully_loaded(42));
}

#[test]
fn fully_loaded_true_after_put_with_flag() {
    let mut cache = MessageCache::default();
    cache.put(1, vec![msg(1, "a")], true);
    assert!(cache.is_fully_loaded(1));
}

#[test]
fn fully_loaded_false_after_put_with_false_flag() {
    let mut cache = MessageCache::default();
    cache.put(1, vec![msg(1, "a")], false);
    assert!(!cache.is_fully_loaded(1));
}

#[test]
fn fully_loaded_replaced_on_subsequent_put() {
    let mut cache = MessageCache::default();
    cache.put(1, vec![msg(1, "a")], true);
    assert!(cache.is_fully_loaded(1));

    cache.put(1, vec![msg(1, "a")], false);
    assert!(!cache.is_fully_loaded(1));
}

#[test]
fn add_message_creates_not_fully_loaded_entry() {
    let mut cache = MessageCache::default();
    cache.add_message(1, msg(1, "a"));
    assert!(!cache.is_fully_loaded(1));
}

#[test]
fn add_message_preserves_fully_loaded_flag() {
    let mut cache = MessageCache::default();
    cache.put(1, vec![msg(1, "a")], true);
    cache.add_message(1, msg(2, "b"));
    assert!(cache.is_fully_loaded(1));
}

// ── Default config tests ──

#[test]
fn default_cache_has_expected_limits() {
    let cache = MessageCache::default();
    assert_eq!(cache.max_cached_chats, DEFAULT_MAX_CACHED_CHATS);
    assert_eq!(cache.max_messages_per_chat, DEFAULT_MAX_MESSAGES_PER_CHAT);
}

// ── Reaction count update tests ──

#[test]
fn update_reaction_count_modifies_existing_message() {
    let mut cache = MessageCache::default();
    cache.put(1, vec![msg(10, "hello"), msg(20, "world")], false);

    cache.update_reaction_count(1, 20, 5);

    let messages = cache.get(1).unwrap();
    assert_eq!(messages[0].reaction_count, 0);
    assert_eq!(messages[1].reaction_count, 5);
}

#[test]
fn update_reaction_count_noop_for_unknown_chat() {
    let mut cache = MessageCache::default();
    cache.update_reaction_count(999, 1, 3);
    assert!(cache.get(999).is_none());
}

#[test]
fn update_reaction_count_noop_for_unknown_message() {
    let mut cache = MessageCache::default();
    cache.put(1, vec![msg(10, "hello")], false);

    cache.update_reaction_count(1, 999, 3);

    let messages = cache.get(1).unwrap();
    assert_eq!(messages[0].reaction_count, 0);
}
