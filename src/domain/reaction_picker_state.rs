#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableReaction {
    pub emoji: String,
    pub needs_premium: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactionPickerData {
    pub items: Vec<AvailableReaction>,
    pub selected_index: usize,
    pub chat_id: i64,
    pub message_id: i64,
}

impl ReactionPickerData {
    pub fn new(items: Vec<AvailableReaction>, chat_id: i64, message_id: i64) -> Self {
        Self {
            items,
            selected_index: 0,
            chat_id,
            message_id,
        }
    }

    pub fn select_next(&mut self) {
        if !self.items.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.items.len();
        }
    }

    pub fn select_previous(&mut self) {
        if !self.items.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.items.len() - 1
            } else {
                self.selected_index - 1
            };
        }
    }

    pub fn selected_emoji(&self) -> Option<&str> {
        self.items
            .get(self.selected_index)
            .map(|r| r.emoji.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReactionPickerState {
    Loading { chat_id: i64, message_id: i64 },
    Ready(ReactionPickerData),
    Error,
}

impl ReactionPickerState {
    pub fn ids(&self) -> Option<(i64, i64)> {
        match self {
            Self::Loading {
                chat_id,
                message_id,
            } => Some((*chat_id, *message_id)),
            Self::Ready(data) => Some((data.chat_id, data.message_id)),
            Self::Error => None,
        }
    }

    pub fn data_mut(&mut self) -> Option<&mut ReactionPickerData> {
        match self {
            Self::Ready(data) => Some(data),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_reactions() -> Vec<AvailableReaction> {
        vec![
            AvailableReaction {
                emoji: "👍".into(),
                needs_premium: false,
            },
            AvailableReaction {
                emoji: "👎".into(),
                needs_premium: false,
            },
            AvailableReaction {
                emoji: "❤".into(),
                needs_premium: false,
            },
        ]
    }

    #[test]
    fn new_picker_starts_at_index_zero() {
        let data = ReactionPickerData::new(sample_reactions(), 1, 2);
        assert_eq!(data.selected_index, 0);
        assert_eq!(data.selected_emoji(), Some("👍"));
    }

    #[test]
    fn select_next_wraps_around() {
        let mut data = ReactionPickerData::new(sample_reactions(), 1, 2);
        data.select_next();
        assert_eq!(data.selected_index, 1);
        data.select_next();
        assert_eq!(data.selected_index, 2);
        data.select_next();
        assert_eq!(data.selected_index, 0);
    }

    #[test]
    fn select_previous_wraps_around() {
        let mut data = ReactionPickerData::new(sample_reactions(), 1, 2);
        data.select_previous();
        assert_eq!(data.selected_index, 2);
        data.select_previous();
        assert_eq!(data.selected_index, 1);
    }

    #[test]
    fn empty_items_does_not_panic() {
        let mut data = ReactionPickerData::new(vec![], 1, 2);
        data.select_next();
        data.select_previous();
        assert_eq!(data.selected_emoji(), None);
    }

    #[test]
    fn loading_state_returns_ids() {
        let state = ReactionPickerState::Loading {
            chat_id: 1,
            message_id: 2,
        };
        assert_eq!(state.ids(), Some((1, 2)));
    }

    #[test]
    fn ready_state_returns_ids() {
        let state = ReactionPickerState::Ready(ReactionPickerData::new(vec![], 1, 2));
        assert_eq!(state.ids(), Some((1, 2)));
    }

    #[test]
    fn error_state_returns_no_ids() {
        let state = ReactionPickerState::Error;
        assert_eq!(state.ids(), None);
    }
}
