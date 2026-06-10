//! Thread-safe cache for TDLib Chat and User objects.
//!
//! Populated by the TDLib update loop from `updateNewChat` and `updateUser`
//! events. Read by `build_summaries_from_ids` and `MessageMapper` to avoid
//! per-item `get_chat`/`get_user` TDLib calls.
//!
//! TDLib guarantees that `updateNewChat`/`updateUser` arrive before the
//! corresponding ID is returned to the application, so cache lookups
//! should always succeed for IDs obtained from TDLib responses.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, RwLock};

use tdlib_rs::types::{Chat, ChatPosition, ForumTopic, Supergroup, User};

/// Thread-safe cache for TDLib objects populated by the update loop.
///
/// Uses `RwLock` for concurrent read access from background threads
/// with exclusive write access from the update loop thread.
#[derive(Debug, Clone)]
pub struct TdLibCache {
    inner: Arc<RwLock<CacheInner>>,
    /// Current user's TDLib user ID, set from `updateOption("my_id")`.
    /// 0 means not yet known.
    my_user_id: Arc<AtomicI64>,
}

#[derive(Debug, Default)]
struct CacheInner {
    chats: HashMap<i64, Chat>,
    users: HashMap<i64, User>,
    supergroups: HashMap<i64, Supergroup>,
    /// Per-forum topic read state, keyed by chat id then topic id. A chat is
    /// present only after a `getForumTopics` snapshot seeded it — updates for
    /// unseeded chats are dropped, since a partial picture would produce a
    /// wrong unread-topic count.
    forum_topics: HashMap<i64, HashMap<i32, TopicReadState>>,
}

/// Read state of a single forum topic, sufficient to derive "has unread".
///
/// `unread_count` is not tracked: `updateForumTopic` does not carry it, so the
/// verdict is derived from the watermark of incoming message ids vs the last
/// read position — both of which TDLib does push.
#[derive(Debug, Clone, Copy)]
struct TopicReadState {
    max_incoming_message_id: i64,
    last_read_inbox_message_id: i64,
}

impl TopicReadState {
    fn is_unread(&self) -> bool {
        self.max_incoming_message_id > self.last_read_inbox_message_id
    }
}

