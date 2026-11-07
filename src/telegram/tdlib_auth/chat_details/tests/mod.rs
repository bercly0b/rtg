use std::cell::RefCell;
use std::collections::HashMap;

use chrono::{DateTime, Local, TimeZone};
use tdlib_rs::enums::{ChatMemberStatus, MessageSender, NotificationSettingsScope};
use tdlib_rs::types::{
    AcceptedGiftTypes, BasicGroup, BasicGroupFullInfo, BotInfo, Chat, ChatMember,
    ChatMemberStatusMember, ChatTypeBasicGroup, ChatTypePrivate, ChatTypeSecret,
    ChatTypeSupergroup, CountryInfo, GiftSettings, MessageSenderUser, ScopeNotificationSettings,
    SecretChat, Supergroup, SupergroupFullInfo, User, UserFullInfo, UserTypeBot, Usernames,
};

use crate::domain::chat::ChatType;
use crate::domain::chat_folder::ChatFolders;
use crate::domain::chat_info_state::ChatInfo;
use crate::telegram::tdlib_cache::tests::{make_test_chat, make_test_user};
use crate::telegram::tdlib_client::TdLibError;
use crate::usecases::chat_subtitle::{ChatInfoQuery, ChatSubtitleError};

use super::{resolve_chat_info, ChatInfoResolver};

mod bot;
mod channel;
mod chat_block;
mod failures;
mod group;
mod user;

// ---------------------------------------------------------------------------
// Fake resolver
// ---------------------------------------------------------------------------

/// Serves every request from its maps; anything missing fails like a TDLib
/// 404. Records the requests it gets so tests can check what was skipped.
#[derive(Default)]
struct FakeResolver {
    chats: HashMap<i64, Chat>,
    users: HashMap<i64, User>,
    user_full_infos: HashMap<i64, UserFullInfo>,
    basic_groups: HashMap<i64, BasicGroup>,
    basic_group_full_infos: HashMap<i64, BasicGroupFullInfo>,
    supergroups: HashMap<i64, Supergroup>,
    supergroup_full_infos: HashMap<i64, SupergroupFullInfo>,
    secret_chats: HashMap<i32, SecretChat>,
    scope_mute_for: Vec<(NotificationSettingsScope, i32)>,
    countries: Option<Vec<CountryInfo>>,
    requests: RefCell<Vec<&'static str>>,
}

impl FakeResolver {
    fn requested(&self, request: &str) -> bool {
        self.requests.borrow().contains(&request)
    }

    fn serve<K, V>(
        &self,
        request: &'static str,
        map: &HashMap<K, V>,
        id: K,
    ) -> Result<V, TdLibError>
    where
        K: std::hash::Hash + Eq + std::fmt::Display,
        V: Clone,
    {
        self.requests.borrow_mut().push(request);
        map.get(&id).cloned().ok_or_else(|| not_found(request, id))
    }
}

fn not_found(request: &str, id: impl std::fmt::Display) -> TdLibError {
    TdLibError::Request {
        code: 404,
        message: format!("{request} {id} not found"),
    }
}

impl ChatInfoResolver for FakeResolver {
    fn get_chat(&self, chat_id: i64) -> Result<Chat, TdLibError> {
        self.serve("getChat", &self.chats, chat_id)
    }

    fn get_user(&self, user_id: i64) -> Result<User, TdLibError> {
        self.serve("getUser", &self.users, user_id)
    }

    fn get_user_full_info(&self, user_id: i64) -> Result<UserFullInfo, TdLibError> {
        self.serve("getUserFullInfo", &self.user_full_infos, user_id)
    }

    fn get_basic_group(&self, basic_group_id: i64) -> Result<BasicGroup, TdLibError> {
        self.serve("getBasicGroup", &self.basic_groups, basic_group_id)
    }

    fn get_basic_group_full_info(
        &self,
        basic_group_id: i64,
    ) -> Result<BasicGroupFullInfo, TdLibError> {
        self.serve(
            "getBasicGroupFullInfo",
            &self.basic_group_full_infos,
            basic_group_id,
        )
    }

    fn get_supergroup(&self, supergroup_id: i64) -> Result<Supergroup, TdLibError> {
        self.serve("getSupergroup", &self.supergroups, supergroup_id)
    }

    fn get_supergroup_full_info(
        &self,
        supergroup_id: i64,
    ) -> Result<SupergroupFullInfo, TdLibError> {
        self.serve(
            "getSupergroupFullInfo",
            &self.supergroup_full_infos,
            supergroup_id,
        )
    }

