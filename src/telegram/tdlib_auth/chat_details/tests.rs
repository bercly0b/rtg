use std::collections::HashMap;

use tdlib_rs::enums::{ChatMemberStatus, MessageSender, UserStatus};
use tdlib_rs::types::{
    BasicGroupFullInfo, Chat, ChatMember, ChatMemberStatusMember, ChatTypeBasicGroup,
    ChatTypeSupergroup, MessageSenderUser, SupergroupFullInfo, User, UserFullInfo,
    UserStatusOnline, Usernames,
};

use crate::domain::chat::ChatType;
use crate::telegram::tdlib_cache::tests::{make_test_chat, make_test_user};
use crate::telegram::tdlib_client::TdLibError;
use crate::usecases::chat_subtitle::ChatSubtitleError;

use super::{resolve_chat_info, ChatInfoResolver};

// ---------------------------------------------------------------------------
// Fake resolver
// ---------------------------------------------------------------------------

/// Serves chats, users and group full info from maps; anything missing fails
/// like a TDLib 404. `getUserFullInfo` always fails: building a `UserFullInfo`
/// takes ~30 unrelated fields, and only its failure path matters here.
#[derive(Default)]
struct FakeResolver {
    chats: HashMap<i64, Chat>,
    users: HashMap<i64, User>,
    basic_groups: HashMap<i64, BasicGroupFullInfo>,
    supergroups: HashMap<i64, SupergroupFullInfo>,
}

fn not_found(what: &str, id: i64) -> TdLibError {
    TdLibError::Request {
        code: 404,
        message: format!("{what} {id} not found"),
    }
}

impl ChatInfoResolver for FakeResolver {
    fn get_chat(&self, chat_id: i64) -> Result<Chat, TdLibError> {
        self.chats
            .get(&chat_id)
            .cloned()
            .ok_or_else(|| not_found("chat", chat_id))
    }

    fn get_user(&self, user_id: i64) -> Result<User, TdLibError> {
        self.users
            .get(&user_id)
            .cloned()
            .ok_or_else(|| not_found("user", user_id))
    }

    fn get_user_full_info(&self, user_id: i64) -> Result<UserFullInfo, TdLibError> {
        Err(not_found("user full info", user_id))
    }

    fn get_basic_group_full_info(
        &self,
        basic_group_id: i64,
    ) -> Result<BasicGroupFullInfo, TdLibError> {
        self.basic_groups
            .get(&basic_group_id)
            .cloned()
            .ok_or_else(|| not_found("basic group", basic_group_id))
    }

    fn get_supergroup_full_info(
        &self,
        supergroup_id: i64,
    ) -> Result<SupergroupFullInfo, TdLibError> {
        self.supergroups
            .get(&supergroup_id)
            .cloned()
            .ok_or_else(|| not_found("supergroup", supergroup_id))
    }
}

// ---------------------------------------------------------------------------
// Critical requests fail the popup
// ---------------------------------------------------------------------------

#[test]
fn get_chat_failure_returns_error() {
    let resolver = FakeResolver::default();

    let result = resolve_chat_info(&resolver, 1, ChatType::Private, "Alice".into());

    assert_eq!(result, Err(ChatSubtitleError::Unavailable));
}

#[test]
fn private_user_failure_returns_error() {
    let mut resolver = FakeResolver::default();
    resolver.chats.insert(1, make_test_chat(1, "Alice"));

    let result = resolve_chat_info(&resolver, 1, ChatType::Private, "Alice".into());

    assert_eq!(result, Err(ChatSubtitleError::Unavailable));
}

#[test]
fn basic_group_full_info_failure_returns_error() {
    let mut resolver = FakeResolver::default();
    resolver.chats.insert(5, make_basic_group_chat(5, 50));

    let result = resolve_chat_info(&resolver, 5, ChatType::Group, "Team".into());

    assert_eq!(result, Err(ChatSubtitleError::Unavailable));
}

#[test]
fn supergroup_full_info_failure_returns_error() {
    let mut resolver = FakeResolver::default();
    resolver.chats.insert(6, make_supergroup_chat(6, 60, false));

    let result = resolve_chat_info(&resolver, 6, ChatType::Group, "Community".into());

    assert_eq!(result, Err(ChatSubtitleError::Unavailable));
}

#[test]
fn channel_full_info_failure_returns_error() {
    let mut resolver = FakeResolver::default();
    resolver.chats.insert(7, make_supergroup_chat(7, 70, true));

    let result = resolve_chat_info(&resolver, 7, ChatType::Channel, "News".into());

    assert_eq!(result, Err(ChatSubtitleError::Unavailable));
}

#[test]
fn chat_type_mismatch_returns_error() {
    let mut resolver = FakeResolver::default();
    resolver.chats.insert(1, make_test_chat(1, "Alice"));

    let result = resolve_chat_info(&resolver, 1, ChatType::Group, "Alice".into());

    assert_eq!(result, Err(ChatSubtitleError::Unavailable));
}

// ---------------------------------------------------------------------------
// Successful resolution
// ---------------------------------------------------------------------------

