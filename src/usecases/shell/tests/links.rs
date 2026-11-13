use super::*;
use crate::domain::{
    message::{LinkTarget, TextLink},
    shell_state::ActivePane,
};

fn url(target: &str) -> LinkTarget {
    LinkTarget::Url(target.to_owned())
}

fn mention(username: &str) -> LinkTarget {
    LinkTarget::Mention(username.to_owned())
}

fn message_with(targets: Vec<LinkTarget>) -> Message {
    let mut msg = message(10, "text");
    msg.links = targets
        .into_iter()
        .enumerate()
        .map(|(offset, target)| TextLink {
            offset,
            length: 1,
            target,
        })
        .collect();
    msg
}

fn press(o: &mut TestOrchestrator, key: &str) {
    o.handle_event(AppEvent::InputKey(KeyInput::new(key, false)))
        .unwrap();
}

fn resolve_mention(
    o: &mut TestOrchestrator,
    username: &str,
    result: Result<Option<ChatSummary>, BackgroundError>,
) {
    o.handle_event(AppEvent::BackgroundTasksCompleted(vec![
        BackgroundTaskResult::MentionResolved {
            username: username.to_owned(),
            result,
        },
    ]))
    .unwrap();
}

/// Chat 1 is open on a message that mentions `@alice`, and `o` was pressed.
fn orchestrator_opening_alice(chats: Vec<ChatSummary>) -> TestOrchestrator {
    let mut o = orchestrator_with_open_chat(chats, 1, vec![message_with(vec![mention("alice")])]);
    press(&mut o, "o");
    o
}

// ── o on a message ──

#[test]
fn o_opens_the_only_link_without_a_picker() {
    let mut o = orchestrator_with_open_chat(
        vec![chat(1, "Chat")],
        1,
        vec![message_with(vec![url("https://example.com")])],
    );

    press(&mut o, "o");

    assert_eq!(o.opener.opened_urls(), vec!["https://example.com"]);
    assert!(o.state().link_picker().is_none());
}

#[test]
fn o_starts_opening_the_only_mention() {
    let o = orchestrator_opening_alice(vec![chat(1, "Chat")]);

    assert_eq!(o.dispatcher.dispatched_mentions(), vec!["alice"]);
    assert_eq!(o.state().active_notification(), Some("Opening @alice..."));
    assert!(o.state().link_picker().is_none());
    assert!(o.opener.opened_urls().is_empty());
}

#[test]
fn o_shows_the_picker_for_several_targets() {
    let mut o = orchestrator_with_open_chat(
        vec![chat(1, "Chat")],
        1,
        vec![message_with(vec![
            mention("alice"),
            url("https://example.com"),
        ])],
    );

    press(&mut o, "o");

    let picker = o.state().link_picker().expect("link picker is open");
    assert_eq!(
        picker.targets(),
        [mention("alice"), url("https://example.com")]
    );
    assert!(o.opener.opened_urls().is_empty());
    assert!(o.dispatcher.dispatched_mentions().is_empty());
}

// ── link picker ──

fn orchestrator_with_picker() -> TestOrchestrator {
    let mut o = orchestrator_with_open_chat(
        vec![chat(1, "Chat")],
        1,
        vec![message_with(vec![
            url("https://first.com"),
            mention("alice"),
            url("https://last.com"),
        ])],
    );
    press(&mut o, "o");
    o
}

#[test]
fn picker_enter_opens_the_selected_link_and_closes() {
    let mut o = orchestrator_with_picker();

    press(&mut o, "j");
    press(&mut o, "j");
    press(&mut o, "enter");

    assert_eq!(o.opener.opened_urls(), vec!["https://last.com"]);
    assert!(o.state().link_picker().is_none());
}

#[test]
fn picker_o_opens_the_selected_mention_and_closes() {
    let mut o = orchestrator_with_picker();

    press(&mut o, "j");
    press(&mut o, "j");
    press(&mut o, "k");
    press(&mut o, "o");

    assert_eq!(o.dispatcher.dispatched_mentions(), vec!["alice"]);
    assert!(o.opener.opened_urls().is_empty());
    assert!(o.state().link_picker().is_none());
}

#[test]
fn picker_q_and_esc_close_without_opening() {
    for key in ["q", "esc"] {
        let mut o = orchestrator_with_picker();

        press(&mut o, key);

        assert!(o.state().link_picker().is_none(), "key {key}");
        assert!(o.opener.opened_urls().is_empty(), "key {key}");
        assert!(o.dispatcher.dispatched_mentions().is_empty(), "key {key}");
    }
}

#[test]
fn picker_captures_keys_of_the_messages_pane() {
    let mut o = orchestrator_with_picker();

    press(&mut o, "h");

    assert!(o.state().link_picker().is_some());
    assert_eq!(o.state().active_pane(), ActivePane::Messages);
}

// ── opening the chat behind a mention ──