    fn get_secret_chat(&self, secret_chat_id: i32) -> Result<SecretChat, TdLibError> {
        self.serve("getSecretChat", &self.secret_chats, secret_chat_id)
    }

    fn get_scope_notification_settings(
        &self,
        scope: NotificationSettingsScope,
    ) -> Result<ScopeNotificationSettings, TdLibError> {
        self.requests
            .borrow_mut()
            .push("getScopeNotificationSettings");
        self.scope_mute_for
            .iter()
            .find(|(s, _)| *s == scope)
            .map(|&(_, mute_for)| scope_notification_settings(mute_for))
            .ok_or_else(|| not_found("getScopeNotificationSettings", format!("{scope:?}")))
    }

    fn get_countries(&self) -> Result<Vec<CountryInfo>, TdLibError> {
        self.requests.borrow_mut().push("getCountries");
        self.countries
            .clone()
            .ok_or_else(|| not_found("getCountries", ""))
    }
}

// ---------------------------------------------------------------------------
// Resolution
// ---------------------------------------------------------------------------

fn now() -> DateTime<Local> {
    Local.with_ymd_and_hms(2026, 11, 6, 12, 0, 0).unwrap()
}

fn resolve(
    resolver: &FakeResolver,
    chat_id: i64,
    chat_type: ChatType,
) -> Result<ChatInfo, ChatSubtitleError> {
    resolve_with_folders(resolver, chat_id, chat_type, ChatFolders::default())
}

fn resolve_with_folders(
    resolver: &FakeResolver,
    chat_id: i64,
    chat_type: ChatType,
    folders: ChatFolders,
) -> Result<ChatInfo, ChatSubtitleError> {
    let query = ChatInfoQuery {
        chat_id,
        chat_type,
        title: "Title".to_owned(),
        folders,
    };
    resolve_chat_info(resolver, &query, now())
}

// ---------------------------------------------------------------------------
// Factories
// ---------------------------------------------------------------------------

fn private_chat(chat_id: i64, user_id: i64) -> Chat {
    let mut chat = make_test_chat(chat_id, "Private");
    chat.r#type = tdlib_rs::enums::ChatType::Private(ChatTypePrivate { user_id });
    chat
}

fn secret_chat(chat_id: i64, secret_chat_id: i32, user_id: i64) -> Chat {
    let mut chat = make_test_chat(chat_id, "Secret");
    chat.r#type = tdlib_rs::enums::ChatType::Secret(ChatTypeSecret {
        secret_chat_id,
        user_id,
    });
    chat
}

fn basic_group_chat(chat_id: i64, basic_group_id: i64) -> Chat {
    let mut chat = make_test_chat(chat_id, "Group");
    chat.r#type = tdlib_rs::enums::ChatType::BasicGroup(ChatTypeBasicGroup { basic_group_id });
    chat
}

fn supergroup_chat(chat_id: i64, supergroup_id: i64, is_channel: bool) -> Chat {
    let mut chat = make_test_chat(chat_id, "Supergroup");
    chat.r#type = tdlib_rs::enums::ChatType::Supergroup(ChatTypeSupergroup {
        supergroup_id,
        is_channel,
    });
    chat
}

fn user(id: i64) -> User {
    make_test_user(id, "Alice")
}

fn usernames(active: &[&str], collectible: &[&str]) -> Option<Usernames> {
    Some(Usernames {
        active_usernames: active.iter().map(|&u| u.to_owned()).collect(),
        disabled_usernames: vec![],
        editable_username: active.first().map(|&u| u.to_owned()).unwrap_or_default(),
        collectible_usernames: collectible.iter().map(|&u| u.to_owned()).collect(),
    })
}

fn bot_type() -> UserTypeBot {
    UserTypeBot {
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
    }
}

fn bot_user(id: i64, bot: UserTypeBot) -> User {
    let mut user = make_test_user(id, "Bot");
    user.r#type = tdlib_rs::enums::UserType::Bot(bot);
    user
}

fn user_full_info() -> UserFullInfo {
    UserFullInfo {
        personal_photo: None,
        photo: None,
        public_photo: None,
        block_list: None,
        can_be_called: false,
        supports_video_calls: false,
        has_private_calls: false,
        has_private_forwards: false,
        has_restricted_voice_and_video_note_messages: false,
        has_posted_to_profile_stories: false,
        has_sponsored_messages_enabled: false,
        need_phone_number_privacy_exception: false,
        set_chat_background: false,
        bio: None,
        birthdate: None,
        personal_chat_id: 0,
        gift_count: 0,
        group_in_common_count: 0,
        incoming_paid_message_star_count: 0,
        outgoing_paid_message_star_count: 0,
        gift_settings: GiftSettings {
            show_gift_button: false,
            accepted_gift_types: AcceptedGiftTypes {
                unlimited_gifts: false,
                limited_gifts: false,
                upgraded_gifts: false,
                gifts_from_channels: false,
                premium_subscription: false,
            },
        },
        bot_verification: None,
        main_profile_tab: None,
        first_profile_audio: None,
        rating: None,
        pending_rating: None,
        pending_rating_date: 0,
        note: None,
        business_info: None,
        bot_info: None,
    }
}