#[test]
fn private_bio_failure_keeps_status_and_username() {
    let mut resolver = FakeResolver::default();
    resolver.chats.insert(1, make_test_chat(1, "Alice"));
    resolver.users.insert(1, make_online_user(1, "alice"));

    let info = resolve_chat_info(&resolver, 1, ChatType::Private, "Alice".into()).unwrap();

    assert_eq!(info.status_line, "online");
    assert_eq!(info.username.as_deref(), Some("@alice"));
    assert_eq!(info.description, None);
}

#[test]
fn basic_group_reports_member_count_and_description() {
    let mut resolver = FakeResolver::default();
    resolver.chats.insert(5, make_basic_group_chat(5, 50));
    resolver
        .basic_groups
        .insert(50, make_basic_group_full_info(&[1, 2], "About us"));

    let info = resolve_chat_info(&resolver, 5, ChatType::Group, "Team".into()).unwrap();

    assert_eq!(info.status_line, "2 members");
    assert_eq!(info.description.as_deref(), Some("About us"));
}

#[test]
fn supergroup_reports_member_count() {
    let mut resolver = FakeResolver::default();
    resolver.chats.insert(6, make_supergroup_chat(6, 60, false));
    resolver
        .supergroups
        .insert(60, make_supergroup_full_info(42, ""));

    let info = resolve_chat_info(&resolver, 6, ChatType::Group, "Community".into()).unwrap();

    assert_eq!(info.status_line, "42 members");
    assert_eq!(info.description, None);
}

#[test]
fn channel_reports_subscriber_count() {
    let mut resolver = FakeResolver::default();
    resolver.chats.insert(7, make_supergroup_chat(7, 70, true));
    resolver
        .supergroups
        .insert(70, make_supergroup_full_info(1000, "Daily news"));

    let info = resolve_chat_info(&resolver, 7, ChatType::Channel, "News".into()).unwrap();

    assert_eq!(info.status_line, "1000 subscribers");
    assert_eq!(info.description.as_deref(), Some("Daily news"));
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_online_user(id: i64, username: &str) -> User {
    let mut user = make_test_user(id, "Alice");
    user.status = UserStatus::Online(UserStatusOnline { expires: 9999 });
    user.usernames = Some(Usernames {
        active_usernames: vec![username.to_owned()],
        disabled_usernames: vec![],
        editable_username: username.to_owned(),
        collectible_usernames: vec![],
    });
    user
}

fn make_basic_group_chat(chat_id: i64, basic_group_id: i64) -> Chat {
    let mut chat = make_test_chat(chat_id, "Group");
    chat.r#type = tdlib_rs::enums::ChatType::BasicGroup(ChatTypeBasicGroup { basic_group_id });
    chat
}

fn make_supergroup_chat(chat_id: i64, supergroup_id: i64, is_channel: bool) -> Chat {
    let mut chat = make_test_chat(chat_id, "Supergroup");
    chat.r#type = tdlib_rs::enums::ChatType::Supergroup(ChatTypeSupergroup {
        supergroup_id,
        is_channel,
    });
    chat
}

fn make_basic_group_full_info(member_ids: &[i64], description: &str) -> BasicGroupFullInfo {
    BasicGroupFullInfo {
        photo: None,
        description: description.to_owned(),
        creator_user_id: 0,
        members: member_ids
            .iter()
            .map(|&user_id| ChatMember {
                member_id: MessageSender::User(MessageSenderUser { user_id }),
                inviter_user_id: 0,
                joined_chat_date: 0,
                status: ChatMemberStatus::Member(ChatMemberStatusMember {
                    member_until_date: 0,
                }),
            })
            .collect(),
        can_hide_members: false,
        can_toggle_aggressive_anti_spam: false,
        invite_link: None,
        bot_commands: vec![],
    }
}

fn make_supergroup_full_info(member_count: i32, description: &str) -> SupergroupFullInfo {
    SupergroupFullInfo {
        photo: None,
        description: description.to_owned(),
        member_count,
        administrator_count: 0,
        restricted_count: 0,
        banned_count: 0,
        linked_chat_id: 0,
        direct_messages_chat_id: 0,
        slow_mode_delay: 0,
        slow_mode_delay_expires_in: 0.0,
        can_enable_paid_messages: false,
        can_enable_paid_reaction: false,
        can_get_members: false,
        has_hidden_members: false,
        can_hide_members: false,
        can_set_sticker_set: false,
        can_set_location: false,
        can_get_statistics: false,
        can_get_revenue_statistics: false,
        can_get_star_revenue_statistics: false,
        can_send_gift: false,
        can_toggle_aggressive_anti_spam: false,
        is_all_history_available: false,
        can_have_sponsored_messages: false,
        has_aggressive_anti_spam_enabled: false,
        has_paid_media_allowed: false,
        has_pinned_stories: false,
        gift_count: 0,
        my_boost_count: 0,
        unrestrict_boost_count: 0,
        outgoing_paid_message_star_count: 0,
        sticker_set_id: 0,
        custom_emoji_sticker_set_id: 0,
        location: None,
        invite_link: None,
        bot_commands: vec![],
        bot_verification: None,
        main_profile_tab: None,
        upgraded_from_basic_group_id: 0,
        upgraded_from_max_message_id: 0,
    }
}
