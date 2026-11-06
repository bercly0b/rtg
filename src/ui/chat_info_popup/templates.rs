//! The info templates: which blocks and rows each kind of chat shows, in
//! which order.

use crate::domain::chat_info_state::{
    AccountInfo, Badge, BotInfo, BusinessInfo, ChannelInfo, ChatBlockInfo, ChatInfo, ChatInfoKind,
    GroupInfo, MemberRole, SecretChatInfo, UserInfo,
};
use crate::domain::message_info_state::format_unix_timestamp;

use super::format;

pub(super) struct InfoBlock {
    pub title: &'static str,
    pub rows: Vec<InfoRow>,
}

pub(super) struct InfoRow {
    pub label: &'static str,
    pub value: Value,
}

pub(super) enum Value {
    Text(String),
    /// Not set or unknown.
    Missing,
    /// Certainly hidden by the other side's privacy settings.
    Hidden,
    Badges(Vec<Badge>),
}

impl From<String> for Value {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

impl From<&str> for Value {
    fn from(text: &str) -> Self {
        Self::Text(text.to_owned())
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(value: Option<T>) -> Self {
        value.map_or(Self::Missing, Into::into)
    }
}

fn row(label: &'static str, value: impl Into<Value>) -> InfoRow {
    InfoRow {
        label,
        value: value.into(),
    }
}

fn badges_row(badges: &[Badge]) -> InfoRow {
    let value = if badges.is_empty() {
        Value::Missing
    } else {
        Value::Badges(badges.to_vec())
    };
    row("Badges", value)
}

fn block(title: &'static str, rows: Vec<InfoRow>) -> InfoBlock {
    InfoBlock { title, rows }
}

/// The blocks of the chat's template, then the Chat block.
pub(super) fn blocks(info: &ChatInfo) -> Vec<InfoBlock> {
    let mut blocks = match &info.kind {
        ChatInfoKind::User(user) => user_blocks(user),
        ChatInfoKind::Bot(bot) => vec![bot_block(bot)],
        ChatInfoKind::Group(group) => group_blocks(group),
        ChatInfoKind::Channel(channel) => channel_blocks(channel),
    };
    blocks.push(chat_block(&info.chat));
    blocks
}

fn user_blocks(user: &UserInfo) -> Vec<InfoBlock> {
    let phone = match &user.phone {
        Some(phone) => Value::Text(format!("+{phone}")),
        None => Value::Hidden,
    };
    let mut blocks = vec![
        block(
            "Profile",
            vec![
                row("User ID", user.user_id.to_string()),
                row("Username", format::usernames(&user.usernames)),
                row("Phone", phone),
                row("Birthday", user.birthdate.map(format::birthdate)),
                row("Channel", user.personal_channel.clone()),
                badges_row(&user.badges),
                row("Bio", user.bio.clone()),
            ],
        ),
        block(
            "You & them",
            vec![
                row("Contact", format::contact(user.contact)),
                row(
                    "In common",
                    user.groups_in_common
                        .map(|n| format::count(n, "group", "groups")),
                ),
                row(
                    "Messages",
                    user.paid_message_stars.map(format::paid_messages),
                ),
                row("Blocked", user.blocked.map(format::block_state)),
                row("Note", user.note.clone()),
            ],
        ),
    ];
    if let Some(account_info) = &user.account_info {
        blocks.push(account_info_block(account_info));
    }
    if let Some(business) = &user.business {
        blocks.push(business_block(business));
    }
    if let Some(secret_chat) = &user.secret_chat {
        blocks.push(secret_chat_block(secret_chat));
    }
    blocks
}

fn account_info_block(info: &AccountInfo) -> InfoBlock {
    block(
        "Account info",
        vec![
            row("Registered", info.registered.map(format::month_year)),
            row("Phone from", info.phone_country.clone()),
            row("Name changed", info.name_changed.map(format_unix_timestamp)),
            row(
                "Photo changed",
                info.photo_changed.map(format_unix_timestamp),
            ),
        ],
    )
}

fn business_block(business: &BusinessInfo) -> InfoBlock {
    block(
        "Business",
        vec![
            row("Address", business.address.clone()),
            row("Hours", business.hours.map(format::business_hours)),
        ],
    )
}

fn secret_chat_block(secret_chat: &SecretChatInfo) -> InfoBlock {
    let key = (!secret_chat.key_hash.is_empty()).then(|| format::key_hash(&secret_chat.key_hash));
    block(
        "Secret chat",
        vec![
            row("State", secret_chat.state.map(format::secret_chat_state)),
            row("Key", key),
        ],
    )
}

fn bot_block(bot: &BotInfo) -> InfoBlock {
    block(
        "Bot",
        vec![
            row("User ID", bot.user_id.to_string()),
            row("Username", format::usernames(&bot.usernames)),
            row("Commands", format::commands(&bot.commands)),
            row("Groups", format::bot_groups(bot.groups)),
            row("Inline", format::inline(bot.inline_placeholder.as_deref())),
            row("Privacy", bot.privacy_policy_url.clone()),
            badges_row(&bot.badges),
            row("Blocked", bot.blocked.map(format::block_state)),
            row("About", bot.about.clone()),
            row("Description", bot.description.clone()),
        ],
    )
}

fn group_blocks(group: &GroupInfo) -> Vec<InfoBlock> {
    let mut blocks = vec![
        block(
            "Group",
            vec![
                row("Link", group.link.as_deref().map(format::link)),
                row("Kind", format::group_kind(group.kind)),
                row("Admins", group.admins.map(|n| n.to_string())),
                row("Channel", group.linked_channel.clone()),
                row(
                    "Slow mode",
                    group.slow_mode_delay.map(format::off_or_period),
                ),
                row("Joining", group.joining.map(format::joining)),
                row("History", group.history_visible.map(format::history)),
                row(
                    "Forwarding",
                    format::forwarding(group.has_protected_content),
                ),
                row("Invite link", group.invite_link.clone()),
                badges_row(&group.badges),
                row("Description", group.description.clone()),
            ],
        ),
        you_block(group.role.as_ref(), "member", group.joined),
    ];
    if let Some(address) = &group.location {
        blocks.push(block("Location", vec![row("Address", address.clone())]));
    }
    blocks
}

fn channel_blocks(channel: &ChannelInfo) -> Vec<InfoBlock> {
    vec![
        block(
            "Channel",
            vec![
                row("Link", channel.link.as_deref().map(format::link)),
                row("Discussion", channel.discussion.clone()),
                row("Signatures", channel.signatures.map(format::on_off)),
                row(
                    "Forwarding",
                    format::forwarding(channel.has_protected_content),
                ),
                row("Admins", channel.admins.map(|n| n.to_string())),
                badges_row(&channel.badges),
                row("Description", channel.description.clone()),
            ],
        ),
        you_block(channel.role.as_ref(), "subscriber", channel.joined),
    ]
}

fn you_block(role: Option<&MemberRole>, member_word: &str, joined: Option<i32>) -> InfoBlock {
    block(
        "You",
        vec![
            row("Role", role.map(|role| format::role(role, member_word))),
            row("Joined", joined.map(format::date)),
        ],
    )
}

fn chat_block(chat: &ChatBlockInfo) -> InfoBlock {
    let folders = (!chat.folders.is_empty()).then(|| chat.folders.join(", "));
    block(
        "Chat",
        vec![
            row("Chat ID", chat.chat_id.to_string()),
            row("Folders", folders),
            row("Muted", chat.muted.map(format::mute)),
            row(
                "Auto-delete",
                format::off_or_period(chat.auto_delete_seconds),
            ),
        ],
    )
}
