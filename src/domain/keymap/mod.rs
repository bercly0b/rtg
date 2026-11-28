use std::time::{Duration, Instant};

const SEQUENCE_TIMEOUT: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumIter)]
pub enum Action {
    // ChatList
    SelectNextChat,
    SelectPreviousChat,
    OpenChat,
    RefreshChatList,
    MarkChatAsRead,
    ShowChatInfo,
    QuickSearch,
    GlobalSearch,
    SelectFirstChat,
    ShowChatFolders,
    ToggleChatArchive,
    // ForumTopicList
    SelectNextTopic,
    SelectPreviousTopic,
    SelectFirstTopic,
    OpenForumTopic,
    BackFromForum,
    ReloadForumTopics,
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
    AttachFile,
    ShowMessageInfo,
    AddReaction,
    DownloadFile,
    SaveFile,
    ScrollToLastMessage,
    // Global
    Quit,
    ShowHelp,
    ToggleLayout,
    InstallUpdate,
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
            Self::QuickSearch => "quick_search",
            Self::GlobalSearch => "global_search",
            Self::SelectFirstChat => "select_first_chat",
            Self::ShowChatFolders => "show_chat_folders",
            Self::ToggleChatArchive => "toggle_chat_archive",
            Self::SelectNextTopic => "select_next_topic",
            Self::SelectPreviousTopic => "select_previous_topic",
            Self::SelectFirstTopic => "select_first_topic",
            Self::OpenForumTopic => "open_forum_topic",
            Self::BackFromForum => "back_from_forum",
            Self::ReloadForumTopics => "reload_forum_topics",
            Self::ScrollNextMessage => "scroll_to_next_message",
            Self::ScrollPreviousMessage => "scroll_to_previous_message",
            Self::BackToChatList => "back_to_chat_list",
            Self::EnterMessageInput => "enter_message_input",
            Self::ReplyToMessage => "reply_to_message",
            Self::EditMessage => "edit_message",
            Self::CopyMessage => "copy_message_to_clipboard",
            Self::DeleteMessage => "delete_message",
            Self::OpenMessage => "open_message",
            Self::OpenLink => "open_link_or_mention",
            Self::RecordVoice => "record_voice_message",
            Self::AttachFile => "attach_file",
            Self::ShowMessageInfo => "show_message_info",
            Self::AddReaction => "add_reaction",
            Self::DownloadFile => "download_file",
            Self::SaveFile => "save_file_to_downloads",
            Self::ScrollToLastMessage => "scroll_to_last_message",
            Self::Quit => "quit",
            Self::ShowHelp => "show_help",
            Self::ToggleLayout => "toggle_layout",
            Self::InstallUpdate => "install_update",
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
            "quick_search" => Some(Self::QuickSearch),
            "global_search" => Some(Self::GlobalSearch),
            "select_first_chat" => Some(Self::SelectFirstChat),
            "show_chat_folders" => Some(Self::ShowChatFolders),
            "toggle_chat_archive" => Some(Self::ToggleChatArchive),
            "select_next_topic" => Some(Self::SelectNextTopic),
            "select_previous_topic" => Some(Self::SelectPreviousTopic),
            "select_first_topic" => Some(Self::SelectFirstTopic),
            "open_forum_topic" => Some(Self::OpenForumTopic),
            "back_from_forum" => Some(Self::BackFromForum),
            "reload_forum_topics" => Some(Self::ReloadForumTopics),
            "scroll_to_next_message" => Some(Self::ScrollNextMessage),
            "scroll_to_previous_message" => Some(Self::ScrollPreviousMessage),
            "back_to_chat_list" => Some(Self::BackToChatList),
            "enter_message_input" => Some(Self::EnterMessageInput),
            "reply_to_message" => Some(Self::ReplyToMessage),
            "edit_message" => Some(Self::EditMessage),
            "copy_message_to_clipboard" => Some(Self::CopyMessage),
            "delete_message" => Some(Self::DeleteMessage),
            "open_message" => Some(Self::OpenMessage),
            "open_link_or_mention" | "open_link_in_browser" => Some(Self::OpenLink),
            "record_voice_message" => Some(Self::RecordVoice),
            "attach_file" => Some(Self::AttachFile),
            "show_message_info" => Some(Self::ShowMessageInfo),
            "add_reaction" => Some(Self::AddReaction),
            "download_file" => Some(Self::DownloadFile),
            "save_file_to_downloads" => Some(Self::SaveFile),
            "scroll_to_last_message" => Some(Self::ScrollToLastMessage),
            "quit" => Some(Self::Quit),
            "show_help" => Some(Self::ShowHelp),
            "toggle_layout" => Some(Self::ToggleLayout),
            "install_update" => Some(Self::InstallUpdate),
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
    ForumTopicList,
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

    #[allow(dead_code, reason = "used only by tests")]
    pub fn reset_pending(&mut self) {
        self.pending_keys.clear();
        self.pending_since = None;
    }

    #[allow(dead_code, reason = "used only by tests")]
    pub fn has_pending(&self) -> bool {
        !self.pending_keys.is_empty()
    }

    pub fn help_groups(&self, context: KeyContext) -> Vec<HelpGroup> {
        HOTKEY_GROUPS
            .iter()
            .filter_map(|&(title, actions)| {
                let entries: Vec<HelpEntry> = actions
                    .iter()
                    .filter_map(|&action| self.help_entry(action, context))
                    .collect();
                (!entries.is_empty()).then_some(HelpGroup { title, entries })
            })
            .collect()
    }

