use crate::domain::shell_state::ShellState;

use super::super::messages_panel;

fn title_to_string(line: &ratatui::text::Line<'_>) -> String {
    line.spans.iter().map(|s| s.content.as_ref()).collect()
}

#[test]
fn open_chat_title_empty_when_no_chat_selected() {
    let state = ShellState::default();

    let title = messages_panel::open_chat_title(state.open_chat(), true);
    let text = title_to_string(&title);

    assert_eq!(text, "Messages");
}

#[test]
fn open_chat_title_includes_chat_name_when_open() {
    let mut state = ShellState::default();
    state.open_chat_mut().set_loading(
        1,
        "General".to_owned(),
        crate::domain::chat::ChatType::Private,
    );

    let title = messages_panel::open_chat_title(state.open_chat(), true);
    let text = title_to_string(&title);

    assert_eq!(text, "General");
}

fn make_message() -> crate::domain::message::Message {
    crate::domain::message::Message {
        id: 1,
        sender_name: "User".to_owned(),
        text: "msg".to_owned(),
        timestamp_ms: 1000,
        is_outgoing: false,
        media: crate::domain::message::MessageMedia::None,
        status: crate::domain::message::MessageStatus::Delivered,
        file_info: None,
        call_info: None,
        reply_to: None,
        forward_info: None,
        reaction_count: 0,
        links: Vec::new(),
        is_edited: false,
    }
}

#[test]
fn open_chat_title_shows_updating_when_refreshing() {
    let mut state = ShellState::default();
    state.open_chat_mut().set_loading(
        1,
        "General".to_owned(),
        crate::domain::chat::ChatType::Private,
    );
    state.open_chat_mut().set_ready(vec![make_message()]);
    state.open_chat_mut().set_refreshing(true);

    let title = messages_panel::open_chat_title(state.open_chat(), true);
    let text = title_to_string(&title);

    assert!(
        text.contains("updating..."),
        "expected 'updating...' in title, got: {text}"
    );
    assert!(text.contains("General"));
}

#[test]
fn open_chat_title_shows_subtitle_when_set() {
    let mut state = ShellState::default();
    state.open_chat_mut().set_loading(
        1,
        "Alice".to_owned(),
        crate::domain::chat::ChatType::Private,
    );
    state.open_chat_mut().set_ready(vec![make_message()]);
    state
        .open_chat_mut()
        .set_chat_subtitle(crate::domain::chat_subtitle::ChatSubtitle::Online);

    let title = messages_panel::open_chat_title(state.open_chat(), true);
    let text = title_to_string(&title);

    assert!(
        text.contains("online"),
        "expected 'online' in title, got: {text}"
    );
    assert!(text.contains("Alice"));
}

#[test]
fn open_chat_title_no_subtitle_when_not_set() {
    let mut state = ShellState::default();
    state.open_chat_mut().set_loading(
        1,
        "General".to_owned(),
        crate::domain::chat::ChatType::Private,
    );
    state.open_chat_mut().set_ready(vec![make_message()]);

    let title = messages_panel::open_chat_title(state.open_chat(), true);
    let text = title_to_string(&title);

    assert!(
        !text.contains("updating..."),
        "no updating indicator expected, got: {text}"
    );
}

#[test]
fn open_chat_title_shows_typing_in_private_chat() {
    let mut state = ShellState::default();
    state.open_chat_mut().set_loading(
        1,
        "Alice".to_owned(),
        crate::domain::chat::ChatType::Private,
    );
    state.open_chat_mut().set_ready(vec![make_message()]);
    state
        .open_chat_mut()
        .typing_state_mut()
        .add_action(42, "Alice".into(), "typing".into());

    let title = messages_panel::open_chat_title(state.open_chat(), true);
    let text = title_to_string(&title);

    assert!(
        text.contains("typing..."),
        "expected 'typing...' in title, got: {text}"
    );
    assert!(text.contains("Alice"));

    let typing_span = &title.spans[1];
    assert_eq!(
        typing_span.style.fg,
        Some(ratatui::style::Color::Blue),
        "typing indicator should be blue"
    );
}

#[test]
fn open_chat_title_shows_typing_in_group_chat() {
    let mut state = ShellState::default();
    state
        .open_chat_mut()
        .set_loading(1, "Team".to_owned(), crate::domain::chat::ChatType::Group);
    state.open_chat_mut().set_ready(vec![make_message()]);
    state
        .open_chat_mut()
        .typing_state_mut()
        .add_action(42, "Bob".into(), "typing".into());

    let title = messages_panel::open_chat_title(state.open_chat(), true);
    let text = title_to_string(&title);

    assert!(
        text.contains("Bob is typing..."),
        "expected 'Bob is typing...' in title, got: {text}"
    );
}

#[test]
fn open_chat_title_typing_overrides_subtitle() {
    let mut state = ShellState::default();
    state.open_chat_mut().set_loading(
        1,
        "Alice".to_owned(),
        crate::domain::chat::ChatType::Private,
    );
    state.open_chat_mut().set_ready(vec![make_message()]);
    state
        .open_chat_mut()
        .set_chat_subtitle(crate::domain::chat_subtitle::ChatSubtitle::Online);
    state
        .open_chat_mut()
        .typing_state_mut()
        .add_action(42, "Alice".into(), "typing".into());

    let title = messages_panel::open_chat_title(state.open_chat(), true);
    let text = title_to_string(&title);

    assert!(
        text.contains("typing..."),
        "typing should override subtitle, got: {text}"
    );
    assert!(
        !text.contains("online"),
        "subtitle should not appear when typing, got: {text}"
    );
}
