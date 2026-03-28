use crate::domain::shell_state::ShellState;

use super::super::messages_panel;

#[test]
fn open_chat_title_empty_when_no_chat_selected() {
    let state = ShellState::default();

    let title = messages_panel::open_chat_title(state.open_chat());

    assert_eq!(title, "Messages");
}

#[test]
fn open_chat_title_includes_chat_name_when_open() {
    let mut state = ShellState::default();
    state.open_chat_mut().set_loading(
        1,
        "General".to_owned(),
        crate::domain::chat::ChatType::Private,
    );

    let title = messages_panel::open_chat_title(state.open_chat());

    assert_eq!(title, "General");
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

    let title = messages_panel::open_chat_title(state.open_chat());

    assert!(
        title.contains("updating..."),
        "expected 'updating...' in title, got: {title}"
    );
    assert!(title.contains("General"));
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

    let title = messages_panel::open_chat_title(state.open_chat());

    assert!(
        title.contains("online"),
        "expected 'online' in title, got: {title}"
    );
    assert!(title.contains("Alice"));
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

    let title = messages_panel::open_chat_title(state.open_chat());

    assert!(
        !title.contains("updating..."),
        "no updating indicator expected, got: {title}"
    );
}
