//! The User and Bot info templates: private and secret chats.

use base64::Engine;
use chrono::{DateTime, Local};
use tdlib_rs::enums::{BlockList, ChatActionBar, ChatType as TdChatType, UserStatus, UserType};
use tdlib_rs::types::{Chat, User, UserFullInfo, UserTypeBot, Usernames};

use crate::domain::chat::ChatType;
use crate::domain::chat_info_state::{
    AccountInfo, Badge, Birthdate, BlockState, BotGroupAccess, BotInfo, BusinessHours,
    BusinessInfo, ChatInfoKind, ContactStatus, MonthYear, SecretChatInfo, SecretChatState,
    UserInfo, Username,
};
use crate::telegram::tdlib_mappers;
use crate::usecases::chat_subtitle::ChatSubtitleError;

use super::{
    chat_title, non_empty, non_zero, optional, request_failed, unexpected_chat_type,
    verification_badges, ChatInfoResolver,
};

/// Only this much of a secret chat's key hash is meant to be compared by hand.
const COMPARABLE_KEY_HASH_LEN: usize = 32;

pub(super) fn resolve(
    resolver: &dyn ChatInfoResolver,
    chat: &Chat,
    now: DateTime<Local>,
) -> Result<(String, ChatInfoKind), ChatSubtitleError> {
    let Some(user_id) = tdlib_mappers::get_private_chat_user_id(&chat.r#type) else {
        return Err(unexpected_chat_type(ChatType::Private, &chat.r#type));
    };
    let user = resolver
        .get_user(user_id)
        .map_err(|e| request_failed("getUser", user_id, e))?;
    let full_info = optional("getUserFullInfo", resolver.get_user_full_info(user_id));

    if let UserType::Bot(bot) = &user.r#type {
        let info = bot_info(&user, bot, full_info.as_ref());
        return Ok((bot_status_line(bot), ChatInfoKind::Bot(info)));
    }

    let info = user_info(resolver, chat, &user, full_info.as_ref());
    Ok((
        user_status_line(&user, now),
        ChatInfoKind::User(Box::new(info)),
    ))
}

fn user_status_line(user: &User, now: DateTime<Local>) -> String {
    if matches!(user.r#type, UserType::Deleted | UserType::Unknown) {
        return "deleted account".to_owned();
    }
    let status = tdlib_mappers::map_user_status_to_subtitle(&user.status).format(now);
    let hidden_by_my_settings = match &user.status {
        UserStatus::Recently(s) => s.by_my_privacy_settings,
        UserStatus::LastWeek(s) => s.by_my_privacy_settings,
        UserStatus::LastMonth(s) => s.by_my_privacy_settings,
        UserStatus::Empty | UserStatus::Online(_) | UserStatus::Offline(_) => false,
    };
    if hidden_by_my_settings {
        format!("{status} (you hide yours)")
    } else {
        status
    }
}

fn bot_status_line(bot: &UserTypeBot) -> String {
    match bot.active_user_count {
        0 => "bot".to_owned(),
        1 => "bot · 1 monthly user".to_owned(),
        count => format!("bot · {count} monthly users"),
    }
}

fn user_info(
    resolver: &dyn ChatInfoResolver,
    chat: &Chat,
    user: &User,
    full_info: Option<&UserFullInfo>,
) -> UserInfo {
    UserInfo {
        user_id: user.id,
        usernames: usernames(user.usernames.as_ref()),
        phone: non_empty(&user.phone_number),
        birthdate: full_info
            .and_then(|f| f.birthdate.as_ref())
            .map(|b| Birthdate {
                day: u32::try_from(b.day).unwrap_or_default(),
                month: u32::try_from(b.month).unwrap_or_default(),
                year: non_zero(b.year),
            }),
        personal_channel: full_info.and_then(|f| chat_title(resolver, f.personal_chat_id)),
        badges: user_badges(user),
        bio: full_info
            .and_then(|f| f.bio.as_ref())
            .and_then(|bio| non_empty(&bio.text)),
        contact: ContactStatus {
            is_contact: user.is_contact,
            is_mutual: user.is_mutual_contact,
            is_close_friend: user.is_close_friend,
        },
        groups_in_common: full_info.map(|f| f.group_in_common_count),
        paid_message_stars: full_info.map(|f| f.outgoing_paid_message_star_count),
        blocked: full_info.map(|f| block_state(f.block_list.as_ref())),
        note: full_info
            .and_then(|f| f.note.as_ref())
            .and_then(|note| non_empty(&note.text)),
        account_info: account_info(resolver, chat),
        business: full_info
            .and_then(|f| f.business_info.as_ref())
            .and_then(business_info),
        secret_chat: secret_chat_info(resolver, chat),
    }
}

fn bot_info(user: &User, bot: &UserTypeBot, full_info: Option<&UserFullInfo>) -> BotInfo {
    let details = full_info.and_then(|f| f.bot_info.as_ref());
    BotInfo {
        user_id: user.id,
        usernames: usernames(user.usernames.as_ref()),
        commands: details
            .map(|d| d.commands.iter().map(|c| c.command.clone()).collect())
            .unwrap_or_default(),
        groups: match (bot.can_join_groups, bot.can_read_all_group_messages) {
            (false, _) => BotGroupAccess::CannotJoin,
            (true, false) => BotGroupAccess::CanJoin,
            (true, true) => BotGroupAccess::ReadsAllMessages,
        },
        inline_placeholder: bot.is_inline.then(|| bot.inline_query_placeholder.clone()),
        privacy_policy_url: details.and_then(|d| non_empty(&d.privacy_policy_url)),
        badges: user_badges(user),
        blocked: full_info.map(|f| block_state(f.block_list.as_ref())),
        about: details.and_then(|d| non_empty(&d.short_description)),
        description: details.and_then(|d| non_empty(&d.description)),
    }
}

fn usernames(usernames: Option<&Usernames>) -> Vec<Username> {
    let Some(usernames) = usernames else {
        return Vec::new();
    };
    usernames
        .active_usernames
        .iter()
        .map(|name| Username {
            name: name.clone(),
            is_collectible: usernames.collectible_usernames.contains(name),
        })
        .collect()
}

fn user_badges(user: &User) -> Vec<Badge> {
    let mut badges = verification_badges(user.verification_status.as_ref());
    if user.is_premium {
        badges.push(Badge::Premium);
    }
    if user.is_support {
        badges.push(Badge::Support);
    }
    badges
}

fn block_state(block_list: Option<&BlockList>) -> BlockState {
    match block_list {
        None => BlockState::NotBlocked,
        Some(BlockList::Main) => BlockState::Blocked,
        Some(BlockList::Stories) => BlockState::StoriesOnly,
    }
}

/// TDLib keeps account info only while the chat's action bar offers to
/// report, add or block the user.
fn account_info(resolver: &dyn ChatInfoResolver, chat: &Chat) -> Option<AccountInfo> {
    let Some(ChatActionBar::ReportAddBlock(bar)) = &chat.action_bar else {
        return None;
    };
    let info = bar.account_info.as_ref()?;
    Some(AccountInfo {
        registered: non_zero(info.registration_year).map(|year| MonthYear {
            month: u32::try_from(info.registration_month)
                .ok()
                .filter(|&month| month != 0),
            year,
        }),
        phone_country: phone_country(resolver, &info.phone_number_country_code),
        name_changed: non_zero(info.last_name_change_date),
        photo_changed: non_zero(info.last_photo_change_date),
    })
}

fn phone_country(resolver: &dyn ChatInfoResolver, country_code: &str) -> Option<String> {
    if country_code.is_empty() {
        return None;
    }
    let name = optional("getCountries", resolver.get_countries())
        .and_then(|countries| {
            countries
                .into_iter()
                .find(|country| country.country_code == country_code)
        })
        .map(|country| country.english_name);
    Some(name.unwrap_or_else(|| country_code.to_owned()))
}

fn business_info(info: &tdlib_rs::types::BusinessInfo) -> Option<BusinessInfo> {
    if info.location.is_none() && info.opening_hours.is_none() {
        return None;
    }
    Some(BusinessInfo {
        address: info
            .location
            .as_ref()
            .and_then(|location| non_empty(&location.address)),
        hours: info
            .opening_hours
            .as_ref()
            .and_then(|_| business_hours(info.next_open_in, info.next_close_in)),
    })
}

/// Whichever of the next opening and closing comes first tells whether the
/// business is open now.
fn business_hours(next_open_in: i32, next_close_in: i32) -> Option<BusinessHours> {
    match (next_open_in, next_close_in) {
        (0, 0) => None,
        (open_in, close_in) if close_in > 0 && (open_in == 0 || close_in < open_in) => {
            Some(BusinessHours::OpenClosesIn(close_in))
        }
        (open_in, _) => Some(BusinessHours::ClosedOpensIn(open_in)),
    }
}

fn secret_chat_info(resolver: &dyn ChatInfoResolver, chat: &Chat) -> Option<SecretChatInfo> {
    let TdChatType::Secret(secret) = &chat.r#type else {
        return None;
    };
    let secret_chat = optional(
        "getSecretChat",
        resolver.get_secret_chat(secret.secret_chat_id),
    );
    Some(SecretChatInfo {
        state: secret_chat.as_ref().map(|c| match c.state {
            tdlib_rs::enums::SecretChatState::Pending => SecretChatState::Pending,
            tdlib_rs::enums::SecretChatState::Ready => SecretChatState::Ready,
            tdlib_rs::enums::SecretChatState::Closed => SecretChatState::Closed,
        }),
        key_hash: secret_chat
            .map(|c| comparable_key_hash(&c.key_hash))
            .unwrap_or_default(),
    })
}

/// TDLib sends `bytes` fields base64-encoded.
fn comparable_key_hash(encoded: &str) -> Vec<u8> {
    let mut bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .unwrap_or_default();
    bytes.truncate(COMPARABLE_KEY_HASH_LEN);
    bytes
}
