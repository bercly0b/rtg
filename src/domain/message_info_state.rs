use chrono::{Local, TimeZone};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactionDetail {
    pub emoji: String,
    pub sender_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewerDetail {
    pub name: String,
    pub view_date: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageInfo {
    pub reactions: Vec<ReactionDetail>,
    pub viewers: Vec<ViewerDetail>,
    pub read_date: Option<i32>,
    pub edit_date: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageInfoPopupState {
    Loading { chat_id: i64, message_id: i64 },
    Loaded(MessageInfo),
    Error,
}

impl MessageInfoPopupState {
    pub fn ids(&self) -> Option<(i64, i64)> {
        match self {
            Self::Loading {
                chat_id,
                message_id,
            } => Some((*chat_id, *message_id)),
            _ => None,
        }
    }
}

pub fn format_unix_timestamp(ts: i32) -> String {
    Local
        .timestamp_opt(i64::from(ts), 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "unknown".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loading_state_returns_ids() {
        let state = MessageInfoPopupState::Loading {
            chat_id: 1,
            message_id: 2,
        };
        assert_eq!(state.ids(), Some((1, 2)));
    }

    #[test]
    fn loaded_state_returns_no_ids() {
        let state = MessageInfoPopupState::Loaded(MessageInfo {
            reactions: vec![],
            viewers: vec![],
            read_date: None,
            edit_date: None,
        });
        assert_eq!(state.ids(), None);
    }

    #[test]
    fn error_state_returns_no_ids() {
        let state = MessageInfoPopupState::Error;
        assert_eq!(state.ids(), None);
    }

    #[test]
    fn format_unix_timestamp_produces_nonempty_string() {
        let result = format_unix_timestamp(1700000000);
        assert!(!result.is_empty());
        assert!(result.contains("2023"));
    }
}
