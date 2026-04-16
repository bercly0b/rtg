#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableReaction {
    pub emoji: String,
    pub needs_premium: bool,
}

impl AvailableReaction {
    pub fn display_name(&self) -> &str {
        emoji_name(&self.emoji)
    }
}

fn emoji_name(emoji: &str) -> &str {
    match emoji {
        "👍" => "Thumbs Up",
        "👎" => "Thumbs Down",
        "❤" | "❤️" => "Heart",
        "🔥" => "Fire",
        "🥰" => "Love",
        "👏" => "Clap",
        "😁" => "Grin",
        "🤔" => "Thinking",
        "🤯" => "Mind Blown",
        "😱" => "Shocked",
        "🤬" => "Angry",
        "😢" => "Crying",
        "🎉" => "Party",
        "🤩" => "Starstruck",
        "🤮" => "Vomiting",
        "💩" => "Poop",
        "🙏" => "Pray",
        "👌" => "OK",
        "🕊" | "🕊️" => "Dove",
        "🤡" => "Clown",
        "🥱" => "Yawning",
        "🥴" => "Woozy",
        "😍" => "Heart Eyes",
        "🐳" => "Whale",
        "❤‍🔥" | "❤️‍🔥" => "Heart on Fire",
        "🌚" => "New Moon",
        "🌭" => "Hot Dog",
        "💯" => "100",
        "🤣" => "ROFL",
        "⚡" | "⚡️" => "Lightning",
        "🍌" => "Banana",
        "🏆" => "Trophy",
        "💔" => "Broken Heart",
        "🤨" => "Raised Brow",
        "😐" => "Neutral",
        "🍓" => "Strawberry",
        "🍾" => "Champagne",
        "💋" => "Kiss",
        "🖕" => "Middle Finger",
        "😈" => "Devil",
        "😴" => "Sleeping",
        "😭" => "Sobbing",
        "🤓" => "Nerd",
        "👻" => "Ghost",
        "👨‍💻" => "Technologist",
        "👀" => "Eyes",
        "🎃" => "Jack-O-Lantern",
        "🙈" => "See-No-Evil",
        "😇" => "Angel",
        "😨" => "Fearful",
        "🤝" => "Handshake",
        "✍" | "✍️" => "Writing",
        "🤗" => "Hugging",
        "🫡" => "Salute",
        "🎅" => "Santa",
        "🎄" => "Christmas Tree",
        "☃" | "☃️" => "Snowman",
        "💅" => "Nail Polish",
        "🤪" => "Zany",
        "🗿" => "Moai",
        "🆒" => "Cool",
        "💘" => "Cupid",
        "🙉" => "Hear-No-Evil",
        "🦄" => "Unicorn",
        "😘" => "Blowing Kiss",
        "💊" => "Pill",
        "🙊" => "Speak-No-Evil",
        "😎" => "Sunglasses",
        "👾" => "Alien Monster",
        "🤷" | "🤷‍♂️" | "🤷‍♀️" => "Shrug",
        "😡" => "Pouting",
        _ => "",
    }
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
        if !self.items.is_empty() && self.selected_index + 1 < self.items.len() {
            self.selected_index += 1;
        }
    }

    pub fn select_previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
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
    fn select_next_stops_at_last() {
        let mut data = ReactionPickerData::new(sample_reactions(), 1, 2);
        data.select_next();
        assert_eq!(data.selected_index, 1);
        data.select_next();
        assert_eq!(data.selected_index, 2);
        data.select_next();
        assert_eq!(data.selected_index, 2);
    }

    #[test]
    fn select_previous_stops_at_first() {
        let mut data = ReactionPickerData::new(sample_reactions(), 1, 2);
        data.select_previous();
        assert_eq!(data.selected_index, 0);
        data.select_next();
        data.select_next();
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

    #[test]
    fn data_mut_returns_none_for_loading() {
        let mut state = ReactionPickerState::Loading {
            chat_id: 1,
            message_id: 2,
        };
        assert!(state.data_mut().is_none());
    }

    #[test]
    fn data_mut_returns_some_for_ready() {
        let mut state =
            ReactionPickerState::Ready(ReactionPickerData::new(sample_reactions(), 1, 2));
        assert!(state.data_mut().is_some());
        state.data_mut().unwrap().select_next();
        match &state {
            ReactionPickerState::Ready(data) => assert_eq!(data.selected_index, 1),
            _ => panic!("expected Ready"),
        }
    }

    #[test]
    fn display_name_returns_known_names() {
        let r = AvailableReaction {
            emoji: "👍".into(),
            needs_premium: false,
        };
        assert_eq!(r.display_name(), "Thumbs Up");

        let r = AvailableReaction {
            emoji: "❤".into(),
            needs_premium: false,
        };
        assert_eq!(r.display_name(), "Heart");

        let r = AvailableReaction {
            emoji: "🔥".into(),
            needs_premium: false,
        };
        assert_eq!(r.display_name(), "Fire");
    }

    #[test]
    fn display_name_returns_empty_for_unknown() {
        let r = AvailableReaction {
            emoji: "🧪".into(),
            needs_premium: false,
        };
        assert_eq!(r.display_name(), "");
    }

    #[test]
    fn single_item_navigation_stays_in_place() {
        let items = vec![AvailableReaction {
            emoji: "👍".into(),
            needs_premium: false,
        }];
        let mut data = ReactionPickerData::new(items, 1, 2);
        assert_eq!(data.selected_index, 0);
        data.select_next();
        assert_eq!(data.selected_index, 0);
        data.select_previous();
        assert_eq!(data.selected_index, 0);
    }
}
