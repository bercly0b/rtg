//! How the chat info popup writes values.

use chrono::{Local, Month, TimeZone};

use crate::domain::chat_info_state::{
    Badge, Birthdate, BlockState, BotGroupAccess, BusinessHours, ContactStatus, GroupKind, Joining,
    MemberRole, MonthYear, MuteState, SecretChatState, Username,
};
use crate::domain::message_info_state::format_unix_timestamp;

const MINUTE: i32 = 60;
const HOUR: i32 = 60 * MINUTE;
const DAY: i32 = 24 * HOUR;
const WEEK: i32 = 7 * DAY;
/// Telegram's "1 month" auto-delete period.
const MONTH: i32 = 31 * DAY;
const YEAR: i32 = 365 * DAY;

/// Largest first: a period is named in the largest unit that divides it.
const UNITS: [(i32, &str, &str); 7] = [
    (YEAR, "year", "years"),
    (MONTH, "month", "months"),
    (WEEK, "week", "weeks"),
    (DAY, "day", "days"),
    (HOUR, "h", "h"),
    (MINUTE, "min", "min"),
    (1, "s", "s"),
];

pub(super) fn count(n: impl Into<i64>, singular: &str, plural: &str) -> String {
    let n = n.into();
    if n == 1 {
        format!("1 {singular}")
    } else {
        format!("{n} {plural}")
    }
}

pub(super) fn usernames(usernames: &[Username]) -> Option<String> {
    if usernames.is_empty() {
        return None;
    }
    let names: Vec<String> = usernames
        .iter()
        .map(|username| {
            if username.is_collectible {
                format!("@{} (collectible)", username.name)
            } else {
                format!("@{}", username.name)
            }
        })
        .collect();
    Some(names.join(", "))
}

pub(super) fn badge(badge: Badge) -> &'static str {
    match badge {
        Badge::Verified => "verified",
        Badge::Scam => "scam",
        Badge::Fake => "fake",
        Badge::Premium => "premium",
        Badge::Support => "support",
    }
}

pub(super) fn birthdate(birthdate: Birthdate) -> String {
    let month = month_name(birthdate.month);
    match birthdate.year {
        Some(year) => format!("{} {month} {year}", birthdate.day),
        None => format!("{} {month}", birthdate.day),
    }
}

pub(super) fn month_year(date: MonthYear) -> String {
    match date.month {
        Some(month) => format!("{} {}", month_name(month), date.year),
        None => date.year.to_string(),
    }
}

fn month_name(month: u32) -> &'static str {
    u8::try_from(month)
        .ok()
        .and_then(|month| Month::try_from(month).ok())
        .map_or("?", |month| month.name())
}

pub(super) fn date(unix_time: i32) -> String {
    Local
        .timestamp_opt(i64::from(unix_time), 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "unknown".to_owned())
}

pub(super) fn off_or_period(seconds: i32) -> String {
    if seconds == 0 {
        "off".to_owned()
    } else {
        period(seconds)
    }
}

/// A period set in whole units, such as auto-delete or slow mode.
fn period(seconds: i32) -> String {
    in_unit(seconds, |size| seconds % size == 0)
}

/// Time left until something happens, rounded down to its largest unit.
fn time_left(seconds: i32) -> String {
    in_unit(seconds, |size| seconds >= size)
}

fn in_unit(seconds: i32, fits: impl Fn(i32) -> bool) -> String {
    let (size, singular, plural) = UNITS
        .into_iter()
        .find(|&(size, ..)| fits(size))
        .unwrap_or(UNITS[UNITS.len() - 1]);
    count(seconds / size, singular, plural)
}

pub(super) fn mute(state: MuteState) -> String {
    match state {
        MuteState::Unmuted => "no".to_owned(),
        MuteState::Forever => "yes".to_owned(),
        MuteState::Until(unix_time) => format!("until {}", format_unix_timestamp(unix_time)),
    }
}

