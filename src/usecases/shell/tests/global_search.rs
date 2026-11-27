use super::*;
use crate::domain::{
    chat::ChatType,
    global_search_state::{ChatScope, FoundChat, FoundMessage, FoundMessagesPage, SearchHit},
};

fn press(o: &mut TestOrchestrator, key: &str) {
    o.handle_event(AppEvent::InputKey(KeyInput::new(key, false)))
        .unwrap();
}

fn press_ctrl(o: &mut TestOrchestrator, key: &str) {
    o.handle_event(AppEvent::InputKey(KeyInput::new(key, true)))
        .unwrap();
}

fn type_query(o: &mut TestOrchestrator, query: &str) {
    for ch in query.chars() {
        press(o, &ch.to_string());
    }
}

/// Lets the pause in typing pass and ticks.
fn pause_typing(o: &mut TestOrchestrator) {
    o.state_mut()
        .global_search_mut()
        .expect("global search is open")
        .end_typing_pause();
    o.handle_event(AppEvent::Tick).unwrap();
}

fn found_chat(chat_id: i64, title: &str) -> FoundChat {
    FoundChat {
        chat_id,
        title: title.to_owned(),
        username: None,
        chat_type: ChatType::Private,
        is_forum: false,
    }
}

fn found_forum(chat_id: i64, title: &str) -> FoundChat {
    FoundChat {
        chat_type: ChatType::Group,
        is_forum: true,
        ..found_chat(chat_id, title)
    }
}

fn found_message(chat: FoundChat, message_id: i64, topic_id: Option<i32>) -> FoundMessage {
    FoundMessage {
        chat,
        message_id,
        topic_id,
        sender_name: "Alice".to_owned(),
        timestamp_ms: 0,
        preview: "text".to_owned(),
    }
}

fn chats_found(
    o: &mut TestOrchestrator,
    query: &str,
    scope: ChatScope,
    result: Result<Vec<FoundChat>, BackgroundError>,
) {
    o.handle_event(AppEvent::BackgroundTasksCompleted(vec![
        BackgroundTaskResult::ChatsFound {
            query: query.to_owned(),
            scope,
            result,
        },
    ]))
    .unwrap();
}

fn messages_found(
    o: &mut TestOrchestrator,
    query: &str,
    offset: &str,
    result: Result<FoundMessagesPage, BackgroundError>,
) {
    o.handle_event(AppEvent::BackgroundTasksCompleted(vec![
        BackgroundTaskResult::MessagesFound {
            query: query.to_owned(),
            offset: offset.to_owned(),
            result,
        },
    ]))
    .unwrap();
}

fn page(messages: Vec<FoundMessage>, next_offset: Option<&str>) -> FoundMessagesPage {
    FoundMessagesPage {
        messages,
        next_offset: next_offset.map(str::to_owned),
    }
}

/// The chat list holds chats 1 and 2, and global search is open on `query`.
fn searching(query: &str) -> TestOrchestrator {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alice"), chat(2, "Bob")]);
    press(&mut o, "s");
    type_query(&mut o, query);
    o
}

fn search_error() -> BackgroundError {
    BackgroundError::new("GLOBAL_SEARCH_UNAVAILABLE")
}

// ── opening and closing ──

#[test]
fn s_in_the_chat_list_opens_an_empty_popup_that_asks_telegram_nothing() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Alice")]);

    press(&mut o, "s");

    let search = o.state().global_search().expect("global search is open");
    assert_eq!(search.query(), "");
    assert!(!search.is_searching());
    assert!(o.dispatcher.dispatched_chat_searches().is_empty());
}

#[test]
fn s_in_the_messages_panel_does_not_open_global_search() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "Alice")], 1, vec![message(1, "hi")]);

    press(&mut o, "s");

    assert!(o.state().global_search().is_none());
}

