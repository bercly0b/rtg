use super::*;

fn group_actions(groups: &[HelpGroup]) -> Vec<(&str, Vec<&str>)> {
    groups
        .iter()
        .map(|g| {
            let actions = g.entries.iter().map(|e| e.action_name.as_str()).collect();
            (g.title, actions)
        })
        .collect()
}

fn key_label<'a>(groups: &'a [HelpGroup], action_name: &str) -> &'a str {
    groups
        .iter()
        .flat_map(|g| &g.entries)
        .find(|e| e.action_name == action_name)
        .map(|e| e.key_label.as_str())
        .expect("action must be listed")
}

const GENERAL_GROUP: (&str, [&str; 4]) = (
    "General",
    ["show_help", "toggle_layout", "install_update", "quit"],
);

#[test]
fn help_groups_for_chat_list() {
    let groups = Keymap::default().help_groups(KeyContext::ChatList);

    assert_eq!(
        group_actions(&groups),
        vec![
            (
                "Navigation",
                vec![
                    "select_next_chat",
                    "select_previous_chat",
                    "select_first_chat",
                    "open_chat",
                ]
            ),
            (
                "Chat lists",
                vec![
                    "quick_search",
                    "global_search",
                    "show_chat_folders",
                    "refresh_chat_list",
                ]
            ),
            (
                "Chat",
                vec!["mark_chat_as_read", "toggle_chat_archive", "show_chat_info"]
            ),
            (GENERAL_GROUP.0, GENERAL_GROUP.1.to_vec()),
        ]
    );
}

#[test]
fn help_groups_for_forum_topic_list() {
    let groups = Keymap::default().help_groups(KeyContext::ForumTopicList);

    assert_eq!(
        group_actions(&groups),
        vec![
            (
                "Navigation",
                vec![
                    "select_next_topic",
                    "select_previous_topic",
                    "select_first_topic",
                    "open_forum_topic",
                    "back_from_forum",
                ]
            ),
            ("Topics", vec!["reload_forum_topics"]),
            (GENERAL_GROUP.0, GENERAL_GROUP.1.to_vec()),
        ]
    );
}

#[test]
fn help_groups_for_messages() {
    let groups = Keymap::default().help_groups(KeyContext::Messages);

    assert_eq!(
        group_actions(&groups),
        vec![
            (
                "Navigation",
                vec![
                    "scroll_to_next_message",
                    "scroll_to_previous_message",
                    "scroll_to_last_message",
                    "back_to_chat_list",
                ]
            ),
            ("Chat", vec!["show_chat_info"]),
            (
                "Writing",
                vec![
                    "enter_message_input",
                    "reply_to_message",
                    "edit_message",
                    "attach_file",
                    "record_voice_message",
                ]
            ),
            (
                "Message",
                vec![
                    "open_message",
                    "open_link_or_mention",
                    "copy_message_to_clipboard",
                    "add_reaction",
                    "show_message_info",
                    "delete_message",
                ]
            ),
            ("Files", vec!["download_file", "save_file_to_downloads"]),
            (GENERAL_GROUP.0, GENERAL_GROUP.1.to_vec()),
        ]
    );
}

#[test]
fn help_groups_join_all_keys_of_an_action() {
    let groups = Keymap::default().help_groups(KeyContext::ChatList);

    assert_eq!(key_label(&groups, "open_chat"), "Enter / l");
    assert_eq!(key_label(&groups, "toggle_chat_archive"), "A");
}

#[test]
fn rebinding_an_action_changes_its_key_label_and_keeps_its_row() {
    let mut overrides = std::collections::HashMap::new();
    overrides.insert("select_next_chat".to_owned(), "n".to_owned());
    let rebound = Keymap::with_overrides(&overrides).help_groups(KeyContext::ChatList);
    let default = Keymap::default().help_groups(KeyContext::ChatList);

    assert_eq!(group_actions(&rebound), group_actions(&default));
    assert_eq!(key_label(&rebound, "select_next_chat"), "n");
}

#[test]
fn every_action_belongs_to_exactly_one_hotkey_group() {
    for action in Action::iter() {
        let memberships = HOTKEY_GROUPS
            .iter()
            .flat_map(|(_, actions)| actions.iter())
            .filter(|grouped| **grouped == action)
            .count();
        assert_eq!(memberships, 1, "{action:?}");
    }
}
