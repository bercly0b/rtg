use std::collections::HashMap;
use std::sync::Mutex;

use tdlib_rs::types::{Chat, ForumTopic, User};

use crate::domain::chat::ChatType;
use crate::telegram::tdlib_cache::tests::{make_test_chat, make_test_user};
use crate::telegram::tdlib_cache::TdLibCache;
use crate::telegram::tdlib_client::TdLibError;

use super::{build_summaries_from_ids, ChatDataResolver};

const NO_FORCE: bool = false;

// ---------------------------------------------------------------------------
// Fake resolver
// ---------------------------------------------------------------------------

struct FakeResolver {
    cache: TdLibCache,
    chats: HashMap<i64, Chat>,
    users: HashMap<i64, User>,
    forum_topics: HashMap<i64, Vec<ForumTopic>>,
    get_chat_calls: Mutex<Vec<i64>>,
    get_user_calls: Mutex<Vec<i64>>,
    get_forum_topics_calls: Mutex<Vec<i64>>,
}

impl FakeResolver {
    fn new() -> Self {
        Self {
            cache: TdLibCache::new(),
            chats: HashMap::new(),
            users: HashMap::new(),
            forum_topics: HashMap::new(),
            get_chat_calls: Mutex::new(Vec::new()),
            get_user_calls: Mutex::new(Vec::new()),
            get_forum_topics_calls: Mutex::new(Vec::new()),
        }
    }

    fn with_cached_chat(self, chat: Chat) -> Self {
        self.cache.upsert_chat(chat);
        self
    }

    fn with_cached_user(self, user: User) -> Self {
        self.cache.upsert_user(user);
        self
    }

    fn with_tdlib_chat(mut self, chat: Chat) -> Self {
        self.chats.insert(chat.id, chat);
        self
    }

    fn with_tdlib_user(mut self, user: User) -> Self {
        self.users.insert(user.id, user);
        self
    }

    fn get_chat_call_count(&self) -> usize {
        self.get_chat_calls.lock().unwrap().len()
    }

    fn get_user_call_count(&self) -> usize {
        self.get_user_calls.lock().unwrap().len()
    }

    fn with_forum_topics(mut self, chat_id: i64, topics: Vec<ForumTopic>) -> Self {
        self.forum_topics.insert(chat_id, topics);
        self
    }

    fn get_forum_topics_call_count(&self) -> usize {
        self.get_forum_topics_calls.lock().unwrap().len()
    }
}

impl ChatDataResolver for FakeResolver {
    fn cache(&self) -> &TdLibCache {
        &self.cache
    }

    fn get_chat(&self, chat_id: i64) -> Result<Chat, TdLibError> {
        self.get_chat_calls.lock().unwrap().push(chat_id);
        self.chats
            .get(&chat_id)
            .cloned()
            .ok_or(TdLibError::Request {
                code: 404,
                message: format!("chat {chat_id} not found"),
            })
    }

    fn get_user(&self, user_id: i64) -> Result<User, TdLibError> {
        self.get_user_calls.lock().unwrap().push(user_id);
        self.users
            .get(&user_id)
            .cloned()
            .ok_or(TdLibError::Request {
                code: 404,
                message: format!("user {user_id} not found"),
            })
    }

    fn get_forum_topics(&self, chat_id: i64, _limit: i32) -> Result<Vec<ForumTopic>, TdLibError> {
        self.get_forum_topics_calls.lock().unwrap().push(chat_id);
        self.forum_topics
            .get(&chat_id)
            .cloned()
            .ok_or(TdLibError::Request {
                code: 404,
                message: format!("forum topics for chat {chat_id} not found"),
            })
    }
}

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
// Failure scenarios
// ---------------------------------------------------------------------------