#[test]
fn esc_closes_the_popup_and_leaves_the_chat_list_as_it_was() {
    let mut o = searching("bo");

    press(&mut o, "esc");

    assert!(o.state().global_search().is_none());
    assert_eq!(o.state().chat_list().selected_chat().unwrap().chat_id, 1);
    assert_eq!(o.state().active_pane(), ActivePane::ChatList);
}

// ── the query and the requests ──

#[test]
fn every_character_goes_into_the_query_even_a_hotkey() {
    let o = searching("qs?/");

    assert_eq!(o.state().global_search().unwrap().query(), "qs?/");
    assert!(o.state().is_running());
}

#[test]
fn known_chats_are_searched_on_every_change_of_the_query() {
    let mut o = searching("ru");
    press(&mut o, "backspace");

    assert_eq!(
        o.dispatcher.dispatched_chat_searches(),
        [
            ("r".to_owned(), ChatScope::Known),
            ("ru".to_owned(), ChatScope::Known),
            ("r".to_owned(), ChatScope::Known),
        ]
    );
}

#[test]
fn the_server_is_asked_only_after_a_pause_in_typing() {
    let mut o = searching("ru");
    o.handle_event(AppEvent::Tick).unwrap();
    assert!(o.dispatcher.dispatched_message_searches().is_empty());

    pause_typing(&mut o);
    o.handle_event(AppEvent::Tick).unwrap();

    let network: Vec<_> = o
        .dispatcher
        .dispatched_chat_searches()
        .into_iter()
        .filter(|(_, scope)| *scope != ChatScope::Known)
        .collect();
    assert_eq!(
        network,
        [
            ("ru".to_owned(), ChatScope::Server),
            ("ru".to_owned(), ChatScope::Contacts),
            ("ru".to_owned(), ChatScope::Public),
        ]
    );
    assert_eq!(
        o.dispatcher.dispatched_message_searches(),
        [("ru".to_owned(), String::new())]
    );
}

#[test]
fn erasing_the_query_searches_for_nothing() {
    let mut o = searching("r");

    press(&mut o, "backspace");
    pause_typing(&mut o);

    assert_eq!(o.dispatcher.dispatched_chat_searches().len(), 1);
    assert!(o.dispatcher.dispatched_message_searches().is_empty());
}

// ── answers ──

#[test]
fn found_chats_and_messages_show_up_in_the_popup() {
    let mut o = searching("r");
    let message = found_message(found_chat(7, "Rust"), 100, None);

    chats_found(
        &mut o,
        "r",
        ChatScope::Known,
        Ok(vec![found_chat(7, "Rust")]),
    );
    messages_found(&mut o, "r", "", Ok(page(vec![message.clone()], None)));

    assert_eq!(
        o.state().global_search().unwrap().hits(),
        [
            SearchHit::Chat(&found_chat(7, "Rust")),
            SearchHit::Message(&message),
        ]
    );
}

#[test]
fn an_answer_to_an_earlier_query_is_dropped() {
    let mut o = searching("ru");

    chats_found(
        &mut o,
        "r",
        ChatScope::Known,
        Ok(vec![found_chat(7, "Rust")]),
    );
    chats_found(&mut o, "r", ChatScope::Public, Err(search_error()));

    assert!(o.state().global_search().unwrap().hits().is_empty());
    assert_eq!(o.state().active_notification(), None);
}

#[test]
fn an_answer_that_comes_after_the_popup_was_closed_is_dropped() {
    let mut o = searching("r");
    press(&mut o, "esc");

    chats_found(&mut o, "r", ChatScope::Known, Err(search_error()));

    assert!(o.state().global_search().is_none());
    assert_eq!(o.state().active_notification(), None);
}

