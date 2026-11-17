use super::*;
use crate::{
    domain::{
        chat_info_state::{tests::sample_chat_info, ChatInfoPopupState, ChatInfoSubject},
        message::{LinkTarget, MessageSender, TextLink},
    },
    usecases::background::CHAT_INFO_NOT_FOUND,
};

const SENDER: MessageSender = MessageSender::User(7);

fn message_from(sender: MessageSender, mentions: &[&str]) -> Message {
    let mut msg = message(10, "text");
    msg.sender = Some(sender);
    msg.sender_name = "Bob".to_owned();
    msg.links = mentions
        .iter()
        .enumerate()
        .map(|(offset, username)| TextLink {
            offset,
            length: 1,
            target: LinkTarget::Mention((*username).to_owned()),
        })
        .collect();
    msg
}

fn press(o: &mut TestOrchestrator, key: &str) {
    o.handle_event(AppEvent::InputKey(KeyInput::new(key, false)))
        .unwrap();
}

fn chat_info_loaded(
    o: &mut TestOrchestrator,
    subject: ChatInfoSubject,
    result: Result<crate::domain::chat_info_state::ChatInfo, BackgroundError>,
) {
    o.handle_event(AppEvent::BackgroundTasksCompleted(vec![
        BackgroundTaskResult::ChatInfoLoaded { subject, result },
    ]))
    .unwrap();
}

fn mention(username: &str) -> ChatInfoSubject {
    ChatInfoSubject::Mention(username.to_owned())
}

/// Chat 1 is open on a message of Bob with these mentions, and `I` was pressed.
fn orchestrator_after_i(mentions: &[&str]) -> TestOrchestrator {
    let mut o = orchestrator_with_open_chat(
        vec![chat(1, "Group")],
        1,
        vec![message_from(SENDER, mentions)],
    );
    press(&mut o, "I");
    o
}

fn loading_subject(o: &TestOrchestrator) -> Option<ChatInfoSubject> {
    o.state().chat_info_popup()?.subject().cloned()
}

// ── I on a message ──

#[test]
fn i_on_a_message_without_mentions_opens_the_popup_about_its_sender() {
    let o = orchestrator_after_i(&[]);

    assert_eq!(loading_subject(&o), Some(ChatInfoSubject::User(7)));
    assert_eq!(o.state().chat_info_popup().unwrap().title(), "Bob");
    assert!(o.state().info_picker().is_none());
    assert!(o.state().message_info_popup().is_none());
    assert_eq!(
        o.dispatcher.last_chat_info_query().unwrap().subject,
        ChatInfoSubject::User(7)
    );
}

#[test]
fn i_on_a_message_sent_by_a_chat_opens_the_popup_about_that_chat() {
    let mut o = orchestrator_with_open_chat(
        vec![chat(1, "Group")],
        1,
        vec![message_from(MessageSender::Chat(-100), &[])],
    );

    press(&mut o, "I");

    assert_eq!(loading_subject(&o), Some(ChatInfoSubject::Chat(-100)));
}

#[test]
fn i_on_a_message_with_unknown_sender_does_nothing() {
    let mut msg = message(10, "text");
    msg.sender = None;
    let mut o = orchestrator_with_open_chat(vec![chat(1, "Group")], 1, vec![msg]);

    press(&mut o, "I");

    assert!(o.state().chat_info_popup().is_none());
    assert!(o.state().info_picker().is_none());
}

#[test]
fn i_on_a_message_with_a_mention_shows_the_info_picker() {
    let o = orchestrator_after_i(&["alice", "bob"]);

    let picker = o.state().info_picker().expect("info picker is open");
    let labels: Vec<&str> = picker.entries().iter().map(|e| e.label.as_str()).collect();
    assert_eq!(labels, ["Bob", "@alice", "@bob"]);
    assert_eq!(picker.selected_index(), Some(0));
    assert!(o.state().chat_info_popup().is_none());
    assert!(o.dispatcher.last_chat_info_query().is_none());
}

#[test]
fn i_closes_the_popup_opened_from_a_message() {
    let mut o = orchestrator_after_i(&[]);

    press(&mut o, "I");

    assert!(o.state().chat_info_popup().is_none());
}

