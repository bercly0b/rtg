use std::collections::HashMap;
use std::time::Instant;

use super::chat::ChatType;

struct TypingAction {
    sender_name: String,
    label: String,
    timestamp: Instant,
}

const TYPING_EXPIRATION_SECS: u64 = 6;

#[derive(Default)]
pub struct TypingState {
    actions: HashMap<i64, TypingAction>,
}

impl TypingState {
    pub fn add_action(&mut self, user_id: i64, sender_name: String, label: String) {
        self.actions.insert(
            user_id,
            TypingAction {
                sender_name,
                label,
                timestamp: Instant::now(),
            },
        );
    }

    pub fn remove_action(&mut self, user_id: i64) {
        self.actions.remove(&user_id);
    }

    pub fn expire_stale(&mut self) {
        let now = Instant::now();
        self.actions
            .retain(|_, a| now.duration_since(a.timestamp).as_secs() < TYPING_EXPIRATION_SECS);
    }

    pub fn clear(&mut self) {
        self.actions.clear();
    }

    pub fn format_label(&self, chat_type: ChatType) -> String {
        if self.actions.is_empty() {
            return String::new();
        }

        let mut entries: Vec<_> = self.actions.values().collect();
        entries.sort_by_key(|a| a.timestamp);

        match chat_type {
            ChatType::Private => {
                format!("{}...", entries[0].label)
            }
            ChatType::Group => {
                let names: Vec<&str> = entries.iter().map(|a| a.sender_name.as_str()).collect();
                match names.len() {
                    1 => format!("{} is typing...", names[0]),
                    2 => format!("{} and {} are typing...", names[0], names[1]),
                    n => format!("{} people are typing...", n),
                }
            }
            ChatType::Channel => String::new(),
        }
    }
}

impl std::fmt::Debug for TypingState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypingState")
            .field("count", &self.actions.len())
            .finish()
    }
}

impl Clone for TypingState {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl PartialEq for TypingState {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for TypingState {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_state_formats_empty() {
        let state = TypingState::default();
        assert_eq!(state.format_label(ChatType::Private), "");
        assert_eq!(state.format_label(ChatType::Group), "");
        assert_eq!(state.format_label(ChatType::Channel), "");
    }

    #[test]
    fn private_chat_shows_action_label() {
        let mut state = TypingState::default();
        state.add_action(1, "Alice".into(), "typing".into());
        assert_eq!(state.format_label(ChatType::Private), "typing...");
    }

    #[test]
    fn group_chat_single_user() {
        let mut state = TypingState::default();
        state.add_action(1, "Alice".into(), "typing".into());
        assert_eq!(state.format_label(ChatType::Group), "Alice is typing...");
    }

    #[test]
    fn group_chat_two_users() {
        let mut state = TypingState::default();
        state.add_action(1, "Alice".into(), "typing".into());
        state.add_action(2, "Bob".into(), "typing".into());
        let label = state.format_label(ChatType::Group);
        assert!(
            label == "Alice and Bob are typing..." || label == "Bob and Alice are typing...",
            "got: {label}"
        );
    }

    #[test]
    fn group_chat_three_users() {
        let mut state = TypingState::default();
        state.add_action(1, "Alice".into(), "typing".into());
        state.add_action(2, "Bob".into(), "typing".into());
        state.add_action(3, "Charlie".into(), "typing".into());
        assert_eq!(
            state.format_label(ChatType::Group),
            "3 people are typing..."
        );
    }

    #[test]
    fn channel_always_empty() {
        let mut state = TypingState::default();
        state.add_action(1, "Alice".into(), "typing".into());
        assert_eq!(state.format_label(ChatType::Channel), "");
    }

    #[test]
    fn remove_action_clears_user() {
        let mut state = TypingState::default();
        state.add_action(1, "Alice".into(), "typing".into());
        state.remove_action(1);
        assert_eq!(state.format_label(ChatType::Private), "");
    }

    #[test]
    fn clear_removes_all() {
        let mut state = TypingState::default();
        state.add_action(1, "Alice".into(), "typing".into());
        state.add_action(2, "Bob".into(), "typing".into());
        state.clear();
        assert_eq!(state.format_label(ChatType::Group), "");
    }
}