#[test]
fn a_failed_request_goes_to_the_status_bar_and_leaves_other_sections_alone() {
    let mut o = searching("r");
    chats_found(
        &mut o,
        "r",
        ChatScope::Known,
        Ok(vec![found_chat(7, "Rust")]),
    );

    chats_found(&mut o, "r", ChatScope::Public, Err(search_error()));
    assert_eq!(
        o.state().active_notification(),
        Some("Could not search public chats")
    );

    messages_found(&mut o, "r", "", Err(search_error()));
    assert_eq!(
        o.state().active_notification(),
        Some("Could not search messages")
    );

    let search = o.state().global_search().unwrap();
    assert_eq!(search.hits(), [SearchHit::Chat(&found_chat(7, "Rust"))]);
}

// ── moving the selection ──

#[test]
fn ctrl_j_ctrl_k_and_the_arrows_move_the_selection() {
    let mut o = searching("r");
    chats_found(
        &mut o,
        "r",
        ChatScope::Known,
        Ok(vec![
            found_chat(7, "A"),
            found_chat(8, "B"),
            found_chat(9, "C"),
        ]),
    );
    let selected = |o: &TestOrchestrator| o.state().global_search().unwrap().selected_index();

    press_ctrl(&mut o, "j");
    press(&mut o, "down");
    assert_eq!(selected(&o), Some(2));

    press_ctrl(&mut o, "k");
    press(&mut o, "up");
    assert_eq!(selected(&o), Some(0));
    assert_eq!(o.state().global_search().unwrap().query(), "r");
}

#[test]
fn reaching_the_last_row_loads_the_next_page_of_messages() {
    let mut o = searching("r");
    let chat = found_chat(7, "Rust");
    messages_found(
        &mut o,
        "r",
        "",
        Ok(page(
            vec![
                found_message(chat.clone(), 100, None),
                found_message(chat.clone(), 99, None),
            ],
            Some("p2"),
        )),
    );

    press_ctrl(&mut o, "j");
    press_ctrl(&mut o, "j");

    assert_eq!(
        o.dispatcher.dispatched_message_searches(),
        [("r".to_owned(), "p2".to_owned())]
    );

    messages_found(
        &mut o,
        "r",
        "p2",
        Ok(page(vec![found_message(chat, 98, None)], None)),
    );
    assert_eq!(o.state().global_search().unwrap().messages().len(), 3);
}

#[test]
fn a_page_without_messages_that_is_not_the_last_is_followed_by_the_next_one() {
    let mut o = searching("r");

    messages_found(&mut o, "r", "", Ok(page(Vec::new(), Some("p2"))));

    assert_eq!(
        o.dispatcher.dispatched_message_searches(),
        [("r".to_owned(), "p2".to_owned())]
    );
    assert!(o.state().global_search().unwrap().is_searching());
}

#[test]
fn a_page_that_failed_to_load_is_asked_for_again_at_the_last_row() {
    let mut o = searching("r");
    let chat = found_chat(7, "Rust");
    messages_found(
        &mut o,
        "r",
        "",
        Ok(page(vec![found_message(chat, 100, None)], Some("p2"))),
    );
    press_ctrl(&mut o, "j");
    messages_found(&mut o, "r", "p2", Err(search_error()));

    press_ctrl(&mut o, "j");

    assert_eq!(
        o.dispatcher.dispatched_message_searches(),
        [
            ("r".to_owned(), "p2".to_owned()),
            ("r".to_owned(), "p2".to_owned()),
        ]
    );
}

// ── Enter ──

#[test]
fn enter_on_a_chat_of_the_active_list_opens_it_and_selects_it_there() {
    let mut o = searching("bo");
    chats_found(
        &mut o,
        "bo",
        ChatScope::Known,
        Ok(vec![found_chat(2, "Bob")]),
    );

    press(&mut o, "enter");

    assert!(o.state().global_search().is_none());
    assert_eq!(o.state().open_chat().chat_id(), Some(2));
    assert_eq!(o.state().open_chat().chat_title(), "Bob");
    assert_eq!(o.state().active_pane(), ActivePane::Messages);
    assert_eq!(o.state().chat_list().selected_chat().unwrap().chat_id, 2);
    assert_eq!(o.dispatcher.last_load_messages(), Some((2, None)));
}

