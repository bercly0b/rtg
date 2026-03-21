//! Static hotkey definitions for the help popup, grouped by active pane context.

/// A single hotkey entry shown in the help popup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HotkeyEntry {
    /// Human-readable key label, e.g. `"j"`, `"Enter / l"`, `"Ctrl+C"`.
    pub key_label: &'static str,
    /// Snake-case action name, e.g. `"select_next_chat"`.
    pub action_name: &'static str,
}

/// Hotkeys available when `ActivePane::ChatList` is focused.
pub fn chat_list_hotkeys() -> &'static [HotkeyEntry] {
    &[
        HotkeyEntry {
            key_label: "j",
            action_name: "select_next_chat",
        },
        HotkeyEntry {
            key_label: "k",
            action_name: "select_previous_chat",
        },
        HotkeyEntry {
            key_label: "Enter / l",
            action_name: "open_chat",
        },
        HotkeyEntry {
            key_label: "R",
            action_name: "refresh_chat_list",
        },
        HotkeyEntry {
            key_label: "r",
            action_name: "mark_chat_as_read",
        },
        HotkeyEntry {
            key_label: "Ctrl+O",
            action_name: "open_in_browser",
        },
        HotkeyEntry {
            key_label: "q",
            action_name: "quit",
        },
        HotkeyEntry {
            key_label: "Ctrl+C",
            action_name: "quit",
        },
        HotkeyEntry {
            key_label: "?",
            action_name: "show_help",
        },
    ]
}

/// Hotkeys available when `ActivePane::Messages` is focused.
pub fn messages_hotkeys() -> &'static [HotkeyEntry] {
    &[
        HotkeyEntry {
            key_label: "j",
            action_name: "scroll_to_next_message",
        },
        HotkeyEntry {
            key_label: "k",
            action_name: "scroll_to_previous_message",
        },
        HotkeyEntry {
            key_label: "h / Esc",
            action_name: "back_to_chat_list",
        },
        HotkeyEntry {
            key_label: "i",
            action_name: "enter_message_input",
        },
        HotkeyEntry {
            key_label: "y",
            action_name: "copy_message_to_clipboard",
        },
        HotkeyEntry {
            key_label: "Ctrl+O",
            action_name: "open_in_browser",
        },
        HotkeyEntry {
            key_label: "q",
            action_name: "quit",
        },
        HotkeyEntry {
            key_label: "Ctrl+C",
            action_name: "quit",
        },
        HotkeyEntry {
            key_label: "?",
            action_name: "show_help",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_list_hotkeys_not_empty() {
        assert!(!chat_list_hotkeys().is_empty());
    }

    #[test]
    fn messages_hotkeys_not_empty() {
        assert!(!messages_hotkeys().is_empty());
    }

    #[test]
    fn no_duplicate_key_labels_in_chat_list() {
        let entries = chat_list_hotkeys();
        for (i, a) in entries.iter().enumerate() {
            for b in entries.iter().skip(i + 1) {
                assert_ne!(
                    a.key_label, b.key_label,
                    "duplicate key label in chat list hotkeys"
                );
            }
        }
    }

    #[test]
    fn no_duplicate_key_labels_in_messages() {
        let entries = messages_hotkeys();
        for (i, a) in entries.iter().enumerate() {
            for b in entries.iter().skip(i + 1) {
                assert_ne!(
                    a.key_label, b.key_label,
                    "duplicate key label in messages hotkeys"
                );
            }
        }
    }
}