#[test]
fn resolved_mention_opens_a_chat_outside_the_list() {
    let mut o = orchestrator_opening_alice(vec![chat(1, "Chat"), chat(2, "Bob")]);

    resolve_mention(&mut o, "alice", Ok(Some(chat(7, "Alice"))));

    assert_eq!(o.state().open_chat().chat_id(), Some(7));
    assert_eq!(o.state().open_chat().chat_title(), "Alice");
    assert_eq!(o.state().active_pane(), ActivePane::Messages);
    assert_eq!(o.dispatcher.last_load_messages(), Some((7, None)));
    assert_eq!(o.dispatcher.last_close_chat(), Some(1));
    assert_eq!(o.dispatcher.last_open_chat(), Some(7));
    assert_eq!(
        o.state().chat_list().selected_chat().map(|c| c.chat_id),
        Some(1),
        "a chat outside the list leaves the selection alone"
    );
}

#[test]
fn resolved_mention_selects_the_chat_when_it_is_listed() {
    let mut o = orchestrator_opening_alice(vec![chat(1, "Chat"), chat(7, "Alice")]);

    resolve_mention(&mut o, "alice", Ok(Some(chat(7, "Alice"))));

    assert_eq!(o.state().open_chat().chat_id(), Some(7));
    assert_eq!(
        o.state().chat_list().selected_chat().map(|c| c.chat_id),
        Some(7)
    );
}

#[test]
fn resolved_forum_mention_enters_its_topic_list() {
    let mut o = orchestrator_opening_alice(vec![chat(1, "Chat")]);

    resolve_mention(&mut o, "alice", Ok(Some(forum_chat(100, "Topics"))));

    let forum_list = o.state().forum_topic_list().expect("forum is entered");
    assert_eq!(forum_list.parent_chat_id(), 100);
    assert_eq!(o.state().active_pane(), ActivePane::ChatList);
    assert_eq!(o.dispatcher.last_forum_topics_chat_id(), Some(100));
}

#[test]
fn mention_opened_from_a_forum_topic_leaves_the_forum() {
    let mut o = orchestrator_with_chats(vec![forum_chat(100, "Topics")]);
    press(&mut o, "enter");
    inject_forum_topics(&mut o, 100, vec![topic(100, 1, "Alpha", 1000)]);
    press(&mut o, "enter");
    inject_messages(&mut o, 100, vec![message_with(vec![mention("alice")])]);
    press(&mut o, "o");

    resolve_mention(&mut o, "alice", Ok(Some(chat(7, "Alice"))));

    assert!(o.state().forum_topic_list().is_none());
    assert_eq!(o.state().open_chat().chat_id(), Some(7));
    assert_eq!(o.state().open_chat().topic_id(), None);
    assert_eq!(o.state().active_pane(), ActivePane::Messages);
}

#[test]
fn mention_of_the_open_chat_changes_nothing() {
    let mut o = orchestrator_opening_alice(vec![chat(1, "Chat")]);
    let loads_before = o.dispatcher.messages_dispatch_count();

    resolve_mention(&mut o, "alice", Ok(Some(chat(1, "Chat"))));

    assert_eq!(o.state().open_chat().chat_id(), Some(1));
    assert_eq!(o.dispatcher.messages_dispatch_count(), loads_before);
}

#[test]
fn unknown_mention_is_reported_and_the_chat_stays() {
    let mut o = orchestrator_opening_alice(vec![chat(1, "Chat")]);

    resolve_mention(&mut o, "alice", Ok(None));

    assert_eq!(o.state().active_notification(), Some("@alice not found"));
    assert_eq!(o.state().open_chat().chat_id(), Some(1));
}

#[test]
fn failed_mention_lookup_is_reported() {
    let mut o = orchestrator_opening_alice(vec![chat(1, "Chat")]);

    resolve_mention(
        &mut o,
        "alice",
        Err(BackgroundError::new("MENTION_UNAVAILABLE")),
    );

    assert_eq!(
        o.state().active_notification(),
        Some("Could not open @alice")
    );
    assert_eq!(o.state().open_chat().chat_id(), Some(1));
}

#[test]
fn late_mention_is_dropped_after_returning_to_the_chat_list() {
    let mut o = orchestrator_opening_alice(vec![chat(1, "Chat")]);
    press(&mut o, "h");

    resolve_mention(&mut o, "alice", Ok(Some(chat(7, "Alice"))));

    assert_eq!(o.state().open_chat().chat_id(), Some(1));
    assert_eq!(o.state().active_pane(), ActivePane::ChatList);
}

#[test]
fn late_mention_is_dropped_after_opening_another_chat() {
    let mut o = orchestrator_opening_alice(vec![chat(1, "Chat"), chat(2, "Bob")]);
    press(&mut o, "h");
    press(&mut o, "j");
    press(&mut o, "enter");

    resolve_mention(&mut o, "alice", Ok(Some(chat(7, "Alice"))));

    assert_eq!(o.state().open_chat().chat_id(), Some(2));
}

#[test]
fn only_the_latest_mention_request_is_honoured() {
    let mut o = orchestrator_with_open_chat(
        vec![chat(1, "Chat")],
        1,
        vec![message_with(vec![mention("alice"), mention("bob")])],
    );
    press(&mut o, "o");
    press(&mut o, "enter");
    press(&mut o, "o");
    press(&mut o, "j");
    press(&mut o, "enter");

    resolve_mention(&mut o, "alice", Ok(Some(chat(7, "Alice"))));
    assert_eq!(o.state().open_chat().chat_id(), Some(1));

    resolve_mention(&mut o, "bob", Ok(Some(chat(8, "Bob"))));
    assert_eq!(o.state().open_chat().chat_id(), Some(8));
}