fn bot_details() -> BotInfo {
    BotInfo {
        short_description: String::new(),
        description: String::new(),
        photo: None,
        animation: None,
        menu_button: None,
        commands: vec![],
        privacy_policy_url: String::new(),
        default_group_administrator_rights: None,
        default_channel_administrator_rights: None,
        affiliate_program: None,
        web_app_background_light_color: -1,
        web_app_background_dark_color: -1,
        web_app_header_light_color: -1,
        web_app_header_dark_color: -1,
        verification_parameters: None,
        can_get_revenue_statistics: false,
        can_manage_emoji_status: false,
        has_media_previews: false,
        edit_commands_link: None,
        edit_description_link: None,
        edit_description_media_link: None,
        edit_settings_link: None,
    }
}

fn formatted(text: &str) -> Option<tdlib_rs::types::FormattedText> {
    Some(tdlib_rs::types::FormattedText {
        text: text.to_owned(),
        entities: vec![],
    })
}

fn member_status() -> ChatMemberStatus {
    ChatMemberStatus::Member(ChatMemberStatusMember {
        member_until_date: 0,
    })
}

fn basic_group(id: i64, status: ChatMemberStatus) -> BasicGroup {
    BasicGroup {
        id,
        member_count: 0,
        status,
        is_active: true,
        upgraded_to_supergroup_id: 0,
    }
}

fn basic_group_full_info(members: Vec<ChatMemberStatus>, description: &str) -> BasicGroupFullInfo {
    BasicGroupFullInfo {
        photo: None,
        description: description.to_owned(),
        creator_user_id: 0,
        members: members
            .into_iter()
            .zip(1..)
            .map(|(status, user_id)| ChatMember {
                member_id: MessageSender::User(MessageSenderUser { user_id }),
                inviter_user_id: 0,
                joined_chat_date: 0,
                status,
            })
            .collect(),
        can_hide_members: false,
        can_toggle_aggressive_anti_spam: false,
        invite_link: None,
        bot_commands: vec![],
    }
}

fn supergroup(id: i64, is_channel: bool) -> Supergroup {
    Supergroup {
        id,
        usernames: None,
        date: 0,
        status: member_status(),
        member_count: 0,
        boost_level: 0,
        has_automatic_translation: false,
        has_linked_chat: false,
        has_location: false,
        sign_messages: false,
        show_message_sender: false,
        join_to_send_messages: true,
        join_by_request: false,
        is_slow_mode_enabled: false,
        is_channel,
        is_broadcast_group: false,
        is_forum: false,
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

fn supergroup_full_info(member_count: i32, description: &str) -> SupergroupFullInfo {
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

fn invite_link(link: &str) -> Option<tdlib_rs::types::ChatInviteLink> {
    Some(tdlib_rs::types::ChatInviteLink {
        invite_link: link.to_owned(),
        name: String::new(),
        creator_user_id: 0,
        date: 0,
        edit_date: 0,
        expiration_date: 0,
        subscription_pricing: None,
        member_limit: 0,
        member_count: 0,
        expired_member_count: 0,
        pending_join_request_count: 0,
        creates_join_request: false,
        is_primary: true,
        is_revoked: false,
    })
}

fn verified() -> Option<tdlib_rs::types::VerificationStatus> {
    Some(tdlib_rs::types::VerificationStatus {
        is_verified: true,
        is_scam: false,
        is_fake: false,
        bot_verification_icon_custom_emoji_id: 0,
    })
}

fn scope_notification_settings(mute_for: i32) -> ScopeNotificationSettings {
    ScopeNotificationSettings {
        mute_for,
        sound_id: 0,
        show_preview: true,
        use_default_mute_stories: true,
        mute_stories: false,
        story_sound_id: 0,
        show_story_poster: false,
        disable_pinned_message_notifications: false,
        disable_mention_notifications: false,
    }
}

fn country(code: &str, english_name: &str) -> CountryInfo {
    CountryInfo {
        country_code: code.to_owned(),
        name: english_name.to_owned(),
        english_name: english_name.to_owned(),
        is_hidden: false,
        calling_codes: vec![],
    }
}
