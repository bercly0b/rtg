use std::time::{Duration, Instant};

const SEQUENCE_TIMEOUT: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    // ChatList
    SelectNextChat,
    SelectPreviousChat,
    OpenChat,
    RefreshChatList,
    MarkChatAsRead,
    ShowChatInfo,
    SearchChats,
    // Messages
    ScrollNextMessage,
    ScrollPreviousMessage,
    BackToChatList,
    EnterMessageInput,
    ReplyToMessage,
    EditMessage,
    CopyMessage,
    DeleteMessage,
    OpenMessage,
    OpenLink,
    RecordVoice,
    ShowMessageInfo,
    DownloadFile,
    SaveFile,
    // Global
    Quit,
    ShowHelp,
}

impl Action {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::SelectNextChat => "select_next_chat",
            Self::SelectPreviousChat => "select_previous_chat",
            Self::OpenChat => "open_chat",
            Self::RefreshChatList => "refresh_chat_list",
            Self::MarkChatAsRead => "mark_chat_as_read",
            Self::ShowChatInfo => "show_chat_info",
            Self::SearchChats => "search_chats",
            Self::ScrollNextMessage => "scroll_to_next_message",
            Self::ScrollPreviousMessage => "scroll_to_previous_message",
            Self::BackToChatList => "back_to_chat_list",
            Self::EnterMessageInput => "enter_message_input",
            Self::ReplyToMessage => "reply_to_message",
            Self::EditMessage => "edit_message",
            Self::CopyMessage => "copy_message_to_clipboard",
            Self::DeleteMessage => "delete_message",
            Self::OpenMessage => "open_message",
            Self::OpenLink => "open_link_in_browser",
            Self::RecordVoice => "record_voice_message",
            Self::ShowMessageInfo => "show_message_info",
            Self::DownloadFile => "download_file",
            Self::SaveFile => "save_file_to_downloads",
            Self::Quit => "quit",
            Self::ShowHelp => "show_help",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "select_next_chat" => Some(Self::SelectNextChat),
            "select_previous_chat" => Some(Self::SelectPreviousChat),
            "open_chat" => Some(Self::OpenChat),
            "refresh_chat_list" => Some(Self::RefreshChatList),
            "mark_chat_as_read" => Some(Self::MarkChatAsRead),
            "show_chat_info" => Some(Self::ShowChatInfo),
            "search_chats" => Some(Self::SearchChats),
            "scroll_to_next_message" => Some(Self::ScrollNextMessage),
            "scroll_to_previous_message" => Some(Self::ScrollPreviousMessage),
            "back_to_chat_list" => Some(Self::BackToChatList),
            "enter_message_input" => Some(Self::EnterMessageInput),
            "reply_to_message" => Some(Self::ReplyToMessage),
            "edit_message" => Some(Self::EditMessage),
            "copy_message_to_clipboard" => Some(Self::CopyMessage),
            "delete_message" => Some(Self::DeleteMessage),
            "open_message" => Some(Self::OpenMessage),
            "open_link_in_browser" => Some(Self::OpenLink),
            "record_voice_message" => Some(Self::RecordVoice),
            "show_message_info" => Some(Self::ShowMessageInfo),
            "download_file" => Some(Self::DownloadFile),
            "save_file_to_downloads" => Some(Self::SaveFile),
            "quit" => Some(Self::Quit),
            "show_help" => Some(Self::ShowHelp),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyPattern {
    Single { key: String, ctrl: bool },
    Sequence(Vec<String>),
}

impl KeyPattern {
    pub fn single(key: impl Into<String>) -> Self {
        Self::Single {
            key: key.into(),
            ctrl: false,
        }
    }

    pub fn single_ctrl(key: impl Into<String>) -> Self {
        Self::Single {
            key: key.into(),
            ctrl: true,
        }
    }

    pub fn sequence(keys: Vec<impl Into<String>>) -> Self {
        Self::Sequence(keys.into_iter().map(Into::into).collect())
    }

