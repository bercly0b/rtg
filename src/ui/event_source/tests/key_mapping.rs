use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::domain::events::{AppEvent, KeyInput};

use super::super::map_key_event;

#[test]
fn q_produces_input_key_not_quit() {
    let event = map_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
    assert_eq!(event, Some(AppEvent::InputKey(KeyInput::new("q", false))));
}

#[test]
fn ctrl_c_produces_quit_requested() {
    let event = map_key_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
    assert_eq!(event, Some(AppEvent::QuitRequested));
}

#[test]
fn question_mark_produces_input_key() {
    let event = map_key_event(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
    assert_eq!(event, Some(AppEvent::InputKey(KeyInput::new("?", false))));
}

#[test]
fn ignores_release_events() {
    let mut event = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
    event.kind = KeyEventKind::Release;
    assert_eq!(map_key_event(event), None);
}

#[test]
fn special_keys_produce_named_input() {
    let cases = vec![
        (KeyCode::Enter, "enter"),
        (KeyCode::Esc, "esc"),
        (KeyCode::Backspace, "backspace"),
        (KeyCode::Delete, "delete"),
        (KeyCode::Left, "left"),
        (KeyCode::Right, "right"),
        (KeyCode::Home, "home"),
        (KeyCode::End, "end"),
    ];
    for (code, expected) in cases {
        let event = map_key_event(KeyEvent::new(code, KeyModifiers::NONE));
        assert_eq!(
            event,
            Some(AppEvent::InputKey(KeyInput::new(expected, false))),
            "failed for key code {:?}",
            code
        );
    }
}

#[test]
fn ctrl_o_produces_ctrl_input_key() {
    let event = map_key_event(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL));
    assert_eq!(event, Some(AppEvent::InputKey(KeyInput::new("o", true))));
}

#[test]
fn unknown_special_key_returns_none() {
    let event = map_key_event(KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE));
    assert_eq!(event, None);
}
