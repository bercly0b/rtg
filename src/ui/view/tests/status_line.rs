use unicode_width::UnicodeWidthStr;

use crate::domain::events::ConnectivityStatus;
use crate::domain::shell_state::ShellState;

use super::{super::status_line, line_to_string};

const STATUS_WIDTH: usize = 80;

#[test]
fn status_line_renders_connected_label() {
    let mut state = ShellState::default();
    state.set_connectivity_status(ConnectivityStatus::Connected);

    let line = status_line::status_line(&state, STATUS_WIDTH);
    let text = line_to_string(&line);

    assert!(text.contains("Connected"));
    assert!(text.contains("\u{25CF}"));
}

#[test]
fn status_line_renders_disconnected_label() {
    let mut state = ShellState::default();
    state.set_connectivity_status(ConnectivityStatus::Disconnected);

    let line = status_line::status_line(&state, STATUS_WIDTH);
    let text = line_to_string(&line);

    assert!(text.contains("Disconnected"));
}

#[test]
fn status_line_contains_help_hint() {
    let state = ShellState::default();

    let line = status_line::status_line(&state, STATUS_WIDTH);
    let text = line_to_string(&line);

    assert!(text.contains("? for help"));
}

#[test]
fn status_line_shows_notification_when_set() {
    let mut state = ShellState::default();
    state.set_notification("Copied to clipboard");

    let line = status_line::status_line(&state, STATUS_WIDTH);
    let text = line_to_string(&line);

    assert!(text.contains("Copied to clipboard"));
}

#[test]
fn status_line_without_notification_has_no_extra_text() {
    let state = ShellState::default();

    let line = status_line::status_line(&state, STATUS_WIDTH);
    let text = line_to_string(&line);

    assert!(!text.contains("Copied"));
    assert!(!text.contains("deleted"));
}

#[test]
fn status_line_hides_expired_notification() {
    let mut state = ShellState::default();
    let expired = std::time::Instant::now() - std::time::Duration::from_secs(5);
    state.set_notification_at("Old message", expired);

    let line = status_line::status_line(&state, STATUS_WIDTH);
    let text = line_to_string(&line);

    assert!(!text.contains("Old message"));
}

#[test]
fn status_line_truncates_long_notification() {
    let mut state = ShellState::default();
    state.set_notification("A".repeat(200));

    let line = status_line::status_line(&state, 40);
    let text = line_to_string(&line);

    assert!(text.contains("? for help"));
    assert!(text.width() <= 40);
}

// -- compute_input_height tests --

#[test]
fn compute_input_height_empty_text_returns_one() {
    assert_eq!(status_line::compute_input_height("", 80), 1);
}

#[test]
fn compute_input_height_short_text_returns_one() {
    assert_eq!(status_line::compute_input_height("Hello", 80), 1);
}

#[test]
fn compute_input_height_long_text_wraps() {
    let text = "a".repeat(32);
    assert_eq!(status_line::compute_input_height(&text, 20), 2);
}

#[test]
fn compute_input_height_capped_at_twenty() {
    let text = "a".repeat(5000);
    assert_eq!(status_line::compute_input_height(&text, 20), 20);
}

#[test]
fn compute_input_height_zero_width_returns_one() {
    assert_eq!(status_line::compute_input_height("hello", 0), 1);
}

#[test]
fn compute_input_height_narrow_width_returns_one() {
    assert_eq!(status_line::compute_input_height("hello", 4), 1);
}