impl TdLibCache {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(CacheInner::default())),
            my_user_id: Arc::new(AtomicI64::new(0)),
        }
    }

    /// Stores the current user's ID (from TDLib `updateOption("my_id")`).
    pub fn set_my_user_id(&self, user_id: i64) {
        self.my_user_id.store(user_id, Ordering::Relaxed);
    }

    /// Returns the current user's ID, or `None` if not yet known.
    pub fn my_user_id(&self) -> Option<i64> {
        let id = self.my_user_id.load(Ordering::Relaxed);
        if id != 0 {
            Some(id)
        } else {
            None
        }
    }

    /// Inserts or replaces a chat in the cache.
    pub fn upsert_chat(&self, chat: Chat) {
        let mut inner = self.inner.write().expect("cache write lock poisoned");
        inner.chats.insert(chat.id, chat);
    }

    /// Inserts or replaces a user in the cache.
    pub fn upsert_user(&self, user: User) {
        let mut inner = self.inner.write().expect("cache write lock poisoned");
        inner.users.insert(user.id, user);
    }

    /// Inserts or replaces a supergroup in the cache.
    ///
    /// TDLib guarantees `updateSupergroup` arrives before the supergroup ID
    /// appears in any response, so reads after the initial sync are race-free.
    pub fn upsert_supergroup(&self, supergroup: Supergroup) {
        let mut inner = self.inner.write().expect("cache write lock poisoned");
        inner.supergroups.insert(supergroup.id, supergroup);
    }

    /// Looks up a supergroup by ID. Returns a clone.
    pub fn get_supergroup(&self, supergroup_id: i64) -> Option<Supergroup> {
        let inner = self.inner.read().expect("cache read lock poisoned");
        inner.supergroups.get(&supergroup_id).cloned()
    }

    /// Finds the chat_id of the cached chat backed by this supergroup, if any.
    ///
    /// Used to refresh chat metadata in the UI when a supergroup property
    /// (e.g. `is_forum`) toggles via `UpdateSupergroup`.
    pub fn find_chat_id_by_supergroup_id(&self, supergroup_id: i64) -> Option<i64> {
        let inner = self.inner.read().expect("cache read lock poisoned");
        inner.chats.iter().find_map(|(id, chat)| {
            if let tdlib_rs::enums::ChatType::Supergroup(sg) = &chat.r#type {
                if sg.supergroup_id == supergroup_id {
                    return Some(*id);
                }
            }
            None
        })
    }

    /// Updates the last message and positions for a cached chat.
    ///
    /// If the chat is not in the cache, this is a no-op.
    pub fn update_chat_last_message(
        &self,
        chat_id: i64,
        last_message: Option<tdlib_rs::types::Message>,
        positions: Vec<ChatPosition>,
    ) {
        let mut inner = self.inner.write().expect("cache write lock poisoned");
        if let Some(chat) = inner.chats.get_mut(&chat_id) {
            chat.last_message = last_message;
            chat.positions = positions;
        }
    }

    /// Updates the position for a cached chat in a specific chat list.
    ///
    /// Replaces the existing position for the same list, or appends if new.
    /// A position with `order == 0` means the chat should be removed from
    /// that list (per TDLib docs), so we remove it.
    pub fn update_chat_position(&self, chat_id: i64, position: ChatPosition) {
        let mut inner = self.inner.write().expect("cache write lock poisoned");
        if let Some(chat) = inner.chats.get_mut(&chat_id) {
            if position.order == 0 {
                chat.positions.retain(|p| p.list != position.list);
            } else {
                if let Some(existing) = chat.positions.iter_mut().find(|p| p.list == position.list)
                {
                    *existing = position;
                } else {
                    chat.positions.push(position);
                }
            }
        }
    }

    /// Updates read inbox state for a cached chat.
    pub fn update_chat_read_inbox(
        &self,
        chat_id: i64,
        unread_count: i32,
        last_read_inbox_message_id: i64,
    ) {
        let mut inner = self.inner.write().expect("cache write lock poisoned");
        if let Some(chat) = inner.chats.get_mut(&chat_id) {
            chat.unread_count = unread_count;
            chat.last_read_inbox_message_id = last_read_inbox_message_id;
        }
    }

    /// Updates read outbox state for a cached chat.
    pub fn update_chat_read_outbox(&self, chat_id: i64, last_read_outbox_message_id: i64) {
        let mut inner = self.inner.write().expect("cache write lock poisoned");
        if let Some(chat) = inner.chats.get_mut(&chat_id) {
            chat.last_read_outbox_message_id = last_read_outbox_message_id;
        }
    }

    /// Updates unread reaction count for a cached chat.
    pub fn update_chat_unread_reaction_count(&self, chat_id: i64, unread_reaction_count: i32) {
        let mut inner = self.inner.write().expect("cache write lock poisoned");
        if let Some(chat) = inner.chats.get_mut(&chat_id) {
            chat.unread_reaction_count = unread_reaction_count;
        }
    }

    /// Looks up a chat by ID. Returns a clone.
    pub fn get_chat(&self, chat_id: i64) -> Option<Chat> {
        let inner = self.inner.read().expect("cache read lock poisoned");
        inner.chats.get(&chat_id).cloned()
    }

    /// Looks up a user by ID. Returns a clone.
    pub fn get_user(&self, user_id: i64) -> Option<User> {
        let inner = self.inner.read().expect("cache read lock poisoned");
        inner.users.get(&user_id).cloned()
    }

    /// Updates user online status.
    pub fn update_user_status(&self, user_id: i64, status: tdlib_rs::enums::UserStatus) {
        let mut inner = self.inner.write().expect("cache write lock poisoned");
        if let Some(user) = inner.users.get_mut(&user_id) {
            user.status = status;
        }
    }

    /// Seeds the topic read state for a forum chat from a full `getForumTopics`
    /// snapshot, replacing any previously held state for that chat.
    pub fn seed_forum_topics(&self, chat_id: i64, topics: &[ForumTopic]) {
        let snapshot = topics
            .iter()
            .map(|t| {
                let last_read = t.last_read_inbox_message_id;
                // The snapshot's `unread_count` is authoritative; the stored ids
                // only need to reproduce its read/unread verdict while staying
                // updatable by later pushes. `last_read + 1` covers an unread
                // topic whose last_message is missing or lagging.
                let max_incoming = if t.unread_count > 0 {
                    t.last_message
                        .as_ref()
                        .map(|m| m.id)
                        .unwrap_or(0)
                        .max(last_read + 1)
                } else {
                    last_read
                };
                (
                    t.info.forum_topic_id,
                    TopicReadState {
                        max_incoming_message_id: max_incoming,
                        last_read_inbox_message_id: last_read,
                    },
                )
            })
            .collect();

        let mut inner = self.inner.write().expect("cache write lock poisoned");
        inner.forum_topics.insert(chat_id, snapshot);
    }

    /// Applies a topic read-position change (from `updateForumTopic` or a local
    /// `viewMessages`). Read positions are monotonic, so the highest one wins —
    /// this also keeps an optimistic local read from being undone by a slightly
    /// older server push. No-op for unseeded chats.
    pub fn apply_forum_topic_read(
        &self,
        chat_id: i64,
        topic_id: i32,
        last_read_inbox_message_id: i64,
    ) {
        let mut inner = self.inner.write().expect("cache write lock poisoned");
        if let Some(topics) = inner.forum_topics.get_mut(&chat_id) {
            let entry = topics.entry(topic_id).or_insert(TopicReadState {
                max_incoming_message_id: 0,
                last_read_inbox_message_id: 0,
            });
            entry.last_read_inbox_message_id = entry
                .last_read_inbox_message_id
                .max(last_read_inbox_message_id);
        }
    }

    /// Records a new incoming message in a forum topic, raising its unread
    /// watermark. A topic unknown to the snapshot (just created) starts unread.
    /// No-op for unseeded chats.
    pub fn note_incoming_topic_message(&self, chat_id: i64, topic_id: i32, message_id: i64) {
        let mut inner = self.inner.write().expect("cache write lock poisoned");
        if let Some(topics) = inner.forum_topics.get_mut(&chat_id) {
            let entry = topics.entry(topic_id).or_insert(TopicReadState {
                max_incoming_message_id: 0,
                last_read_inbox_message_id: 0,
            });
            entry.max_incoming_message_id = entry.max_incoming_message_id.max(message_id);
        }
    }

    /// Returns the number of topics with unread messages for a forum chat, or
    /// `None` when the chat was never seeded (badge unknown).
    pub fn unread_forum_topic_count(&self, chat_id: i64) -> Option<u32> {
        let inner = self.inner.read().expect("cache read lock poisoned");
        inner
            .forum_topics
            .get(&chat_id)
            .map(|topics| topics.values().filter(|t| t.is_unread()).count() as u32)
    }

    /// Returns the number of cached chats (for diagnostics).
    #[cfg(test)]
    pub fn chat_count(&self) -> usize {
        let inner = self.inner.read().expect("cache read lock poisoned");
        inner.chats.len()
    }

    /// Returns the number of cached users (for diagnostics).
    #[cfg(test)]
    pub fn user_count(&self) -> usize {
        let inner = self.inner.read().expect("cache read lock poisoned");
        inner.users.len()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn make_test_chat(id: i64, title: &str) -> Chat {
        Chat {
            id,
            r#type: tdlib_rs::enums::ChatType::Private(tdlib_rs::types::ChatTypePrivate {
                user_id: id,
            }),
            title: title.to_owned(),
            photo: None,
            accent_color_id: 0,
            background_custom_emoji_id: 0,
            upgraded_gift_colors: None,
            profile_accent_color_id: 0,
            profile_background_custom_emoji_id: 0,
            permissions: tdlib_rs::types::ChatPermissions {
                can_send_basic_messages: false,
                can_send_audios: false,
                can_send_documents: false,
                can_send_photos: false,
                can_send_videos: false,
                can_send_video_notes: false,
                can_send_voice_notes: false,
                can_send_polls: false,
                can_send_other_messages: false,
                can_add_link_previews: false,
                can_change_info: false,
                can_invite_users: false,
                can_pin_messages: false,
                can_create_topics: false,
            },
            last_message: None,
            positions: vec![],
            chat_lists: vec![],
            message_sender_id: None,
            block_list: None,
            has_protected_content: false,
            is_translatable: false,
            is_marked_as_unread: false,
            view_as_topics: false,
            has_scheduled_messages: false,
            can_be_deleted_only_for_self: false,
            can_be_deleted_for_all_users: false,
            can_be_reported: false,
            default_disable_notification: false,
            unread_count: 0,
            last_read_inbox_message_id: 0,
            last_read_outbox_message_id: 0,
            unread_mention_count: 0,
            unread_reaction_count: 0,
            notification_settings: tdlib_rs::types::ChatNotificationSettings {
                use_default_mute_for: false,
                mute_for: 0,
                use_default_sound: true,
                sound_id: 0,
                use_default_show_preview: true,
                show_preview: false,
                use_default_mute_stories: false,
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
            available_reactions: tdlib_rs::enums::ChatAvailableReactions::Some(
                tdlib_rs::types::ChatAvailableReactionsSome {
                    reactions: vec![],
                    max_reaction_count: 0,
                },
            ),
            message_auto_delete_time: 0,
            emoji_status: None,
            background: None,
            theme: None,
            action_bar: None,
            business_bot_manage_bar: None,
            video_chat: tdlib_rs::types::VideoChat {
                group_call_id: 0,
                has_participants: false,
                default_participant_id: None,
            },
            pending_join_requests: None,
            reply_markup_message_id: 0,
            draft_message: None,
            client_data: String::new(),
        }
    }

    pub fn make_test_supergroup(id: i64, is_forum: bool) -> Supergroup {
        Supergroup {
            id,
            usernames: None,
            date: 0,
            status: tdlib_rs::enums::ChatMemberStatus::Member(
                tdlib_rs::types::ChatMemberStatusMember {
                    member_until_date: 0,
                },
            ),
            member_count: 0,
            boost_level: 0,
            has_automatic_translation: false,
            has_linked_chat: false,
            has_location: false,
            sign_messages: false,
            show_message_sender: false,
            join_to_send_messages: false,
            join_by_request: false,
            is_slow_mode_enabled: false,
            is_channel: false,
            is_broadcast_group: false,
            is_forum,
            is_direct_messages_group: false,
            is_administered_direct_messages_group: false,
            verification_status: None,
            has_direct_messages_group: false,
            has_forum_tabs: false,
            restriction_info: None,
            paid_message_star_count: 0,
            active_story_state: None,
        }
    }

    pub fn make_test_forum_topic(topic_id: i32, unread_count: i32) -> ForumTopic {
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
                creator_id: tdlib_rs::enums::MessageSender::User(
                    tdlib_rs::types::MessageSenderUser { user_id: 0 },
                ),
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

    pub fn make_test_user(id: i64, first_name: &str) -> User {
        User {
            id,
            first_name: first_name.to_owned(),
            last_name: String::new(),
            usernames: None,
            phone_number: String::new(),
            status: tdlib_rs::enums::UserStatus::Empty,
            profile_photo: None,
            accent_color_id: 0,
            background_custom_emoji_id: 0,
            upgraded_gift_colors: None,
            profile_accent_color_id: 0,
            profile_background_custom_emoji_id: 0,
            emoji_status: None,
            is_contact: false,
            is_mutual_contact: false,
            is_close_friend: false,
            verification_status: None,
            is_premium: false,
            is_support: false,
            restriction_info: None,
            active_story_state: None,
            restricts_new_chats: false,
            paid_message_star_count: 0,
            have_access: true,
            r#type: tdlib_rs::enums::UserType::Regular,
            language_code: String::new(),
            added_to_attachment_menu: false,
        }
    }

    #[test]
    fn upsert_and_get_chat() {
        let cache = TdLibCache::new();
        let chat = make_test_chat(42, "Test Chat");

        cache.upsert_chat(chat);

        let cached = cache.get_chat(42).expect("chat should be cached");
        assert_eq!(cached.id, 42);
        assert_eq!(cached.title, "Test Chat");
    }

    #[test]
    fn upsert_replaces_existing_chat() {
        let cache = TdLibCache::new();
        cache.upsert_chat(make_test_chat(1, "Old"));
        cache.upsert_chat(make_test_chat(1, "New"));

        let cached = cache.get_chat(1).expect("chat should be cached");
        assert_eq!(cached.title, "New");
        assert_eq!(cache.chat_count(), 1);
    }

    #[test]
    fn get_unknown_chat_returns_none() {
        let cache = TdLibCache::new();
        assert!(cache.get_chat(999).is_none());
    }

    #[test]
    fn upsert_and_get_user() {
        let cache = TdLibCache::new();
        let user = make_test_user(7, "Alice");

        cache.upsert_user(user);

        let cached = cache.get_user(7).expect("user should be cached");
        assert_eq!(cached.id, 7);
        assert_eq!(cached.first_name, "Alice");
    }

    #[test]
    fn get_unknown_user_returns_none() {
        let cache = TdLibCache::new();
        assert!(cache.get_user(999).is_none());
    }

    #[test]
    fn update_chat_last_message_modifies_cached_chat() {
        let cache = TdLibCache::new();
        cache.upsert_chat(make_test_chat(1, "Chat"));

        cache.update_chat_last_message(1, None, vec![]);

        let cached = cache.get_chat(1).expect("chat should be cached");
        assert!(cached.last_message.is_none());
        assert!(cached.positions.is_empty());
    }

    #[test]
    fn update_chat_last_message_noop_for_unknown_chat() {
        let cache = TdLibCache::new();
        cache.update_chat_last_message(999, None, vec![]);
        assert!(cache.get_chat(999).is_none());
    }

    #[test]
    fn update_chat_read_inbox_modifies_unread_count() {
        let cache = TdLibCache::new();
        cache.upsert_chat(make_test_chat(1, "Chat"));

        cache.update_chat_read_inbox(1, 5, 100);

        let cached = cache.get_chat(1).expect("chat should be cached");
        assert_eq!(cached.unread_count, 5);
        assert_eq!(cached.last_read_inbox_message_id, 100);
    }

    #[test]
    fn update_chat_read_outbox_modifies_read_id() {
        let cache = TdLibCache::new();
        cache.upsert_chat(make_test_chat(1, "Chat"));

        cache.update_chat_read_outbox(1, 200);

        let cached = cache.get_chat(1).expect("chat should be cached");
        assert_eq!(cached.last_read_outbox_message_id, 200);
    }

    #[test]
    fn update_user_status_modifies_cached_user() {
        let cache = TdLibCache::new();
        cache.upsert_user(make_test_user(7, "Alice"));

        cache.update_user_status(
            7,
            tdlib_rs::enums::UserStatus::Online(tdlib_rs::types::UserStatusOnline {
                expires: 9999,
            }),
        );

        let cached = cache.get_user(7).expect("user should be cached");
        assert!(matches!(
            cached.status,
            tdlib_rs::enums::UserStatus::Online(_)
        ));
    }

    #[test]
    fn update_chat_position_replaces_existing() {
        let cache = TdLibCache::new();
        let mut chat = make_test_chat(1, "Chat");
        chat.positions.push(ChatPosition {
            list: tdlib_rs::enums::ChatList::Main,
            order: 100,
            is_pinned: false,
            source: None,
        });
        cache.upsert_chat(chat);

        cache.update_chat_position(
            1,
            ChatPosition {
                list: tdlib_rs::enums::ChatList::Main,
                order: 200,
                is_pinned: true,
                source: None,
            },
        );

        let cached = cache.get_chat(1).expect("chat should be cached");
        assert_eq!(cached.positions.len(), 1);
        assert_eq!(cached.positions[0].order, 200);
        assert!(cached.positions[0].is_pinned);
    }

    #[test]
    fn update_chat_position_removes_on_zero_order() {
        let cache = TdLibCache::new();
        let mut chat = make_test_chat(1, "Chat");
        chat.positions.push(ChatPosition {
            list: tdlib_rs::enums::ChatList::Main,
            order: 100,
            is_pinned: false,
            source: None,
        });
        cache.upsert_chat(chat);

        cache.update_chat_position(
            1,
            ChatPosition {
                list: tdlib_rs::enums::ChatList::Main,
                order: 0,
                is_pinned: false,
                source: None,
            },
        );

        let cached = cache.get_chat(1).expect("chat should be cached");
        assert!(cached.positions.is_empty());
    }

    #[test]
    fn upsert_and_get_supergroup() {
        let cache = TdLibCache::new();
        cache.upsert_supergroup(make_test_supergroup(101, true));

        let cached = cache
            .get_supergroup(101)
            .expect("supergroup should be cached");
        assert_eq!(cached.id, 101);
        assert!(cached.is_forum);
    }

    #[test]
    fn upsert_replaces_existing_supergroup() {
        let cache = TdLibCache::new();
        cache.upsert_supergroup(make_test_supergroup(1, false));
        cache.upsert_supergroup(make_test_supergroup(1, true));

        let cached = cache
            .get_supergroup(1)
            .expect("supergroup should be cached");
        assert!(cached.is_forum);
    }

    #[test]
    fn get_unknown_supergroup_returns_none() {
        let cache = TdLibCache::new();
        assert!(cache.get_supergroup(999).is_none());
    }

    fn unread_topic_with_read_position(topic_id: i32, last_read: i64, last_msg: i64) -> ForumTopic {
        let mut topic = make_test_forum_topic(topic_id, 1);
        topic.last_read_inbox_message_id = last_read;
        topic.last_message = Some(make_test_topic_message(last_msg));
        topic
    }

    pub fn make_test_topic_message(id: i64) -> tdlib_rs::types::Message {
        use tdlib_rs::enums::{MessageContent, MessageSender};
        tdlib_rs::types::Message {
            id,
            sender_id: MessageSender::User(tdlib_rs::types::MessageSenderUser { user_id: 1 }),
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
            content: MessageContent::MessageText(tdlib_rs::types::MessageText {
                text: tdlib_rs::types::FormattedText {
                    text: "test".to_owned(),
                    entities: vec![],
                },
                link_preview: None,
                link_preview_options: None,
            }),
            reply_markup: None,
        }
    }

    #[test]
    fn seed_counts_unread_topics() {
        let cache = TdLibCache::new();
        cache.seed_forum_topics(
            10,
            &[
                make_test_forum_topic(1, 3),
                make_test_forum_topic(2, 0),
                make_test_forum_topic(3, 1),
            ],
        );

        assert_eq!(cache.unread_forum_topic_count(10), Some(2));
    }

    #[test]
    fn unseeded_chat_has_no_unread_topic_count() {
        let cache = TdLibCache::new();
        assert_eq!(cache.unread_forum_topic_count(10), None);
    }

    #[test]
    fn seed_unread_topic_without_last_message_still_counts() {
        let mut topic = make_test_forum_topic(1, 5);
        topic.last_message = None;
        topic.last_read_inbox_message_id = 100;

        let cache = TdLibCache::new();
        cache.seed_forum_topics(10, &[topic]);

        assert_eq!(cache.unread_forum_topic_count(10), Some(1));
    }

    #[test]
    fn apply_read_clears_topic_unread() {
        let cache = TdLibCache::new();
        cache.seed_forum_topics(10, &[unread_topic_with_read_position(1, 100, 150)]);
        assert_eq!(cache.unread_forum_topic_count(10), Some(1));

        cache.apply_forum_topic_read(10, 1, 150);

        assert_eq!(cache.unread_forum_topic_count(10), Some(0));
    }

    #[test]
    fn apply_read_below_watermark_keeps_topic_unread() {
        let cache = TdLibCache::new();
        cache.seed_forum_topics(10, &[unread_topic_with_read_position(1, 100, 150)]);

        cache.apply_forum_topic_read(10, 1, 120);

        assert_eq!(cache.unread_forum_topic_count(10), Some(1));
    }

    #[test]
    fn apply_read_never_regresses() {
        let cache = TdLibCache::new();
        cache.seed_forum_topics(10, &[unread_topic_with_read_position(1, 100, 150)]);

        cache.apply_forum_topic_read(10, 1, 150);
        // A lagging server push with an older read position must not resurrect
        // the optimistic local read.
        cache.apply_forum_topic_read(10, 1, 110);

        assert_eq!(cache.unread_forum_topic_count(10), Some(0));
    }

    #[test]
    fn apply_read_ignored_for_unseeded_chat() {
        let cache = TdLibCache::new();
        cache.apply_forum_topic_read(10, 1, 150);

        assert_eq!(cache.unread_forum_topic_count(10), None);
    }

    #[test]
    fn incoming_message_marks_read_topic_unread() {
        let cache = TdLibCache::new();
        cache.seed_forum_topics(10, &[make_test_forum_topic(1, 0)]);
        assert_eq!(cache.unread_forum_topic_count(10), Some(0));

        cache.note_incoming_topic_message(10, 1, 200);

        assert_eq!(cache.unread_forum_topic_count(10), Some(1));
    }

    #[test]
    fn incoming_message_in_unknown_topic_counts_as_unread() {
        let cache = TdLibCache::new();
        cache.seed_forum_topics(10, &[make_test_forum_topic(1, 0)]);

        cache.note_incoming_topic_message(10, 99, 200);

        assert_eq!(cache.unread_forum_topic_count(10), Some(1));
    }

    #[test]
    fn incoming_message_ignored_for_unseeded_chat() {
        let cache = TdLibCache::new();
        cache.note_incoming_topic_message(10, 1, 200);

        assert_eq!(cache.unread_forum_topic_count(10), None);
    }

    #[test]
    fn reseed_replaces_previous_topic_state() {
        let cache = TdLibCache::new();
        cache.seed_forum_topics(
            10,
            &[make_test_forum_topic(1, 4), make_test_forum_topic(2, 1)],
        );
        assert_eq!(cache.unread_forum_topic_count(10), Some(2));

        // Topic 2 deleted, topic 1 read in the fresh snapshot.
        cache.seed_forum_topics(10, &[make_test_forum_topic(1, 0)]);

        assert_eq!(cache.unread_forum_topic_count(10), Some(0));
    }

    #[test]
    fn clone_shares_underlying_data() {
        let cache = TdLibCache::new();
        let cache2 = cache.clone();

        cache.upsert_chat(make_test_chat(1, "Shared"));
        assert!(cache2.get_chat(1).is_some());
    }
}
