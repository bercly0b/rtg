//! Chat info popup state — displays user/group/channel details.
//!
//! Opened by pressing `I` on a chat in the chat list.
//! Shows different information depending on the chat type:
//! - Private/Bot: name, online status or "bot", bio
//! - Group: name, member count, description
//! - Channel: name, subscriber count, description

use super::chat::ChatType;

/// Resolved chat information for display in the popup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatInfo {
    pub title: String,
    pub chat_type: ChatType,
    /// Human-readable status: "online", "bot", "42 members", "1000 subscribers", etc.
    pub status_line: String,
    /// User bio or group/channel description (may be absent).
    pub description: Option<String>,
}

/// State of the chat info popup overlay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatInfoPopupState {
    /// Data is being fetched from Telegram.
    Loading { chat_id: i64, title: String },
    /// Data has been resolved.
    Loaded(ChatInfo),
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
mod tests {
    use super::*;

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
        let state = ChatInfoPopupState::Loaded(ChatInfo {
            title: "Alice".into(),
            chat_type: ChatType::Private,
            status_line: "online".into(),
            description: None,
        });
        assert_eq!(state.chat_id(), None);
    }

    #[test]
    fn loaded_state_returns_title() {
        let state = ChatInfoPopupState::Loaded(ChatInfo {
            title: "Bob".into(),
            chat_type: ChatType::Private,
            status_line: "online".into(),
            description: None,
        });
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
    fn loaded_with_description() {
        let info = ChatInfo {
            title: "Dev Chat".into(),
            chat_type: ChatType::Group,
            status_line: "42 members".into(),
            description: Some("A developer community".into()),
        };
        assert_eq!(info.description.as_deref(), Some("A developer community"));
    }

    #[test]
    fn loaded_without_description() {
        let info = ChatInfo {
            title: "Alice".into(),
            chat_type: ChatType::Private,
            status_line: "last seen recently".into(),
            description: None,
        };
        assert!(info.description.is_none());
    }
}
