mod cache_lookup;
mod failures;
mod metadata;

use std::collections::HashMap;
use std::sync::Mutex;

use tdlib_rs::types::{Chat, User};

use crate::domain::chat::ChatType;
use crate::telegram::tdlib_cache::tests::{make_test_chat, make_test_forum_topic, make_test_user};
use crate::telegram::tdlib_cache::TdLibCache;
use crate::telegram::tdlib_client::TdLibError;
use crate::usecases::list_chats::ListChatsSourceError;

use super::{build_summaries_from_ids, resolve_chat_list, ChatDataResolver};

const NO_FORCE: bool = false;

// ---------------------------------------------------------------------------
// Fake resolver
// ---------------------------------------------------------------------------

struct FakeResolver {
    cache: TdLibCache,
    chats: HashMap<i64, Chat>,
    users: HashMap<i64, User>,
    get_chat_calls: Mutex<Vec<i64>>,
    get_user_calls: Mutex<Vec<i64>>,
}

impl FakeResolver {
    fn new() -> Self {
        Self {
            cache: TdLibCache::new(),
            chats: HashMap::new(),
            users: HashMap::new(),
            get_chat_calls: Mutex::new(Vec::new()),
            get_user_calls: Mutex::new(Vec::new()),
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

fn make_unread_forum_chat(chat_id: i64, supergroup_id: i64, chat_unread: i32) -> Chat {
    let mut chat = make_supergroup_chat(chat_id, supergroup_id, "Forum");
    chat.unread_count = chat_unread;
    chat
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