#[test]
fn double_miss_skips_chat() {
    let resolver = FakeResolver::new();

    let summaries = build_summaries_from_ids(&resolver, vec![999], NO_FORCE);

    assert!(summaries.is_empty(), "unresolvable chat should be skipped");
    assert_eq!(
        resolver.get_chat_call_count(),
        1,
        "get_chat called as fallback"
    );
}

#[test]
fn all_chats_fail_returns_empty_vec() {
    let resolver = FakeResolver::new();

    let summaries = build_summaries_from_ids(&resolver, vec![10, 20, 30], NO_FORCE);

    assert!(summaries.is_empty());
    assert_eq!(resolver.get_chat_call_count(), 3);
}

#[test]
fn partial_failure_returns_resolved_chats_only() {
    let resolver = FakeResolver::new().with_cached_chat(make_test_chat(1, "Good Chat"));

    let summaries = build_summaries_from_ids(&resolver, vec![1, 2], NO_FORCE);

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].chat_id, 1);
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
// Empty input
// ---------------------------------------------------------------------------

#[test]
fn empty_chat_ids_returns_empty_vec() {
    let resolver = FakeResolver::new();

    let summaries = build_summaries_from_ids(&resolver, vec![], NO_FORCE);

    assert!(summaries.is_empty());
    assert_eq!(resolver.get_chat_call_count(), 0);
}

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
// Helpers
// ---------------------------------------------------------------------------

fn make_online_user(id: i64, name: &str) -> User {
    let mut user = make_test_user(id, name);
    user.status =
        tdlib_rs::enums::UserStatus::Online(tdlib_rs::types::UserStatusOnline { expires: 9999 });
    user
}

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

fn make_group_chat(id: i64, title: &str) -> Chat {
    let mut chat = make_test_chat(id, title);
    chat.r#type = tdlib_rs::enums::ChatType::BasicGroup(tdlib_rs::types::ChatTypeBasicGroup {
        basic_group_id: id,
    });
    chat
}

fn make_supergroup_chat(chat_id: i64, supergroup_id: i64, title: &str) -> Chat {
    let mut chat = make_test_chat(chat_id, title);
    chat.r#type = tdlib_rs::enums::ChatType::Supergroup(tdlib_rs::types::ChatTypeSupergroup {
        supergroup_id,
        is_channel: false,
    });
    chat
}

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
// Forum unread-topic count: the badge counts unread topics, not messages
// ---------------------------------------------------------------------------

fn make_forum_topic(topic_id: i32, unread_count: i32) -> ForumTopic {
    ForumTopic {
        info: tdlib_rs::types::ForumTopicInfo {
            chat_id: 0,
            forum_topic_id: topic_id,
            name: format!("Topic {topic_id}"),
            icon: tdlib_rs::types::ForumTopicIcon {
                color: 0,
                custom_emoji_id: 0,
            },
            creation_date: 0,
            creator_id: tdlib_rs::enums::MessageSender::User(tdlib_rs::types::MessageSenderUser {
                user_id: 0,
            }),
            is_general: false,
            is_outgoing: false,
            is_closed: false,
            is_hidden: false,
            is_name_implicit: false,
        },
        last_message: None,
        order: i64::from(topic_id),
        is_pinned: false,
        unread_count,
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

fn make_unread_forum_chat(chat_id: i64, supergroup_id: i64, chat_unread: i32) -> Chat {
    let mut chat = make_supergroup_chat(chat_id, supergroup_id, "Forum");
    chat.unread_count = chat_unread;
    chat
}

#[test]
fn forum_badge_counts_unread_topics_not_messages() {
    use crate::telegram::tdlib_cache::tests::make_test_supergroup;

    // chat-level unread_count (42) is the stale message total; the real unread
    // is two topics with 1 and 7 messages — the badge must read 2.
    let resolver = FakeResolver::new()
        .with_cached_chat(make_unread_forum_chat(10, 100, 42))
        .with_forum_topics(
            10,
            vec![
                make_forum_topic(1, 1),
                make_forum_topic(2, 7),
                make_forum_topic(3, 0),
            ],
        );
    resolver
        .cache
        .upsert_supergroup(make_test_supergroup(100, true));

    let summaries = build_summaries_from_ids(&resolver, vec![10], NO_FORCE);

    assert_eq!(summaries.len(), 1);
    assert!(summaries[0].is_forum);
    assert_eq!(summaries[0].unread_count, 42);
    assert_eq!(summaries[0].unread_topic_count, Some(2));
}

#[test]
fn read_forum_skips_topic_fetch_and_leaves_count_none() {
    use crate::telegram::tdlib_cache::tests::make_test_supergroup;

    let resolver = FakeResolver::new().with_cached_chat(make_unread_forum_chat(10, 100, 0));
    resolver
        .cache
        .upsert_supergroup(make_test_supergroup(100, true));

    let summaries = build_summaries_from_ids(&resolver, vec![10], NO_FORCE);

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].unread_topic_count, None);
    assert_eq!(
        resolver.get_forum_topics_call_count(),
        0,
        "fully-read forum must not trigger a topic fetch"
    );
}

