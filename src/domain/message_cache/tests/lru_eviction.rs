use super::*;

#[test]
fn evicts_lru_chat_when_over_capacity() {
    let mut cache = MessageCache::new(3, 200);

    cache.put(1, vec![msg(10, "chat 1")], true);
    cache.put(2, vec![msg(20, "chat 2")], true);
    cache.put(3, vec![msg(30, "chat 3")], true);
    // All 3 fit
    assert!(cache.has_messages(1));

    // Adding a 4th should evict chat 1 (LRU)
    cache.put(4, vec![msg(40, "chat 4")], true);

    assert!(!cache.has_messages(1), "chat 1 should have been evicted");
    assert!(cache.has_messages(2));
    assert!(cache.has_messages(3));
    assert!(cache.has_messages(4));
}

#[test]
fn get_touch_prevents_eviction() {
    let mut cache = MessageCache::new(3, 200);

    cache.put(1, vec![msg(10, "chat 1")], true);
    cache.put(2, vec![msg(20, "chat 2")], true);
    cache.put(3, vec![msg(30, "chat 3")], true);

    // Touch chat 1 via get (makes it MRU)
    let _ = cache.get(1);

    // Adding chat 4 should evict chat 2 (now LRU), not chat 1
    cache.put(4, vec![msg(40, "chat 4")], true);

    assert!(cache.has_messages(1), "chat 1 should survive (was touched)");
    assert!(!cache.has_messages(2), "chat 2 should be evicted (now LRU)");
    assert!(cache.has_messages(3));
    assert!(cache.has_messages(4));
}

#[test]
fn has_messages_does_not_touch_lru() {
    let mut cache = MessageCache::new(3, 200);

    cache.put(1, vec![msg(10, "chat 1")], true);
    cache.put(2, vec![msg(20, "chat 2")], true);
    cache.put(3, vec![msg(30, "chat 3")], true);

    // has_messages should NOT touch LRU order
    assert!(cache.has_messages(1));

    // Adding chat 4 should still evict chat 1 (not touched by has_messages)
    cache.put(4, vec![msg(40, "chat 4")], true);

    assert!(
        !cache.has_messages(1),
        "chat 1 should be evicted (has_messages doesn't touch)"
    );
    assert!(cache.has_messages(2));
}

#[test]
fn add_message_new_entry_can_trigger_eviction() {
    let mut cache = MessageCache::new(2, 200);

    cache.put(1, vec![msg(10, "a")], true);
    cache.put(2, vec![msg(20, "b")], true);

    // add_message for a new chat should evict the LRU
    cache.add_message(3, msg(30, "c"));

    assert!(!cache.has_messages(1), "chat 1 should be evicted");
    assert!(cache.has_messages(2));
    assert!(cache.has_messages(3));
}

#[test]
fn add_message_existing_entry_does_not_evict() {
    let mut cache = MessageCache::new(2, 200);

    cache.put(1, vec![msg(10, "a")], true);
    cache.put(2, vec![msg(20, "b")], true);

    // add_message to existing chat should not evict anything
    cache.add_message(2, msg(21, "b2"));

    assert!(cache.has_messages(1));
    assert!(cache.has_messages(2));
    assert_eq!(cache.get(2).unwrap().len(), 2);
}
