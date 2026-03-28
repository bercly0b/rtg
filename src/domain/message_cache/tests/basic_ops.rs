use super::*;

#[test]
fn empty_cache_returns_none() {
    let mut cache = MessageCache::default();
    assert!(cache.get(42).is_none());
}

#[test]
fn empty_cache_has_no_messages() {
    let cache = MessageCache::default();
    assert!(!cache.has_messages(42));
}

#[test]
fn put_and_get_roundtrip() {
    let mut cache = MessageCache::default();
    let messages = vec![msg(1, "hello"), msg(2, "world")];

    cache.put(100, messages.clone(), true);

    let cached = cache.get(100).unwrap();
    assert_eq!(cached.len(), 2);
    assert_eq!(cached[0].text, "hello");
    assert_eq!(cached[1].text, "world");
}

#[test]
fn has_messages_returns_true_after_put() {
    let mut cache = MessageCache::default();
    cache.put(100, vec![msg(1, "hello")], true);
    assert!(cache.has_messages(100));
}

#[test]
fn has_messages_returns_false_for_empty_vec() {
    let mut cache = MessageCache::default();
    cache.put(100, vec![], true);
    assert!(!cache.has_messages(100));
}

#[test]
fn put_replaces_existing_entry() {
    let mut cache = MessageCache::default();
    cache.put(100, vec![msg(1, "old")], false);
    cache.put(100, vec![msg(2, "new"), msg(3, "data")], true);

    let cached = cache.get(100).unwrap();
    assert_eq!(cached.len(), 2);
    assert_eq!(cached[0].text, "new");
}

#[test]
fn multiple_chats_are_independent() {
    let mut cache = MessageCache::default();
    cache.put(1, vec![msg(10, "chat one")], true);
    cache.put(2, vec![msg(20, "chat two")], true);

    assert_eq!(cache.get(1).unwrap()[0].text, "chat one");
    assert_eq!(cache.get(2).unwrap()[0].text, "chat two");
}

#[test]
fn get_unknown_chat_returns_none() {
    let mut cache = MessageCache::default();
    cache.put(1, vec![msg(10, "hello")], true);
    assert!(cache.get(999).is_none());
}