    pub fn display_label(&self) -> String {
        match self {
            Self::Single { key, ctrl: true } => format!("Ctrl+{}", key.to_uppercase()),
            Self::Single { key, ctrl: false } => match key.as_str() {
                "enter" => "Enter".to_owned(),
                "esc" => "Esc".to_owned(),
                "backspace" => "Backspace".to_owned(),
                "delete" => "Delete".to_owned(),
                "left" => "Left".to_owned(),
                "right" => "Right".to_owned(),
                "home" => "Home".to_owned(),
                "end" => "End".to_owned(),
                _ => key.clone(),
            },
            Self::Sequence(keys) => keys.join(""),
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.is_empty() {
            return None;
        }
        if let Some(rest) = s.strip_prefix("Ctrl+") {
            return Some(Self::single_ctrl(rest.to_lowercase()));
        }
        if let Some(rest) = s.strip_prefix("ctrl+") {
            return Some(Self::single_ctrl(rest.to_lowercase()));
        }
        let lower = s.to_lowercase();
        match lower.as_str() {
            "enter" | "esc" | "backspace" | "delete" | "left" | "right" | "home" | "end" => {
                return Some(Self::single(lower));
            }
            _ => {}
        }
        let chars: Vec<char> = s.chars().collect();
        if chars.len() > 1 && chars.iter().all(|c| c.is_alphanumeric()) {
            return Some(Self::Sequence(
                chars.iter().map(|c| c.to_string()).collect(),
            ));
        }
        if chars.len() == 1 {
            return Some(Self::single(s));
        }
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyContext {
    ChatList,
    Messages,
    Global,
}

#[derive(Debug, Clone)]
pub struct KeyBinding {
    pub pattern: KeyPattern,
    pub action: Action,
    pub context: KeyContext,
}

#[derive(Debug, Clone)]
pub struct Keymap {
    bindings: Vec<KeyBinding>,
    pending_keys: Vec<String>,
    pending_since: Option<Instant>,
}

impl Default for Keymap {
    fn default() -> Self {
        Self {
            bindings: default_bindings(),
            pending_keys: Vec::new(),
            pending_since: None,
        }
    }
}

impl Keymap {
    pub fn with_overrides(overrides: &std::collections::HashMap<String, String>) -> Self {
        let mut keymap = Self::default();
        for (action_name, key_str) in overrides {
            let Some(action) = Action::from_name(action_name) else {
                tracing::warn!(
                    action_name,
                    key_str,
                    "unknown action in key config, skipping"
                );
                continue;
            };
            let Some(pattern) = KeyPattern::parse(key_str) else {
                tracing::warn!(
                    action_name,
                    key_str,
                    "invalid key pattern in config, skipping"
                );
                continue;
            };
            keymap.rebind(action, pattern);
        }
        keymap
    }

    pub fn resolve(&mut self, key: &str, ctrl: bool, context: KeyContext) -> ResolveResult {
        if self.sequence_timed_out() {
            self.pending_keys.clear();
            self.pending_since = None;
        }

        let mut candidate_keys = self.pending_keys.clone();
        candidate_keys.push(key.to_owned());

        if let Some(action) = self.find_exact_match(&candidate_keys, ctrl, context) {
            self.pending_keys.clear();
            self.pending_since = None;
            return ResolveResult::Action(action);
        }

        if !ctrl && self.has_sequence_prefix(&candidate_keys, context) {
            self.pending_keys = candidate_keys;
            if self.pending_since.is_none() {
                self.pending_since = Some(Instant::now());
            }
            return ResolveResult::Pending;
        }

        self.pending_keys.clear();
        self.pending_since = None;

        if let Some(action) = self.find_single_match(key, ctrl, context) {
            return ResolveResult::Action(action);
        }

        ResolveResult::Unmatched
    }

    #[allow(dead_code)]
    pub fn reset_pending(&mut self) {
        self.pending_keys.clear();
        self.pending_since = None;
    }

    #[allow(dead_code)]
    pub fn has_pending(&self) -> bool {
        !self.pending_keys.is_empty()
    }

    pub fn help_entries(&self, context: KeyContext) -> Vec<HelpEntry> {
        let mut entries = Vec::new();
        let mut seen_actions = std::collections::HashSet::new();

        for binding in &self.bindings {
            if binding.context != context && binding.context != KeyContext::Global {
                continue;
            }
            if !seen_actions.insert(binding.action) {
                continue;
            }

            let all_patterns: Vec<&KeyBinding> = self
                .bindings
                .iter()
                .filter(|b| {
                    b.action == binding.action
                        && (b.context == context || b.context == KeyContext::Global)
                })
                .collect();

            let label = if all_patterns.len() > 1 {
                all_patterns
                    .iter()
                    .map(|b| b.pattern.display_label())
                    .collect::<Vec<_>>()
                    .join(" / ")
            } else {
                binding.pattern.display_label()
            };

            entries.push(HelpEntry {
                key_label: label,
                action_name: binding.action.display_name().to_owned(),
            });
        }
        entries
    }

    fn rebind(&mut self, action: Action, new_pattern: KeyPattern) {
        let contexts: Vec<KeyContext> = self
            .bindings
            .iter()
            .filter(|b| b.action == action)
            .map(|b| b.context)
            .collect();

        self.bindings.retain(|b| b.action != action);

        for ctx in contexts {
            self.bindings.push(KeyBinding {
                pattern: new_pattern.clone(),
                action,
                context: ctx,
            });
        }
    }

    fn find_exact_match(&self, keys: &[String], ctrl: bool, context: KeyContext) -> Option<Action> {
        for binding in &self.bindings {
            if binding.context != context && binding.context != KeyContext::Global {
                continue;
            }
            match &binding.pattern {
                KeyPattern::Single { key, ctrl: bc } => {
                    if keys.len() == 1 && keys[0] == *key && ctrl == *bc {
                        return Some(binding.action);
                    }
                }
                KeyPattern::Sequence(seq) => {
                    if !ctrl && keys == seq.as_slice() {
                        return Some(binding.action);
                    }
                }
            }
        }
        None
    }

    fn find_single_match(&self, key: &str, ctrl: bool, context: KeyContext) -> Option<Action> {
        for binding in &self.bindings {
            if binding.context != context && binding.context != KeyContext::Global {
                continue;
            }
            if let KeyPattern::Single { key: bk, ctrl: bc } = &binding.pattern {
                if key == bk && ctrl == *bc {
                    return Some(binding.action);
                }
            }
        }
        None
    }

    fn has_sequence_prefix(&self, keys: &[String], context: KeyContext) -> bool {
        for binding in &self.bindings {
            if binding.context != context && binding.context != KeyContext::Global {
                continue;
            }
            if let KeyPattern::Sequence(seq) = &binding.pattern {
                if seq.len() > keys.len() && seq.starts_with(keys) {
                    return true;
                }
            }
        }
        false
    }

    fn sequence_timed_out(&self) -> bool {
        self.pending_since
            .map(|t| t.elapsed() >= SEQUENCE_TIMEOUT)
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveResult {
    Action(Action),
    Pending,
    Unmatched,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpEntry {
    pub key_label: String,
    pub action_name: String,
}

fn default_bindings() -> Vec<KeyBinding> {
    vec![
        // ── ChatList ──
        KeyBinding {
            pattern: KeyPattern::single("j"),
            action: Action::SelectNextChat,
            context: KeyContext::ChatList,
        },
        KeyBinding {
            pattern: KeyPattern::single("k"),
            action: Action::SelectPreviousChat,
            context: KeyContext::ChatList,
        },
        KeyBinding {
            pattern: KeyPattern::single("enter"),
            action: Action::OpenChat,
            context: KeyContext::ChatList,
        },
        KeyBinding {
            pattern: KeyPattern::single("l"),
            action: Action::OpenChat,
            context: KeyContext::ChatList,
        },
        KeyBinding {
            pattern: KeyPattern::single("R"),
            action: Action::RefreshChatList,
            context: KeyContext::ChatList,
        },
        KeyBinding {
            pattern: KeyPattern::single("r"),
            action: Action::MarkChatAsRead,
            context: KeyContext::ChatList,
        },
        KeyBinding {
            pattern: KeyPattern::single("I"),
            action: Action::ShowChatInfo,
            context: KeyContext::ChatList,
        },
        KeyBinding {
            pattern: KeyPattern::single("/"),
            action: Action::SearchChats,
            context: KeyContext::ChatList,
        },
        // ── Messages ──
        KeyBinding {
            pattern: KeyPattern::single("j"),
            action: Action::ScrollNextMessage,
            context: KeyContext::Messages,
        },
        KeyBinding {
            pattern: KeyPattern::single("k"),
            action: Action::ScrollPreviousMessage,
            context: KeyContext::Messages,
        },
        KeyBinding {
            pattern: KeyPattern::single("h"),
            action: Action::BackToChatList,
            context: KeyContext::Messages,
        },
        KeyBinding {
            pattern: KeyPattern::single("esc"),
            action: Action::BackToChatList,
            context: KeyContext::Messages,
        },
        KeyBinding {
            pattern: KeyPattern::single("i"),
            action: Action::EnterMessageInput,
            context: KeyContext::Messages,
        },
        KeyBinding {
            pattern: KeyPattern::single("r"),
            action: Action::ReplyToMessage,
            context: KeyContext::Messages,
        },
        KeyBinding {
            pattern: KeyPattern::single("e"),
            action: Action::EditMessage,
            context: KeyContext::Messages,
        },
        KeyBinding {
            pattern: KeyPattern::single("y"),
            action: Action::CopyMessage,
            context: KeyContext::Messages,
        },
        KeyBinding {
            pattern: KeyPattern::sequence(vec!["d", "d"]),
            action: Action::DeleteMessage,
            context: KeyContext::Messages,
        },
        KeyBinding {
            pattern: KeyPattern::single("l"),
            action: Action::OpenMessage,
            context: KeyContext::Messages,
        },
        KeyBinding {
            pattern: KeyPattern::single("o"),
            action: Action::OpenLink,
            context: KeyContext::Messages,
        },
        KeyBinding {
            pattern: KeyPattern::single("v"),
            action: Action::RecordVoice,
            context: KeyContext::Messages,
        },
        KeyBinding {
            pattern: KeyPattern::single("I"),
            action: Action::ShowMessageInfo,
            context: KeyContext::Messages,
        },
        KeyBinding {
            pattern: KeyPattern::single("D"),
            action: Action::DownloadFile,
            context: KeyContext::Messages,
        },
        KeyBinding {
            pattern: KeyPattern::single("S"),
            action: Action::SaveFile,
            context: KeyContext::Messages,
        },
        // ── Global ──
        KeyBinding {
            pattern: KeyPattern::single("q"),
            action: Action::Quit,
            context: KeyContext::Global,
        },
        KeyBinding {
            pattern: KeyPattern::single("?"),
            action: Action::ShowHelp,
            context: KeyContext::Global,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_keymap_resolves_single_keys() {
        let mut km = Keymap::default();
        assert_eq!(
            km.resolve("j", false, KeyContext::ChatList),
            ResolveResult::Action(Action::SelectNextChat)
        );
        assert_eq!(
            km.resolve("j", false, KeyContext::Messages),
            ResolveResult::Action(Action::ScrollNextMessage)
        );
    }

    #[test]
    fn dd_sequence_resolved() {
        let mut km = Keymap::default();
        assert_eq!(
            km.resolve("d", false, KeyContext::Messages),
            ResolveResult::Pending
        );
        assert_eq!(
            km.resolve("d", false, KeyContext::Messages),
            ResolveResult::Action(Action::DeleteMessage)
        );
    }

    #[test]
    fn dd_sequence_cancelled_by_other_key() {
        let mut km = Keymap::default();
        assert_eq!(
            km.resolve("d", false, KeyContext::Messages),
            ResolveResult::Pending
        );
        assert_eq!(
            km.resolve("j", false, KeyContext::Messages),
            ResolveResult::Action(Action::ScrollNextMessage)
        );
        assert!(!km.has_pending());
    }

    #[test]
    fn dd_sequence_timeout() {
        let mut km = Keymap::default();
        assert_eq!(
            km.resolve("d", false, KeyContext::Messages),
            ResolveResult::Pending
        );
        km.pending_since = Some(Instant::now() - Duration::from_secs(2));
        assert_eq!(
            km.resolve("d", false, KeyContext::Messages),
            ResolveResult::Pending
        );
    }

    #[test]
    fn global_bindings_available_in_all_contexts() {
        let mut km = Keymap::default();
        assert_eq!(
            km.resolve("q", false, KeyContext::ChatList),
            ResolveResult::Action(Action::Quit)
        );
        assert_eq!(
            km.resolve("q", false, KeyContext::Messages),
            ResolveResult::Action(Action::Quit)
        );
    }

    #[test]
    fn context_specific_binding_overrides_global() {
        let mut km = Keymap::default();
        // "r" in ChatList context maps to MarkChatAsRead, not a global action
        assert_eq!(
            km.resolve("r", false, KeyContext::ChatList),
            ResolveResult::Action(Action::MarkChatAsRead)
        );
    }

    #[test]
    fn unmatched_key_returns_unmatched() {
        let mut km = Keymap::default();
        assert_eq!(
            km.resolve("z", false, KeyContext::ChatList),
            ResolveResult::Unmatched
        );
    }

    #[test]
    fn reset_pending_clears_state() {
        let mut km = Keymap::default();
        km.resolve("d", false, KeyContext::Messages);
        assert!(km.has_pending());
        km.reset_pending();
        assert!(!km.has_pending());
    }

    #[test]
    fn help_entries_for_chat_list() {
        let km = Keymap::default();
        let entries = km.help_entries(KeyContext::ChatList);
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.action_name == "select_next_chat"));
        assert!(entries.iter().any(|e| e.action_name == "quit"));
        assert!(entries.iter().any(|e| e.action_name == "show_help"));
    }

    #[test]
    fn help_entries_for_messages() {
        let km = Keymap::default();
        let entries = km.help_entries(KeyContext::Messages);
        assert!(entries.iter().any(|e| e.action_name == "delete_message"));
        assert!(entries.iter().any(|e| e.key_label == "dd"));
    }

    #[test]
    fn help_entries_merge_duplicate_actions() {
        let km = Keymap::default();
        let entries = km.help_entries(KeyContext::ChatList);
        let open_chat: Vec<_> = entries
            .iter()
            .filter(|e| e.action_name == "open_chat")
            .collect();
        assert_eq!(open_chat.len(), 1);
        assert!(open_chat[0].key_label.contains('/'));
    }

    #[test]
    fn key_pattern_parse_single() {
        assert_eq!(KeyPattern::parse("j"), Some(KeyPattern::single("j")));
        assert_eq!(KeyPattern::parse("?"), Some(KeyPattern::single("?")));
    }

    #[test]
    fn key_pattern_parse_ctrl() {
        assert_eq!(
            KeyPattern::parse("Ctrl+C"),
            Some(KeyPattern::single_ctrl("c"))
        );
    }

    #[test]
    fn key_pattern_parse_sequence() {
        assert_eq!(
            KeyPattern::parse("dd"),
            Some(KeyPattern::sequence(vec!["d", "d"]))
        );
        assert_eq!(
            KeyPattern::parse("gg"),
            Some(KeyPattern::sequence(vec!["g", "g"]))
        );
    }

    #[test]
    fn key_pattern_parse_special() {
        assert_eq!(
            KeyPattern::parse("Enter"),
            Some(KeyPattern::single("enter"))
        );
        assert_eq!(KeyPattern::parse("Esc"), Some(KeyPattern::single("esc")));
    }

    #[test]
    fn key_pattern_parse_empty() {
        assert_eq!(KeyPattern::parse(""), None);
    }

    #[test]
    fn with_overrides_rebinds_action() {
        let mut overrides = std::collections::HashMap::new();
        overrides.insert("select_next_chat".to_owned(), "n".to_owned());
        let mut km = Keymap::with_overrides(&overrides);
        assert_eq!(
            km.resolve("n", false, KeyContext::ChatList),
            ResolveResult::Action(Action::SelectNextChat)
        );
        assert_eq!(
            km.resolve("j", false, KeyContext::ChatList),
            ResolveResult::Unmatched
        );
    }

    #[test]
    fn with_overrides_unknown_action_ignored() {
        let mut overrides = std::collections::HashMap::new();
        overrides.insert("nonexistent_action".to_owned(), "x".to_owned());
        let km = Keymap::with_overrides(&overrides);
        assert_eq!(km.bindings.len(), default_bindings().len());
    }

    #[test]
    fn with_overrides_sequence() {
        let mut overrides = std::collections::HashMap::new();
        overrides.insert("delete_message".to_owned(), "xx".to_owned());
        let mut km = Keymap::with_overrides(&overrides);
        assert_eq!(
            km.resolve("x", false, KeyContext::Messages),
            ResolveResult::Pending
        );
        assert_eq!(
            km.resolve("x", false, KeyContext::Messages),
            ResolveResult::Action(Action::DeleteMessage)
        );
    }

    #[test]
    fn display_label_for_patterns() {
        assert_eq!(KeyPattern::single("j").display_label(), "j");
        assert_eq!(KeyPattern::single("enter").display_label(), "Enter");
        assert_eq!(KeyPattern::single_ctrl("c").display_label(), "Ctrl+C");
        assert_eq!(KeyPattern::sequence(vec!["d", "d"]).display_label(), "dd");
    }

    #[test]
    fn action_round_trip_name() {
        let actions = [
            Action::SelectNextChat,
            Action::DeleteMessage,
            Action::Quit,
            Action::ShowHelp,
        ];
        for action in actions {
            let name = action.display_name();
            assert_eq!(Action::from_name(name), Some(action));
        }
    }

    #[test]
    fn d_in_chat_list_is_unmatched() {
        let mut km = Keymap::default();
        assert_eq!(
            km.resolve("d", false, KeyContext::ChatList),
            ResolveResult::Unmatched
        );
    }

    #[test]
    fn enter_and_l_both_open_chat() {
        let mut km = Keymap::default();
        assert_eq!(
            km.resolve("enter", false, KeyContext::ChatList),
            ResolveResult::Action(Action::OpenChat)
        );
        assert_eq!(
            km.resolve("l", false, KeyContext::ChatList),
            ResolveResult::Action(Action::OpenChat)
        );
    }
}