#[test]
fn forum_topic_fetch_failure_yields_zero_not_message_count() {
    use crate::telegram::tdlib_cache::tests::make_test_supergroup;

    // unread_count > 0 but no topics registered in the fake -> fetch errors.
    let resolver = FakeResolver::new().with_cached_chat(make_unread_forum_chat(10, 100, 42));
    resolver
        .cache
        .upsert_supergroup(make_test_supergroup(100, true));

    let summaries = build_summaries_from_ids(&resolver, vec![10], NO_FORCE);

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].unread_topic_count, Some(0));
    assert_eq!(resolver.get_forum_topics_call_count(), 1);
}

#[test]
fn non_forum_unread_chat_never_fetches_topics() {
    let mut chat = make_test_chat(1, "Plain");
    chat.unread_count = 9;
    let resolver = FakeResolver::new().with_cached_chat(chat);

    let summaries = build_summaries_from_ids(&resolver, vec![1], NO_FORCE);

    assert_eq!(summaries[0].unread_count, 9);
    assert_eq!(summaries[0].unread_topic_count, None);
    assert_eq!(resolver.get_forum_topics_call_count(), 0);
}

fn make_message_from_user(user_id: i64) -> tdlib_rs::types::Message {
    tdlib_rs::types::Message {
        id: 1000,
        sender_id: tdlib_rs::enums::MessageSender::User(tdlib_rs::types::MessageSenderUser {
            user_id,
        }),
        chat_id: 0,
        sending_state: None,
        scheduling_state: None,
        is_outgoing: false,
        is_pinned: false,
        is_from_offline: false,
        can_be_saved: true,
        has_timestamped_media: false,
        is_channel_post: false,
        is_paid_star_suggested_post: false,
        is_paid_ton_suggested_post: false,
        contains_unread_mention: false,
        date: 1_700_000_000,
        edit_date: 0,
        forward_info: None,
        import_info: None,
        interaction_info: None,
        unread_reactions: vec![],
        fact_check: None,
        suggested_post_info: None,
        reply_to: None,
        topic_id: None,
        self_destruct_type: None,
        self_destruct_in: 0.0,
        auto_delete_in: 0.0,
        via_bot_user_id: 0,
        sender_business_bot_user_id: 0,
        sender_boost_count: 0,
        paid_message_star_count: 0,
        author_signature: String::new(),
        media_album_id: 0,
        effect_id: 0,
        restriction_info: None,
        summary_language_code: String::new(),
        content: tdlib_rs::enums::MessageContent::MessageText(tdlib_rs::types::MessageText {
            text: tdlib_rs::types::FormattedText {
                text: "test message".to_owned(),
                entities: vec![],
            },
            link_preview: None,
            link_preview_options: None,
        }),
        reply_markup: None,
    }
}