pub(super) fn contact(status: ContactStatus) -> String {
    let relation = if status.is_mutual {
        "mutual"
    } else if status.is_contact {
        "contact"
    } else {
        "not a contact"
    };
    if status.is_close_friend {
        format!("{relation}, close friend")
    } else {
        relation.to_owned()
    }
}

pub(super) fn paid_messages(stars: i64) -> String {
    if stars == 0 {
        "free".to_owned()
    } else {
        format!("paid, {} each", count(stars, "star", "stars"))
    }
}

pub(super) fn block_state(state: BlockState) -> &'static str {
    match state {
        BlockState::NotBlocked => "no",
        BlockState::Blocked => "yes",
        BlockState::StoriesOnly => "stories only",
    }
}

pub(super) fn business_hours(hours: BusinessHours) -> String {
    match hours {
        BusinessHours::OpenClosesIn(seconds) => format!("open, closes in {}", time_left(seconds)),
        BusinessHours::ClosedOpensIn(seconds) => format!("closed, opens in {}", time_left(seconds)),
    }
}

pub(super) fn secret_chat_state(state: SecretChatState) -> &'static str {
    match state {
        SecretChatState::Pending => "pending",
        SecretChatState::Ready => "ready",
        SecretChatState::Closed => "closed",
    }
}

/// Hex in groups of two bytes, the way native clients print the key.
pub(super) fn key_hash(bytes: &[u8]) -> String {
    let groups: Vec<String> = bytes
        .chunks(2)
        .map(|pair| pair.iter().map(|byte| format!("{byte:02x}")).collect())
        .collect();
    groups.join(" ")
}

pub(super) fn bot_groups(access: BotGroupAccess) -> &'static str {
    match access {
        BotGroupAccess::CannotJoin => "cannot join",
        BotGroupAccess::CanJoin => "can join",
        BotGroupAccess::ReadsAllMessages => "can join, reads all messages",
    }
}

pub(super) fn inline(placeholder: Option<&str>) -> String {
    match placeholder {
        None => "no".to_owned(),
        Some("") => "yes".to_owned(),
        Some(placeholder) => format!("yes, \"{placeholder}\""),
    }
}

pub(super) fn commands(commands: &[String]) -> Option<String> {
    if commands.is_empty() {
        return None;
    }
    let commands: Vec<String> = commands.iter().map(|c| format!("/{c}")).collect();
    Some(commands.join(" "))
}

pub(super) fn link(username: &str) -> String {
    format!("t.me/{username}")
}

pub(super) fn group_kind(kind: GroupKind) -> &'static str {
    match kind {
        GroupKind::BasicGroup => "basic group",
        GroupKind::Supergroup => "supergroup",
        GroupKind::Forum => "supergroup, forum",
        GroupKind::BroadcastGroup => "supergroup, broadcast group",
    }
}

pub(super) fn joining(joining: Joining) -> &'static str {
    match joining {
        Joining::ByRequest => "by request",
        Joining::Open => "open",
        Joining::InviteOnly => "invite only",
    }
}

pub(super) fn history(visible: bool) -> &'static str {
    if visible {
        "visible to new members"
    } else {
        "hidden from new members"
    }
}

pub(super) fn forwarding(has_protected_content: bool) -> &'static str {
    if has_protected_content {
        "restricted"
    } else {
        "allowed"
    }
}

pub(super) fn on_off(on: bool) -> &'static str {
    if on {
        "on"
    } else {
        "off"
    }
}

/// `member_word` is "member" in groups and "subscriber" in channels.
pub(super) fn role(role: &MemberRole, member_word: &str) -> String {
    match role {
        MemberRole::Owner => "owner".to_owned(),
        MemberRole::Admin { custom_title } if custom_title.is_empty() => "admin".to_owned(),
        MemberRole::Admin { custom_title } => format!("admin \"{custom_title}\""),
        MemberRole::Member => member_word.to_owned(),
        MemberRole::Restricted { until: 0 } => "restricted".to_owned(),
        MemberRole::Restricted { until } => {
            format!("restricted until {}", format_unix_timestamp(*until))
        }
        MemberRole::Left => "left".to_owned(),
        MemberRole::Banned => "banned".to_owned(),
    }
}
