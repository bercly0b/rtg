use ratatui::{backend::TestBackend, Terminal};

use super::{super::render, chat};
use crate::domain::{
    chat::ChatType,
    layout::LayoutColumns,
    message::{Message, MessageMedia, MessageStatus},
    shell_state::{ActivePane, ShellState},
};

/// Longer than the chat list panel of a two-column layout is wide at 80 columns.
const LIST_CHAT: &str = "A chat with quite a long title";
const MESSAGE: &str = "hello-from-bob";
const VERTICAL_SEPARATOR: &str = "\u{2502}";

fn message() -> Message {
    Message {
        id: 1,
        sender_name: "Bob".to_owned(),
        text: MESSAGE.to_owned(),
        timestamp_ms: 1000,
        is_outgoing: false,
        media: MessageMedia::None,
        status: MessageStatus::Delivered,
        file_info: None,
        call_info: None,
        reply_to: None,
        forward_info: None,
        reaction_count: 0,
        links: Vec::new(),
        is_edited: false,
        sender: None,
        is_service: false,
    }
}

fn state_with_open_chat(active_pane: ActivePane) -> ShellState {
    let mut state = ShellState::with_initial_chat_list(vec![chat(1, LIST_CHAT, 0, None)]);
    state
        .open_chat_mut()
        .set_loading(2, "Bob".to_owned(), ChatType::Private);
    state.open_chat_mut().set_ready(vec![message()]);
    state.set_active_pane(active_pane);
    state
}

fn render_at(state: &mut ShellState, width: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, 20)).unwrap();
    terminal.draw(|frame| render(frame, state, &[])).unwrap();
    let buffer = terminal.backend().buffer();
    buffer.content().iter().map(|cell| cell.symbol()).collect()
}

#[test]
fn narrow_window_with_focus_on_the_chat_list_shows_only_the_chat_list_panel() {
    let mut state = state_with_open_chat(ActivePane::ChatList);

    let text = render_at(&mut state, 80);

    assert!(text.contains(LIST_CHAT), "{text}");
    assert!(!text.contains(MESSAGE), "{text}");
    assert!(!text.contains(VERTICAL_SEPARATOR), "{text}");
}

#[test]
fn narrow_window_with_focus_on_messages_shows_only_the_messages_panel() {
    let mut state = state_with_open_chat(ActivePane::Messages);

    let text = render_at(&mut state, 80);

    assert!(text.contains(MESSAGE), "{text}");
    assert!(!text.contains("A chat"), "{text}");
    assert!(!text.contains(VERTICAL_SEPARATOR), "{text}");
}

#[test]
fn narrow_window_with_focus_on_the_message_input_keeps_the_messages_panel() {
    let mut state = state_with_open_chat(ActivePane::MessageInput);
    state.message_input_mut().insert_char('x');

    let text = render_at(&mut state, 80);

    assert!(text.contains(MESSAGE), "{text}");
    assert!(!text.contains("A chat"), "{text}");
}

#[test]
fn window_of_100_columns_shows_both_panels_side_by_side() {
    let mut state = state_with_open_chat(ActivePane::Messages);

    let text = render_at(&mut state, 100);

    assert!(text.contains("A chat"), "{text}");
    assert!(text.contains(MESSAGE), "{text}");
    assert!(text.contains(VERTICAL_SEPARATOR), "{text}");
}

#[test]
fn single_column_layout_is_not_limited_in_a_wide_window() {
    let mut state =
        state_with_open_chat(ActivePane::Messages).with_layout_columns(LayoutColumns::One);

    let text = render_at(&mut state, 200);

    assert!(text.contains(MESSAGE), "{text}");
    assert!(!text.contains("A chat"), "{text}");
    assert!(text.contains(&"\u{2500}".repeat(200)), "{text}");
}

#[test]
fn toggling_the_layout_in_a_narrow_window_brings_both_panels_back() {
    let mut state = state_with_open_chat(ActivePane::Messages);
    render_at(&mut state, 80);

    state.toggle_layout();
    let text = render_at(&mut state, 80);

    assert!(text.contains("A chat"), "{text}");
    assert!(text.contains(MESSAGE), "{text}");
}

#[test]
fn narrow_window_with_focus_on_the_topic_list_hides_the_topic_that_stays_open() {
    let mut state = state_with_open_chat(ActivePane::ChatList);
    state.enter_forum(2, "Some forum");

    let text = render_at(&mut state, 80);

    assert!(text.contains("Some forum"), "{text}");
    assert!(!text.contains(MESSAGE), "{text}");
    assert!(state.open_chat().is_open());
}