    fn help_entry(&self, action: Action, context: KeyContext) -> Option<HelpEntry> {
        let labels: Vec<String> = self
            .bindings
            .iter()
            .filter(|b| {
                b.action == action && (b.context == context || b.context == KeyContext::Global)
            })
            .map(|b| b.pattern.display_label())
            .collect();

        (!labels.is_empty()).then(|| HelpEntry {
            key_label: labels.join(" / "),
            action_name: action.display_name().to_owned(),
        })
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpGroup {
    pub title: &'static str,
    pub entries: Vec<HelpEntry>,
}

/// Every action belongs to exactly one hotkey group. The help popup lists
/// groups and their actions in this order, whatever the bindings are.
const HOTKEY_GROUPS: &[(&str, &[Action])] = &[
    (
        "Navigation",
        &[
            Action::SelectNextChat,
            Action::SelectPreviousChat,
            Action::SelectFirstChat,
            Action::OpenChat,
            Action::ScrollNextMessage,
            Action::ScrollPreviousMessage,
            Action::ScrollToLastMessage,
            Action::BackToChatList,
            Action::SelectNextTopic,
            Action::SelectPreviousTopic,
            Action::SelectFirstTopic,
            Action::OpenForumTopic,
            Action::BackFromForum,
        ],
    ),
    (
        "Chat lists",
        &[
            Action::QuickSearch,
            Action::GlobalSearch,
            Action::ShowChatFolders,
            Action::RefreshChatList,
        ],
    ),
    ("Topics", &[Action::ReloadForumTopics]),
    (
        "Chat",
        &[
            Action::MarkChatAsRead,
            Action::ToggleChatArchive,
            Action::ShowChatInfo,
        ],
    ),
    (
        "Writing",
        &[
            Action::EnterMessageInput,
            Action::ReplyToMessage,
            Action::EditMessage,
            Action::AttachFile,
            Action::RecordVoice,
        ],
    ),
    (
        "Message",
        &[
            Action::OpenMessage,
            Action::OpenLink,
            Action::CopyMessage,
            Action::AddReaction,
            Action::ShowMessageInfo,
            Action::DeleteMessage,
        ],
    ),
    ("Files", &[Action::DownloadFile, Action::SaveFile]),
    (
        "General",
        &[
            Action::ShowHelp,
            Action::ToggleLayout,
            Action::InstallUpdate,
            Action::Quit,
        ],
    ),
];

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
            action: Action::QuickSearch,
            context: KeyContext::ChatList,
        },
        KeyBinding {
            pattern: KeyPattern::single("s"),
            action: Action::GlobalSearch,
            context: KeyContext::ChatList,
        },
        KeyBinding {
            pattern: KeyPattern::sequence(vec!["g", "g"]),
            action: Action::SelectFirstChat,
            context: KeyContext::ChatList,
        },
        KeyBinding {
            pattern: KeyPattern::single("f"),
            action: Action::ShowChatFolders,
            context: KeyContext::ChatList,
        },
        KeyBinding {
            pattern: KeyPattern::single("A"),
            action: Action::ToggleChatArchive,
            context: KeyContext::ChatList,
        },
        // ── ForumTopicList ──
        KeyBinding {
            pattern: KeyPattern::single("j"),
            action: Action::SelectNextTopic,
            context: KeyContext::ForumTopicList,
        },
        KeyBinding {
            pattern: KeyPattern::single("k"),
            action: Action::SelectPreviousTopic,
            context: KeyContext::ForumTopicList,
        },
        KeyBinding {
            pattern: KeyPattern::single("enter"),
            action: Action::OpenForumTopic,
            context: KeyContext::ForumTopicList,
        },
        KeyBinding {
            pattern: KeyPattern::single("l"),
            action: Action::OpenForumTopic,
            context: KeyContext::ForumTopicList,
        },
        KeyBinding {
            pattern: KeyPattern::single("h"),
            action: Action::BackFromForum,
            context: KeyContext::ForumTopicList,
        },
        KeyBinding {
            pattern: KeyPattern::sequence(vec!["g", "g"]),
            action: Action::SelectFirstTopic,
            context: KeyContext::ForumTopicList,
        },
        KeyBinding {
            pattern: KeyPattern::single("R"),
            action: Action::ReloadForumTopics,
            context: KeyContext::ForumTopicList,
        },
        // ── Messages ──
        KeyBinding {
            pattern: KeyPattern::single("G"),
            action: Action::ScrollToLastMessage,
            context: KeyContext::Messages,
        },
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
            pattern: KeyPattern::single("A"),
            action: Action::AttachFile,
            context: KeyContext::Messages,
        },
        KeyBinding {
            pattern: KeyPattern::single("I"),
            action: Action::ShowChatInfo,
            context: KeyContext::Messages,
        },
        KeyBinding {
            pattern: KeyPattern::single("m"),
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
        KeyBinding {
            pattern: KeyPattern::single("R"),
            action: Action::AddReaction,
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
        KeyBinding {
            pattern: KeyPattern::single("L"),
            action: Action::ToggleLayout,
            context: KeyContext::Global,
        },
        KeyBinding {
            pattern: KeyPattern::single("U"),
            action: Action::InstallUpdate,
            context: KeyContext::Global,
        },
    ]
}

#[cfg(test)]
mod tests;
