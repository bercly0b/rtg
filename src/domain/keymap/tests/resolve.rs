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
fn chat_info_is_on_the_same_key_in_the_chat_list_and_in_messages() {
    let mut km = Keymap::default();
    for context in [KeyContext::ChatList, KeyContext::Messages] {
        assert_eq!(
            km.resolve("I", false, context),
            ResolveResult::Action(Action::ShowChatInfo)
        );
    }
    assert_eq!(
        km.resolve("m", false, KeyContext::Messages),
        ResolveResult::Action(Action::ShowMessageInfo)
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
fn shift_u_installs_an_update_in_every_context() {
    let mut km = Keymap::default();
    for context in [
        KeyContext::ChatList,
        KeyContext::ForumTopicList,
        KeyContext::Messages,
    ] {
        assert_eq!(
            km.resolve("U", false, context),
            ResolveResult::Action(Action::InstallUpdate),
            "{context:?}"
        );
    }
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
fn shift_a_attaches_a_file_in_messages_only() {
    let mut km = Keymap::default();

    assert_eq!(
        km.resolve("A", false, KeyContext::Messages),
        ResolveResult::Action(Action::AttachFile)
    );
    assert_eq!(
        km.resolve("a", false, KeyContext::Messages),
        ResolveResult::Unmatched
    );
    assert_eq!(Action::from_name("attach_file"), Some(Action::AttachFile));
}

#[test]
fn gg_sequence_resolved_in_chat_list() {
    let mut km = Keymap::default();
    assert_eq!(
        km.resolve("g", false, KeyContext::ChatList),
        ResolveResult::Pending
    );
    assert_eq!(
        km.resolve("g", false, KeyContext::ChatList),
        ResolveResult::Action(Action::SelectFirstChat)
    );
}

#[test]
fn gg_sequence_not_in_messages() {
    let mut km = Keymap::default();
    assert_eq!(
        km.resolve("g", false, KeyContext::Messages),
        ResolveResult::Unmatched
    );
}

#[test]
fn shift_g_scrolls_to_last_message() {
    let mut km = Keymap::default();
    assert_eq!(
        km.resolve("G", false, KeyContext::Messages),
        ResolveResult::Action(Action::ScrollToLastMessage)
    );
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

#[test]
fn forum_topic_list_navigation_keys_resolve() {
    let mut km = Keymap::default();
    assert_eq!(
        km.resolve("j", false, KeyContext::ForumTopicList),
        ResolveResult::Action(Action::SelectNextTopic)
    );
    assert_eq!(
        km.resolve("k", false, KeyContext::ForumTopicList),
        ResolveResult::Action(Action::SelectPreviousTopic)
    );
}

#[test]
fn forum_topic_list_open_keys_resolve_to_open_forum_topic() {
    let mut km = Keymap::default();
    assert_eq!(
        km.resolve("enter", false, KeyContext::ForumTopicList),
        ResolveResult::Action(Action::OpenForumTopic)
    );
    let mut km = Keymap::default();
    assert_eq!(
        km.resolve("l", false, KeyContext::ForumTopicList),
        ResolveResult::Action(Action::OpenForumTopic)
    );
}

#[test]
fn h_in_forum_topic_list_returns_to_root_chat_list() {
    let mut km = Keymap::default();
    assert_eq!(
        km.resolve("h", false, KeyContext::ForumTopicList),
        ResolveResult::Action(Action::BackFromForum)
    );
}

#[test]
fn gg_in_forum_topic_list_selects_first_topic() {
    let mut km = Keymap::default();
    assert_eq!(
        km.resolve("g", false, KeyContext::ForumTopicList),
        ResolveResult::Pending
    );
    assert_eq!(
        km.resolve("g", false, KeyContext::ForumTopicList),
        ResolveResult::Action(Action::SelectFirstTopic)
    );
}

#[test]
fn open_chat_action_is_not_resolved_in_forum_topic_list_context() {
    let mut km = Keymap::default();
    // `l` resolves to OpenForumTopic, never OpenChat, in ForumTopicList ctx.
    match km.resolve("l", false, KeyContext::ForumTopicList) {
        ResolveResult::Action(Action::OpenForumTopic) => {}
        other => panic!("expected OpenForumTopic, got {other:?}"),
    }
}

#[test]
fn f_shows_chat_folders_only_in_chat_list() {
    let mut km = Keymap::default();
    assert_eq!(
        km.resolve("f", false, KeyContext::ChatList),
        ResolveResult::Action(Action::ShowChatFolders)
    );
    assert_eq!(
        km.resolve("f", false, KeyContext::ForumTopicList),
        ResolveResult::Unmatched
    );
    assert_eq!(
        km.resolve("f", false, KeyContext::Messages),
        ResolveResult::Unmatched
    );
}

#[test]
fn shift_a_toggles_chat_archive_only_in_chat_list() {
    let mut km = Keymap::default();
    assert_eq!(
        km.resolve("A", false, KeyContext::ChatList),
        ResolveResult::Action(Action::ToggleChatArchive)
    );
    assert_eq!(
        km.resolve("A", false, KeyContext::ForumTopicList),
        ResolveResult::Unmatched
    );
    assert_eq!(
        km.resolve("A", false, KeyContext::Messages),
        ResolveResult::Action(Action::AttachFile)
    );
}

#[test]
fn reload_forum_topics_resolves_in_forum_context() {
    let mut km = Keymap::default();
    assert_eq!(
        km.resolve("R", false, KeyContext::ForumTopicList),
        ResolveResult::Action(Action::ReloadForumTopics)
    );
}

#[test]
fn shift_l_toggles_the_layout_in_every_context() {
    let mut km = Keymap::default();
    for context in [
        KeyContext::ChatList,
        KeyContext::ForumTopicList,
        KeyContext::Messages,
    ] {
        assert_eq!(
            km.resolve("L", false, context),
            ResolveResult::Action(Action::ToggleLayout),
            "{context:?}"
        );
    }
}