// ── info picker ──

#[test]
fn picker_enter_opens_the_popup_about_the_sender() {
    let mut o = orchestrator_after_i(&["alice"]);

    press(&mut o, "enter");

    assert!(o.state().info_picker().is_none());
    assert_eq!(loading_subject(&o), Some(ChatInfoSubject::User(7)));
}

#[test]
fn picker_i_opens_the_popup_about_the_selected_mention() {
    let mut o = orchestrator_after_i(&["alice"]);

    press(&mut o, "j");
    press(&mut o, "I");

    assert!(o.state().info_picker().is_none());
    assert_eq!(loading_subject(&o), Some(mention("alice")));
    assert_eq!(o.state().chat_info_popup().unwrap().title(), "@alice");
    assert_eq!(
        o.dispatcher.last_chat_info_query().unwrap().subject,
        mention("alice")
    );
    assert!(o.state().active_notification().is_none());
    assert!(o.dispatcher.dispatched_mentions().is_empty());
}

#[test]
fn picker_k_moves_the_selection_back() {
    let mut o = orchestrator_after_i(&["alice"]);

    press(&mut o, "j");
    press(&mut o, "k");

    assert_eq!(o.state().info_picker().unwrap().selected_index(), Some(0));
}

#[test]
fn picker_q_and_esc_close_without_opening() {
    for key in ["q", "esc"] {
        let mut o = orchestrator_after_i(&["alice"]);

        press(&mut o, key);

        assert!(o.state().info_picker().is_none(), "key {key}");
        assert!(o.state().chat_info_popup().is_none(), "key {key}");
        assert!(o.state().is_running(), "key {key}");
        assert!(o.state().open_chat().is_open(), "key {key}");
    }
}

// ── answers ──

#[test]
fn loaded_info_replaces_the_loading_popup() {
    let mut o = orchestrator_after_i(&[]);

    chat_info_loaded(
        &mut o,
        ChatInfoSubject::User(7),
        Ok(sample_chat_info("Bob Smith")),
    );

    assert_eq!(o.state().chat_info_popup().unwrap().title(), "Bob Smith");
    assert_eq!(loading_subject(&o), None);
}

#[test]
fn unoccupied_mention_closes_the_popup_with_a_notification() {
    let mut o = orchestrator_after_i(&["alice"]);
    press(&mut o, "j");
    press(&mut o, "enter");

    chat_info_loaded(
        &mut o,
        mention("alice"),
        Err(BackgroundError::new(CHAT_INFO_NOT_FOUND)),
    );

    assert!(o.state().chat_info_popup().is_none());
    assert_eq!(o.state().active_notification(), Some("@alice not found"));
}

#[test]
fn failed_lookup_shows_the_error_state() {
    let mut o = orchestrator_after_i(&["alice"]);
    press(&mut o, "j");
    press(&mut o, "enter");

    chat_info_loaded(
        &mut o,
        mention("alice"),
        Err(BackgroundError::new("CHAT_INFO_UNAVAILABLE")),
    );

    assert_eq!(
        o.state().chat_info_popup(),
        Some(&ChatInfoPopupState::Error {
            title: "@alice".to_owned()
        })
    );
}

#[test]
fn answer_arriving_after_the_popup_was_closed_is_dropped() {
    let mut o = orchestrator_after_i(&["alice"]);
    press(&mut o, "j");
    press(&mut o, "enter");
    press(&mut o, "esc");

    chat_info_loaded(&mut o, mention("alice"), Ok(sample_chat_info("Alice")));
    chat_info_loaded(
        &mut o,
        mention("alice"),
        Err(BackgroundError::new(CHAT_INFO_NOT_FOUND)),
    );

    assert!(o.state().chat_info_popup().is_none());
    assert!(o.state().active_notification().is_none());
}

#[test]
fn answer_about_another_subject_is_dropped() {
    let mut o = orchestrator_after_i(&[]);

    chat_info_loaded(&mut o, mention("alice"), Ok(sample_chat_info("Alice")));

    assert_eq!(loading_subject(&o), Some(ChatInfoSubject::User(7)));
}
