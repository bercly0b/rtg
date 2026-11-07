//! The header, the loading and error states, and the popup frame.

use super::*;
use crate::domain::chat_info_state::{ChannelInfo, GroupInfo, GroupKind, Joining};

fn group_with_description(description: String) -> ChatInfoKind {
    ChatInfoKind::Group(GroupInfo {
        link: None,
        kind: Some(GroupKind::BasicGroup),
        admins: None,
        linked_channel: None,
        slow_mode_delay: None,
        joining: Some(Joining::InviteOnly),
        history_visible: None,
        has_protected_content: false,
        invite_link: None,
        badges: vec![],
        description: Some(description),
        role: None,
        joined: None,
        location: None,
    })
}

fn rendered_popup_rect(state: &ChatInfoPopupState, width: u16, height: u16) -> Rect {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| render_chat_info_popup(frame, frame.area(), state))
        .unwrap();
    popup_utils::rendered_border_rect(terminal.backend().buffer()).unwrap()
}

#[test]
fn loading_state_shows_title_and_loading_text() {
    let state = ChatInfoPopupState::Loading {
        chat_id: 1,
        title: "Alice".into(),
    };

    assert_eq!(rendered_lines(&state), vec!["Alice", "Loading..."]);
}

#[test]
fn error_state_shows_title_and_error_text() {
    let state = ChatInfoPopupState::Error {
        title: "Alice".into(),
    };

    assert_eq!(
        rendered_lines(&state),
        vec!["Alice", "Failed to load chat info"]
    );
}

#[test]
fn header_is_the_title_and_status_line_without_a_name_row() {
    let state = loaded("3 members", group_with_description("About".into()));

    let lines = build_info_lines(&state, VALUE_WIDTH);

    assert_eq!(line_text(&lines[0]), "Title");
    assert_eq!(
        lines[0].spans[0].style,
        styles::chat_info_popup_title_style()
    );
    assert_eq!(line_text(&lines[1]), "3 members");
    assert!(!rendered_lines(&state)
        .iter()
        .any(|line| line.contains("Name")));
}

#[test]
fn loading_popup_is_sized_to_content() {
    let state = ChatInfoPopupState::Loading {
        chat_id: 1,
        title: "Alice".into(),
    };
    let rect = rendered_popup_rect(&state, 100, 40);
    // Minimum width; title and loading rows plus borders and padding.
    assert_eq!((rect.width, rect.height), (30, 6));
}

#[test]
fn long_description_wraps_within_the_popup_and_stays_visible() {
    let description = format!("{} width", "long description ".repeat(10));
    let state = loaded("5 members", group_with_description(description));

    let screen = rendered_screen(&state, 100, 60);
    let rect = rendered_popup_rect(&state, 100, 60);

    assert!(screen.contains("width"));
    assert!(rect.width <= 80, "popup is {} wide", rect.width);
}

#[test]
fn short_terminal_cuts_off_the_bottom() {
    let channel = ChannelInfo {
        link: None,
        discussion: None,
        signatures: None,
        has_protected_content: false,
        admins: None,
        badges: vec![],
        description: None,
        role: None,
        joined: None,
    };
    let state = loaded("1 subscriber", ChatInfoKind::Channel(channel));

    let screen = rendered_screen(&state, 100, 12);

    assert!(screen.contains("Title"));
    assert!(!screen.contains("Press q"));
}

#[test]
fn title_shows_only_chat_info_label() {
    let state = loaded("3 members", group_with_description("About".into()));
    let mut terminal = Terminal::new(TestBackend::new(100, 40)).unwrap();
    terminal
        .draw(|frame| render_chat_info_popup(frame, frame.area(), &state))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let rect = popup_utils::rendered_border_rect(buffer).unwrap();
    let top_border: String = (rect.x..rect.right())
        .map(|x| buffer[(x, rect.y)].symbol())
        .collect();

    assert!(top_border.contains(" Chat Info "));
    assert!(!top_border.contains("Title"));
}
