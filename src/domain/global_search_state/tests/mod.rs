use super::*;

fn chat(chat_id: i64) -> FoundChat {
    FoundChat {
        chat_id,
        title: format!("Chat {chat_id}"),
        username: None,
        chat_type: ChatType::Private,
        is_forum: false,
    }
}

fn chats(ids: impl IntoIterator<Item = i64>) -> Vec<FoundChat> {
    ids.into_iter().map(chat).collect()
}

fn message(chat_id: i64, message_id: i64) -> FoundMessage {
    FoundMessage {
        chat: chat(chat_id),
        message_id,
        topic_id: None,
        sender_name: "Alice".to_owned(),
        timestamp_ms: 0,
        preview: "text".to_owned(),
    }
}

fn page(messages: Vec<FoundMessage>, next_offset: Option<&str>) -> FoundMessagesPage {
    FoundMessagesPage {
        messages,
        next_offset: next_offset.map(str::to_owned),
    }
}

fn searching(query: &str) -> GlobalSearchState {
    let mut state = GlobalSearchState::default();
    let now = Instant::now();
    query.chars().for_each(|ch| state.insert_char(ch, now));
    state
}

fn ids(chats: Vec<&FoundChat>) -> Vec<i64> {
    chats.into_iter().map(|chat| chat.chat_id).collect()
}

fn answer_all_chats(state: &mut GlobalSearchState, query: &str) {
    for scope in ChatScope::ALL {
        state.apply_chats(query, scope, Vec::new());
    }
}

#[test]
fn an_empty_popup_searches_for_nothing() {
    let state = GlobalSearchState::default();

    assert_eq!(state.search_text(), None);
    assert!(!state.is_searching());
    assert!(state.hits().is_empty());
    assert_eq!(state.selected_index(), None);
}

#[test]
fn a_blank_query_is_not_searched_for() {
    let mut state = searching("  ");

    assert_eq!(state.search_text(), None);
    assert!(!state.is_searching());
    assert!(!state.take_due_network_search(Instant::now() + Duration::from_secs(1)));
}

#[test]
fn the_query_is_searched_for_without_the_spaces_around_it() {
    assert_eq!(searching(" rust ").search_text(), Some("rust"));
}

#[test]
fn the_network_search_is_due_once_after_a_pause_in_typing() {
    let mut state = GlobalSearchState::default();
    let start = Instant::now();
    state.insert_char('r', start);

    assert!(!state.take_due_network_search(start + Duration::from_millis(299)));
    assert!(state.take_due_network_search(start + Duration::from_millis(300)));
    assert!(!state.take_due_network_search(start + Duration::from_secs(1)));
}

#[test]
fn typing_on_postpones_the_network_search() {
    let mut state = GlobalSearchState::default();
    let start = Instant::now();
    state.insert_char('r', start);
    state.insert_char('u', start + Duration::from_millis(200));

    assert!(!state.take_due_network_search(start + Duration::from_millis(400)));
    assert!(state.take_due_network_search(start + Duration::from_millis(500)));
}

#[test]
fn chats_of_the_three_account_scopes_are_joined_without_duplicates() {
    let mut state = searching("r");

    state.apply_chats("r", ChatScope::Contacts, chats([3, 4]));
    state.apply_chats("r", ChatScope::Server, chats([2, 3]));
    state.apply_chats("r", ChatScope::Known, chats([1, 2]));

    assert_eq!(ids(state.chats()), [1, 2, 3, 4]);
}

#[test]
fn the_chats_section_shows_at_most_ten_chats() {
    let mut state = searching("r");

    state.apply_chats("r", ChatScope::Known, chats(1..=8));
    state.apply_chats("r", ChatScope::Server, chats(9..=16));

    assert_eq!(ids(state.chats()), (1..=10).collect::<Vec<_>>());
}

#[test]
fn the_global_section_skips_shown_chats_and_shows_at_most_five() {
    let mut state = searching("r");

    state.apply_chats("r", ChatScope::Known, chats([1, 2]));
    state.apply_chats("r", ChatScope::Public, chats([2, 20, 21, 22, 23, 24, 25]));

    assert_eq!(ids(state.public_chats()), [20, 21, 22, 23, 24]);
}

#[test]
fn rows_go_chats_then_global_then_messages() {
    let mut state = searching("r");
    let found = message(1, 100);

    state.apply_messages("r", "", page(vec![found.clone()], None));
    state.apply_chats("r", ChatScope::Public, chats([20]));
    state.apply_chats("r", ChatScope::Known, chats([1]));

    assert_eq!(
        state.hits(),
        [
            SearchHit::Chat(&chat(1)),
            SearchHit::Chat(&chat(20)),
            SearchHit::Message(&found),
        ]
    );
}

#[test]
fn a_chat_may_be_both_in_chats_and_the_source_of_a_message() {
    let mut state = searching("r");

    state.apply_chats("r", ChatScope::Known, chats([1]));
    state.apply_messages("r", "", page(vec![message(1, 100)], None));

    assert_eq!(state.hits().len(), 2);
}

#[test]
fn an_answer_to_an_earlier_query_is_refused() {
    let mut state = searching("ru");

    assert!(!state.apply_chats("r", ChatScope::Known, chats([1])));
    assert!(!state.apply_messages("r", "", page(vec![message(1, 100)], None)));

    assert!(state.hits().is_empty());
    assert!(state.is_searching());
}

