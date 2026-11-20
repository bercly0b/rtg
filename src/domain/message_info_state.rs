use super::local_time::LocalTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactionDetail {
    pub emoji: String,
    pub sender_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactionCount {
    pub emoji: String,
    pub count: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewerDetail {
    pub name: String,
    pub view_date: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageAuthor {
    pub name: String,
    pub username: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageInfo {
    pub author: Option<MessageAuthor>,
    pub reactions: Vec<ReactionDetail>,
    pub reaction_counts: Vec<ReactionCount>,
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

pub fn format_unix_timestamp(ts: impl Into<i64>) -> String {
    LocalTime::from_unix_seconds(ts.into())
        .map(|time| time.format("%Y-%m-%d %H:%M"))
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
            author: None,
            reactions: vec![],
            reaction_counts: vec![],
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
    fn format_unix_timestamp_writes_local_date_and_time() {
        let ts = LocalTime::from_local(2023, 11, 4, 9, 7).unix_seconds();

        assert_eq!(format_unix_timestamp(ts), "2023-11-04 09:07");
    }

    #[test]
    fn format_unix_timestamp_out_of_range_is_unknown() {
        assert_eq!(format_unix_timestamp(i64::MIN), "unknown");
    }
}
