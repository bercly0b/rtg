use super::*;

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
        Action::SelectFirstChat,
        Action::ShowChatFolders,
        Action::ToggleChatArchive,
        Action::DeleteMessage,
        Action::Quit,
        Action::ShowHelp,
        Action::InstallUpdate,
    ];
    for action in actions {
        let name = action.display_name();
        assert_eq!(Action::from_name(name), Some(action));
    }
}

#[test]
fn open_link_action_keeps_its_old_config_name() {
    assert_eq!(Action::OpenLink.display_name(), "open_link_or_mention");
    assert_eq!(
        Action::from_name("open_link_or_mention"),
        Some(Action::OpenLink)
    );
    assert_eq!(
        Action::from_name("open_link_in_browser"),
        Some(Action::OpenLink)
    );
}

#[test]
fn back_from_forum_can_be_remapped_via_override() {
    let mut overrides = std::collections::HashMap::new();
    overrides.insert("back_from_forum".to_owned(), "b".to_owned());
    let mut km = Keymap::with_overrides(&overrides);
    assert_eq!(
        km.resolve("b", false, KeyContext::ForumTopicList),
        ResolveResult::Action(Action::BackFromForum)
    );
}