#[test]
fn enter_on_a_chat_outside_the_active_list_opens_it_and_keeps_the_list() {
    let mut o = searching("ru");
    chats_found(
        &mut o,
        "ru",
        ChatScope::Public,
        Ok(vec![found_chat(50, "Rust News")]),
    );

    press(&mut o, "enter");

    assert_eq!(o.state().open_chat().chat_id(), Some(50));
    assert_eq!(o.state().chat_list().selected_chat().unwrap().chat_id, 1);
    assert_eq!(o.state().chat_list().chats().len(), 2);
}

#[test]
fn enter_on_a_forum_opens_its_topic_list() {
    let mut o = searching("fo");
    chats_found(
        &mut o,
        "fo",
        ChatScope::Known,
        Ok(vec![found_forum(60, "Forum")]),
    );

    press(&mut o, "enter");

    assert_eq!(
        o.state()
            .forum_topic_list()
            .map(|list| list.parent_chat_id()),
        Some(60)
    );
    assert_eq!(o.state().active_pane(), ActivePane::ChatList);
}

#[test]
fn enter_on_a_message_opens_its_chat_at_the_latest_messages() {
    let mut o = searching("hi");
    messages_found(
        &mut o,
        "hi",
        "",
        Ok(page(
            vec![found_message(found_chat(2, "Bob"), 100, None)],
            None,
        )),
    );

    press(&mut o, "enter");

    assert!(o.state().global_search().is_none());
    assert_eq!(o.state().open_chat().chat_id(), Some(2));
    assert_eq!(o.state().open_chat().topic_id(), None);
    assert_eq!(o.dispatcher.last_load_messages(), Some((2, None)));
}

#[test]
fn enter_on_a_message_of_a_forum_topic_opens_that_topic_at_once() {
    let mut o = searching("hi");
    messages_found(
        &mut o,
        "hi",
        "",
        Ok(page(
            vec![found_message(found_forum(60, "Forum"), 100, Some(5))],
            None,
        )),
    );

    press(&mut o, "enter");

    assert_eq!(o.state().open_chat().chat_id(), Some(60));
    assert_eq!(o.state().open_chat().topic_id(), Some(5));
    assert_eq!(o.state().active_pane(), ActivePane::Messages);
    assert_eq!(o.dispatcher.last_load_messages(), Some((60, Some(5))));
    assert_eq!(o.dispatcher.last_forum_topics_chat_id(), Some(60));
}

#[test]
fn a_topic_opened_from_a_message_is_titled_once_the_topics_are_loaded() {
    let mut o = searching("hi");
    messages_found(
        &mut o,
        "hi",
        "",
        Ok(page(
            vec![found_message(found_forum(60, "Forum"), 100, Some(5))],
            None,
        )),
    );
    press(&mut o, "enter");
    assert_eq!(o.state().open_chat().chat_title(), "Forum");

    inject_forum_topics(&mut o, 60, vec![topic(60, 5, "Releases", 1)]);

    assert_eq!(o.state().open_chat().chat_title(), "Forum > Releases");
    assert_eq!(o.state().open_chat().topic_id(), Some(5));
}

#[test]
fn h_from_a_topic_opened_from_a_message_leads_to_the_topic_list() {
    let mut o = searching("hi");
    messages_found(
        &mut o,
        "hi",
        "",
        Ok(page(
            vec![found_message(found_forum(60, "Forum"), 100, Some(5))],
            None,
        )),
    );
    press(&mut o, "enter");

    press(&mut o, "h");

    assert_eq!(
        o.state().key_context(),
        crate::domain::keymap::KeyContext::ForumTopicList
    );
}

#[test]
fn enter_without_results_keeps_the_popup_open() {
    let mut o = searching("zz");

    press(&mut o, "enter");

    assert!(o.state().global_search().is_some());
    assert!(!o.state().open_chat().is_open());
}
