use std::time::Instant;

use super::*;
use crate::domain::{
    chat::ChatType,
    global_search_state::{ChatScope, FoundMessagesPage},
};

const WIDTH: usize = 40;

fn at(year: i16, month: i8, day: i8, hour: i8, minute: i8) -> LocalTime {
    LocalTime::from_local(year, month, day, hour, minute)
}

fn now() -> LocalTime {
    at(2026, 11, 16, 18, 0)
}

fn chat(chat_id: i64, title: &str, username: Option<&str>) -> FoundChat {
    FoundChat {
        chat_id,
        title: title.to_owned(),
        username: username.map(str::to_owned),
        chat_type: ChatType::Private,
        is_forum: false,
    }
}

fn message(chat_title: &str, sender: &str, sent: LocalTime, preview: &str) -> FoundMessage {
    FoundMessage {
        chat: chat(1, chat_title, None),
        message_id: sent.unix_seconds(),
        topic_id: None,
        sender_name: sender.to_owned(),
        timestamp_ms: sent.unix_millis(),
        preview: preview.to_owned(),
    }
}

/// A search for `r` that every request has answered.
fn finished(
    chats: Vec<FoundChat>,
    public_chats: Vec<FoundChat>,
    messages: Vec<FoundMessage>,
) -> GlobalSearchState {
    let mut state = GlobalSearchState::default();
    state.insert_char('r', Instant::now());
    state.apply_chats("r", ChatScope::Known, chats);
    state.apply_chats("r", ChatScope::Server, Vec::new());
    state.apply_chats("r", ChatScope::Contacts, Vec::new());
    state.apply_chats("r", ChatScope::Public, public_chats);
    state.apply_messages(
        "r",
        "",
        FoundMessagesPage {
            messages,
            next_offset: None,
        },
    );
    state
}

fn line_text(line: &Line<'_>) -> String {
    line.spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect()
}

fn texts(state: &GlobalSearchState, list_height: usize) -> Vec<String> {
    build_lines(state, WIDTH, list_height, now())
        .iter()
        .map(|line| line_text(line).trim_end().to_owned())
        .collect()
}

fn highlighted(state: &GlobalSearchState, list_height: usize) -> Vec<String> {
    build_lines(state, WIDTH, list_height, now())
        .iter()
        .filter(|line| {
            line.spans
                .first()
                .is_some_and(|span| span.style.bg == Some(Color::White))
        })
        .map(|line| line_text(line).trim_end().to_owned())
        .collect()
}

#[test]
fn an_empty_query_shows_an_empty_popup() {
    let lines = texts(&GlobalSearchState::default(), 3);

    assert_eq!(lines, [">", "", "", "", "", "", FOOTER]);
}

#[test]
fn results_are_listed_in_sections_under_the_query() {
    let state = finished(
        vec![chat(1, "Rust chat", Some("rustlang"))],
        vec![chat(2, "Rust News", None)],
        vec![message(
            "Rust chat",
            "Bob",
            at(2026, 11, 12, 9, 5),
            "did you see the release",
        )],
    );

    assert_eq!(
        texts(&state, 8),
        [
            "> r",
            "",
            "Chats",
            "  Rust chat  @rustlang",
            "Global",
            "  Rust News",
            "Messages",
            "  Rust chat · Bob                 12 Nov",
            "    did you see the release",
            "",
            "",
            FOOTER,
        ]
    );
}

#[test]
fn a_section_that_found_nothing_is_left_out_with_its_title() {
    let state = finished(Vec::new(), vec![chat(2, "Rust News", None)], Vec::new());

    assert_eq!(texts(&state, 2)[2..4], ["Global", "  Rust News"]);
}

#[test]
fn a_message_shows_its_time_today_and_its_year_when_it_is_old() {
    let state = finished(
        Vec::new(),
        Vec::new(),
        vec![
            message("Chat", "Bob", at(2026, 11, 16, 14, 32), "today"),
            message("Chat", "Bob", at(2025, 3, 1, 9, 5), "old"),
        ],
    );

    let lines = texts(&state, 5);

    assert_eq!(lines[3], "  Chat · Bob                       14:32");
    assert_eq!(lines[5], "  Chat · Bob                  2025-03-01");
}

#[test]
fn a_sender_that_is_the_chat_itself_is_not_repeated() {
    let state = finished(
        Vec::new(),
        Vec::new(),
        vec![message(
            "Rust News",
            "Rust News",
            at(2026, 11, 12, 9, 5),
            "post",
        )],
    );

    assert_eq!(
        texts(&state, 3)[3],
        "  Rust News                       12 Nov"
    );
}

#[test]
fn a_long_message_takes_one_cut_line_whatever_its_line_breaks() {
    let state = finished(
        Vec::new(),
        Vec::new(),
        vec![message(
            "Chat",
            "Bob",
            at(2026, 11, 12, 9, 5),
            "first line\nsecond line that goes on and on and on",
        )],
    );

    let lines = texts(&state, 3);

    assert_eq!(lines[4], "    first line second line that goes ...");
    assert_eq!(lines[4].width(), WIDTH);
}

#[test]
fn a_long_chat_title_leaves_room_for_the_date() {
    let state = finished(
        Vec::new(),
        Vec::new(),
        vec![message(
            "A chat with a very long title indeed, really",
            "Bob",
            at(2026, 11, 12, 9, 5),
            "text",
        )],
    );

    assert_eq!(
        texts(&state, 3)[3],
        "  A chat with a very long tit...  12 Nov"
    );
}

#[test]
fn the_selected_row_is_highlighted_across_both_lines_of_a_message() {
    let mut state = finished(
        vec![chat(1, "Rust chat", None)],
        Vec::new(),
        vec![message("Rust chat", "Bob", at(2026, 11, 12, 9, 5), "text")],
    );
    assert_eq!(highlighted(&state, 6), ["  Rust chat"]);

    state.select_next();

    assert_eq!(
        highlighted(&state, 6),
        ["  Rust chat · Bob                 12 Nov", "    text"]
    );
}

#[test]
fn the_list_scrolls_to_keep_the_selected_row_in_view() {
    let chats = (1..=8)
        .map(|id| chat(id, &format!("Chat {id}"), None))
        .collect();
    let mut state = finished(chats, Vec::new(), Vec::new());
    assert_eq!(texts(&state, 3)[2..5], ["Chats", "  Chat 1", "  Chat 2"]);

    (0..7).for_each(|_| state.select_next());

    assert_eq!(texts(&state, 3)[2..5], ["  Chat 6", "  Chat 7", "  Chat 8"]);
    assert_eq!(highlighted(&state, 3), ["  Chat 8"]);
}

#[test]
fn searching_is_shown_under_the_results_until_every_request_answers() {
    let mut state = GlobalSearchState::default();
    state.insert_char('r', Instant::now());
    state.apply_chats("r", ChatScope::Known, vec![chat(1, "Rust", None)]);

    let lines = texts(&state, 3);

    assert_eq!(lines[3], "  Rust");
    assert_eq!(lines[5], SEARCHING);
}

#[test]
fn nothing_found_is_shown_once_every_request_found_nothing() {
    let state = finished(Vec::new(), Vec::new(), Vec::new());

    assert_eq!(texts(&state, 3)[5], NOTHING_FOUND);
}

#[test]
fn a_finished_search_with_results_says_nothing_more() {
    let state = finished(vec![chat(1, "Rust", None)], Vec::new(), Vec::new());

    assert_eq!(texts(&state, 3)[5], "");
}
