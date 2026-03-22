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
            key_label: "dd",
            action_name: "delete_message",
        },
        HotkeyEntry {
            key_label: "o",
            action_name: "open_link_in_browser",
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

    #[test]
    fn all_action_names_are_snake_case() {
        let all_entries = chat_list_hotkeys().iter().chain(messages_hotkeys().iter());
        for entry in all_entries {
            assert!(
                entry
                    .action_name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c == '_'),
                "action name '{}' is not snake_case",
                entry.action_name
            );
        }
    }

    #[test]
    fn all_key_labels_are_non_empty() {
        let all_entries = chat_list_hotkeys().iter().chain(messages_hotkeys().iter());
        for entry in all_entries {
            assert!(
                !entry.key_label.is_empty(),
                "key label should not be empty for action '{}'",
                entry.action_name
            );
        }
    }

    #[test]
    fn all_action_names_are_non_empty() {
        let all_entries = chat_list_hotkeys().iter().chain(messages_hotkeys().iter());
        for entry in all_entries {
            assert!(
                !entry.action_name.is_empty(),
                "action name should not be empty for key '{}'",
                entry.key_label
            );
        }
    }

    #[test]
    fn chat_list_contains_quit_hotkey() {
        assert!(
            chat_list_hotkeys().iter().any(|e| e.action_name == "quit"),
            "chat list hotkeys should include quit"
        );
    }

    #[test]
    fn messages_contains_quit_hotkey() {
        assert!(
            messages_hotkeys().iter().any(|e| e.action_name == "quit"),
            "messages hotkeys should include quit"
        );
    }

    #[test]
    fn chat_list_contains_show_help_hotkey() {
        assert!(
            chat_list_hotkeys()
                .iter()
                .any(|e| e.action_name == "show_help"),
            "chat list hotkeys should include show_help"
        );
    }

    #[test]
    fn messages_contains_show_help_hotkey() {
        assert!(
            messages_hotkeys()
                .iter()
                .any(|e| e.action_name == "show_help"),
            "messages hotkeys should include show_help"
        );
    }

    #[test]
    fn chat_list_and_messages_have_different_content() {
        let cl: Vec<&str> = chat_list_hotkeys().iter().map(|e| e.action_name).collect();
        let msg: Vec<&str> = messages_hotkeys().iter().map(|e| e.action_name).collect();
        assert_ne!(cl, msg, "chat list and messages hotkeys should differ");
    }
}
