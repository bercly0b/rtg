//! Chat info popup state — what is known about a chat and whoever stands
//! behind it.
//!
//! Opened by pressing `I` on a chat in the chat list. Each kind of chat has
//! its own info template (User, Bot, Group, Channel); the design lives in
//! `docs/features/chat-info-user-data.md`.
//!
//! `None` in a field means the value is not set or unknown: the popup keeps
//! the row and shows `—`.

/// Resolved chat information for display in the popup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatInfo {
    pub title: String,
    /// Header line under the title, as in the open chat's subtitle.
    pub status_line: String,
    pub kind: ChatInfoKind,
    pub chat: ChatBlockInfo,
}

/// The info template the popup uses, with the data for its blocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatInfoKind {
    User(Box<UserInfo>),
    Bot(BotInfo),
    Group(GroupInfo),
    Channel(ChannelInfo),
}

/// The Chat block every template ends with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatBlockInfo {
    pub chat_id: i64,
    /// Titles of the user's folders holding the chat, plus `Archive`.
    pub folders: Vec<String>,
    pub muted: Option<MuteState>,
    /// Message auto-delete period in seconds; 0 when off.
    pub auto_delete_seconds: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MuteState {
    Unmuted,
    Forever,
    /// Muted until this Unix time.
    Until(i64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Username {
    pub name: String,
    pub is_collectible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Badge {
    Verified,
    Scam,
    Fake,
    Premium,
    Support,
}

impl Badge {
    /// Scam and fake marks warn the user rather than describe the account.
    pub fn is_warning(self) -> bool {
        matches!(self, Self::Scam | Self::Fake)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockState {
    NotBlocked,
    Blocked,
    StoriesOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserInfo {
    pub user_id: i64,
    pub usernames: Vec<Username>,
    pub phone: Option<PhoneNumber>,
    pub birthdate: Option<Birthdate>,
    /// Title of the user's personal channel.
    pub personal_channel: Option<String>,
    pub badges: Vec<Badge>,
    pub bio: Option<String>,
    pub contact: ContactStatus,
    pub groups_in_common: Option<i32>,
    /// Stars the current user pays for each message to the user; 0 if free.
    pub paid_message_stars: Option<i64>,
    pub blocked: Option<BlockState>,
    pub note: Option<String>,
    pub account_info: Option<AccountInfo>,
    pub business: Option<BusinessInfo>,
    pub secret_chat: Option<SecretChatInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhoneNumber {
    /// Digits without the leading `+`.
    Shown(String),
    /// Hidden by privacy settings: every account has a number.
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Birthdate {
    pub day: u32,
    pub month: u32,
    pub year: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ContactStatus {
    pub is_contact: bool,
    pub is_mutual: bool,
    pub is_close_friend: bool,
}

/// What Telegram reveals about a non-contact who wrote first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountInfo {
    pub registered: Option<MonthYear>,
    /// Country name of the phone number, or its ISO code if the name is
    /// unavailable.
    pub phone_country: Option<String>,
    pub name_changed: Option<i32>,
    pub photo_changed: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonthYear {
    pub month: Option<u32>,
    pub year: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BusinessInfo {
    pub address: Option<String>,
    pub hours: Option<BusinessHours>,
}

/// Whether a business is open now, with seconds until that changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusinessHours {
    OpenClosesIn(i32),
    ClosedOpensIn(i32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretChatInfo {
    pub state: Option<SecretChatState>,
    /// The part of the key hash meant to be compared by hand; empty if unknown.
    pub key_hash: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretChatState {
    Pending,
    Ready,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BotInfo {
    pub user_id: i64,
    pub usernames: Vec<Username>,
    /// Command names without the leading `/`.
    pub commands: Vec<String>,
    pub groups: BotGroupAccess,
    /// `Some` when the bot supports inline queries, with the input field
    /// placeholder (may be empty).
    pub inline_placeholder: Option<String>,
    pub privacy_policy_url: Option<String>,
    pub badges: Vec<Badge>,
    pub blocked: Option<BlockState>,
    pub about: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotGroupAccess {
    CannotJoin,
    CanJoin,
    /// Can join with privacy mode off.
    ReadsAllMessages,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupInfo {
    /// Public username.
    pub link: Option<String>,
    pub kind: Option<GroupKind>,
    pub admins: Option<i32>,
    /// Title of the linked channel.
    pub linked_channel: Option<String>,
    /// Seconds between messages; 0 when off.
    pub slow_mode_delay: Option<i32>,
    pub joining: Option<Joining>,
    pub history_visible: Option<bool>,
    pub has_protected_content: bool,
    pub invite_link: Option<String>,
    pub badges: Vec<Badge>,
    pub description: Option<String>,
    pub role: Option<MemberRole>,
    /// Unix time the current user joined.
    pub joined: Option<i32>,
    /// Address of a location-based group.
    pub location: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupKind {
    BasicGroup,
    Supergroup,
    Forum,
    BroadcastGroup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Joining {
    ByRequest,
    Open,
    InviteOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelInfo {
    /// Public username.
    pub link: Option<String>,
    /// Title of the discussion group.
    pub discussion: Option<String>,
    pub signatures: Option<bool>,
    pub has_protected_content: bool,
    pub admins: Option<i32>,
    pub badges: Vec<Badge>,
    pub description: Option<String>,
    pub role: Option<MemberRole>,
    /// Unix time the current user joined.
    pub joined: Option<i32>,
}

/// The current user's status in a group or channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemberRole {
    Owner,
    Admin {
        custom_title: String,
    },
    Member,
    /// Restricted until this Unix time; 0 if forever.
    Restricted {
        until: i32,
    },
    Left,
    Banned,
}

/// State of the chat info popup overlay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatInfoPopupState {
    /// Data is being fetched from Telegram.
    Loading { chat_id: i64, title: String },
    /// Data has been resolved.
    Loaded(Box<ChatInfo>),
    /// Fetching failed.
    Error { title: String },
}

impl ChatInfoPopupState {
    pub fn title(&self) -> &str {
        match self {
            Self::Loading { title, .. } | Self::Error { title } => title,
            Self::Loaded(info) => &info.title,
        }
    }

    /// Returns the `chat_id` this popup was opened for (only available in Loading state).
    pub fn chat_id(&self) -> Option<i64> {
        match self {
            Self::Loading { chat_id, .. } => Some(*chat_id),
            _ => None,
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn sample_chat_info(title: &str) -> ChatInfo {
        ChatInfo {
            title: title.into(),
            status_line: "42 members".into(),
            kind: ChatInfoKind::Channel(ChannelInfo {
                link: None,
                discussion: None,
                signatures: None,
                has_protected_content: false,
                admins: None,
                badges: vec![],
                description: None,
                role: None,
                joined: None,
            }),
            chat: ChatBlockInfo {
                chat_id: 1,
                folders: vec![],
                muted: None,
                auto_delete_seconds: 0,
            },
        }
    }

    #[test]
    fn loading_state_returns_title() {
        let state = ChatInfoPopupState::Loading {
            chat_id: 1,
            title: "Alice".into(),
        };
        assert_eq!(state.title(), "Alice");
    }

    #[test]
    fn loading_state_returns_chat_id() {
        let state = ChatInfoPopupState::Loading {
            chat_id: 42,
            title: "Alice".into(),
        };
        assert_eq!(state.chat_id(), Some(42));
    }

    #[test]
    fn loaded_state_returns_no_chat_id() {
        let state = ChatInfoPopupState::Loaded(Box::new(sample_chat_info("Alice")));
        assert_eq!(state.chat_id(), None);
    }

    #[test]
    fn loaded_state_returns_title() {
        let state = ChatInfoPopupState::Loaded(Box::new(sample_chat_info("Bob")));
        assert_eq!(state.title(), "Bob");
    }

    #[test]
    fn error_state_returns_title() {
        let state = ChatInfoPopupState::Error {
            title: "Group".into(),
        };
        assert_eq!(state.title(), "Group");
    }

    #[test]
    fn only_scam_and_fake_badges_are_warnings() {
        let warnings: Vec<Badge> = [
            Badge::Verified,
            Badge::Scam,
            Badge::Fake,
            Badge::Premium,
            Badge::Support,
        ]
        .into_iter()
        .filter(|badge| badge.is_warning())
        .collect();
        assert_eq!(warnings, vec![Badge::Scam, Badge::Fake]);
    }
}
