use super::*;

// ── add_message tests ──

#[test]
fn add_message_to_empty_cache_creates_entry() {
    let mut cache = MessageCache::default();
    cache.add_message(100, msg(1, "hello"));

    assert!(cache.has_messages(100));
    assert_eq!(cache.get(100).unwrap().len(), 1);
    assert_eq!(cache.get(100).unwrap()[0].text, "hello");
}

#[test]
fn add_message_appends_to_existing_entry() {
    let mut cache = MessageCache::default();
    cache.put(100, vec![msg_with_ts(1, "first", 1000)], true);
    cache.add_message(100, msg_with_ts(2, "second", 2000));

    let cached = cache.get(100).unwrap();
    assert_eq!(cached.len(), 2);
    assert_eq!(cached[0].text, "first");
    assert_eq!(cached[1].text, "second");
}

#[test]
fn add_message_inserts_in_timestamp_order() {
    let mut cache = MessageCache::default();
    cache.put(
        100,
        vec![msg_with_ts(1, "early", 1000), msg_with_ts(3, "late", 3000)],
        true,
    );
    cache.add_message(100, msg_with_ts(2, "middle", 2000));

    let cached = cache.get(100).unwrap();
    assert_eq!(cached.len(), 3);
    assert_eq!(cached[0].text, "early");
    assert_eq!(cached[1].text, "middle");
    assert_eq!(cached[2].text, "late");
}

#[test]
fn add_message_skips_duplicate_by_id() {
    let mut cache = MessageCache::default();
    cache.put(100, vec![msg(1, "original")], true);
    cache.add_message(100, msg(1, "duplicate"));

    let cached = cache.get(100).unwrap();
    assert_eq!(cached.len(), 1);
    assert_eq!(cached[0].text, "original");
}

// ── remove_messages tests ──

#[test]
fn remove_messages_removes_by_id() {
    let mut cache = MessageCache::default();
    cache.put(100, vec![msg(1, "a"), msg(2, "b"), msg(3, "c")], true);
    cache.remove_messages(100, &[2]);

    let cached = cache.get(100).unwrap();
    assert_eq!(cached.len(), 2);
    assert_eq!(cached[0].text, "a");
    assert_eq!(cached[1].text, "c");
}

#[test]
fn remove_messages_handles_multiple_ids() {
    let mut cache = MessageCache::default();
    cache.put(100, vec![msg(1, "a"), msg(2, "b"), msg(3, "c")], true);
    cache.remove_messages(100, &[1, 3]);

    let cached = cache.get(100).unwrap();
    assert_eq!(cached.len(), 1);
    assert_eq!(cached[0].text, "b");
}

#[test]
fn remove_messages_ignores_unknown_chat() {
    let mut cache = MessageCache::default();
    cache.remove_messages(999, &[1, 2]); // should not panic
    assert!(cache.get(999).is_none());
}

#[test]
fn remove_messages_ignores_unknown_ids() {
    let mut cache = MessageCache::default();
    cache.put(100, vec![msg(1, "a")], true);
    cache.remove_messages(100, &[999]);

    assert_eq!(cache.get(100).unwrap().len(), 1);
}