#[test]
fn a_scope_answers_a_query_once() {
    let mut state = searching("r");

    assert!(state.apply_chats("r", ChatScope::Known, chats([1])));
    assert!(!state.apply_chats("r", ChatScope::Known, chats([2])));

    assert_eq!(ids(state.chats()), [1]);
}

#[test]
fn editing_the_query_drops_the_results_and_the_selection() {
    let mut state = searching("r");
    state.apply_chats("r", ChatScope::Known, chats([1, 2]));
    state.apply_messages("r", "", page(vec![message(1, 100)], Some("next")));
    state.select_next();

    state.insert_char('u', Instant::now());

    assert!(state.hits().is_empty());
    assert_eq!(state.request_more_messages(), None);
    state.apply_chats("ru", ChatScope::Known, chats([5, 6]));
    assert_eq!(state.selected_index(), Some(0));
}

#[test]
fn erasing_the_whole_query_empties_the_popup() {
    let mut state = searching("r");
    state.apply_chats("r", ChatScope::Known, chats([1]));

    state.delete_char_before(Instant::now());

    assert_eq!(state.query(), "");
    assert!(state.hits().is_empty());
    assert!(!state.is_searching());
}

#[test]
fn searching_lasts_until_every_request_has_answered() {
    let mut state = searching("r");

    answer_all_chats(&mut state, "r");
    assert!(state.is_searching());

    state.apply_messages("r", "", page(Vec::new(), None));
    assert!(!state.is_searching());
}

#[test]
fn the_selection_walks_all_sections_as_one_list_and_stops_at_its_ends() {
    let mut state = searching("r");
    state.apply_chats("r", ChatScope::Known, chats([1]));
    state.apply_chats("r", ChatScope::Public, chats([20]));
    state.apply_messages("r", "", page(vec![message(1, 100)], None));

    state.select_previous();
    assert_eq!(state.selected(), Some(SearchHit::Chat(&chat(1))));
    state.select_next();
    assert_eq!(state.selected(), Some(SearchHit::Chat(&chat(20))));
    state.select_next();
    state.select_next();
    assert_eq!(state.selected(), Some(SearchHit::Message(&message(1, 100))));
}

#[test]
fn the_selection_stays_on_top_until_the_user_moves_it() {
    let mut state = searching("r");
    state.apply_chats("r", ChatScope::Server, chats([2]));

    state.apply_chats("r", ChatScope::Known, chats([1]));

    assert_eq!(state.selected(), Some(SearchHit::Chat(&chat(1))));
}

#[test]
fn a_moved_selection_follows_its_row_when_more_results_arrive() {
    let mut state = searching("r");
    state.apply_chats("r", ChatScope::Server, chats([2, 3]));
    state.select_next();

    state.apply_chats("r", ChatScope::Known, chats([1]));

    assert_eq!(state.selected(), Some(SearchHit::Chat(&chat(3))));
}

#[test]
fn the_next_messages_page_is_requested_once_at_the_last_row() {
    let mut state = searching("r");
    state.apply_messages(
        "r",
        "",
        page(vec![message(1, 100), message(1, 99)], Some("p2")),
    );

    assert_eq!(state.request_more_messages(), None);
    state.select_next();
    assert_eq!(state.request_more_messages(), Some("p2".to_owned()));
    assert_eq!(state.request_more_messages(), None);
    assert!(state.is_searching());
}

#[test]
fn the_next_messages_page_is_added_after_the_loaded_ones() {
    let mut state = searching("r");
    state.apply_messages("r", "", page(vec![message(1, 100)], Some("p2")));
    state.request_more_messages();

    assert!(state.apply_messages("r", "p2", page(vec![message(1, 99)], None)));

    assert_eq!(state.messages(), [message(1, 100), message(1, 99)]);
    assert_eq!(state.request_more_messages(), None);
}

#[test]
fn a_selected_chat_is_followed_from_global_into_chats() {
    let mut state = searching("r");
    state.apply_chats("r", ChatScope::Known, chats([1]));
    state.apply_chats("r", ChatScope::Public, chats([20, 21]));
    state.select_next();
    state.select_next();

    state.apply_chats("r", ChatScope::Server, chats([21, 2]));

    assert_eq!(state.selected(), Some(SearchHit::Chat(&chat(21))));
}

#[test]
fn a_page_that_failed_to_load_is_asked_for_again() {
    let mut state = searching("r");
    state.apply_messages("r", "", page(vec![message(1, 100)], Some("p2")));
    state.request_more_messages();

    assert!(state.fail_messages("r", "p2"));

    assert_eq!(state.messages(), [message(1, 100)]);
    assert_eq!(state.request_more_messages(), Some("p2".to_owned()));
}

#[test]
fn a_failure_of_a_page_that_is_not_awaited_is_ignored() {
    let mut state = searching("ru");

    assert!(!state.fail_messages("r", ""));
    assert!(state.is_searching());
}

#[test]
fn a_messages_page_that_is_not_awaited_is_refused() {
    let mut state = searching("r");
    state.apply_messages("r", "", page(vec![message(1, 100)], Some("p2")));

    assert!(!state.apply_messages("r", "", page(vec![message(1, 100)], Some("p2"))));
    assert!(!state.apply_messages("r", "p2", page(vec![message(1, 99)], None)));

    assert_eq!(state.messages(), [message(1, 100)]);
}
