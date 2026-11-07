use std::time::{Duration, Instant};

use unicode_width::UnicodeWidthStr;

use crate::domain::download_speed::MIN_TRACKED_FILE_SIZE;
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
fn status_line_renders_updating_label() {
    let mut state = ShellState::default();
    state.set_connectivity_status(ConnectivityStatus::Updating);

    let line = status_line::status_line(&state, STATUS_WIDTH);
    let text = line_to_string(&line);

    assert!(text.contains("Updating"));
}

#[test]
fn status_line_renders_connecting_label() {
    let mut state = ShellState::default();
    state.set_connectivity_status(ConnectivityStatus::Connecting);

    let line = status_line::status_line(&state, STATUS_WIDTH);
    let text = line_to_string(&line);

    assert!(text.contains("Connecting"));
}

#[test]
fn connecting_label_cycles_through_dot_count_pattern() {
    assert_eq!(status_line::connecting_label_for_phase(0), "Connecting.");
    assert_eq!(status_line::connecting_label_for_phase(1), "Connecting..");
    assert_eq!(status_line::connecting_label_for_phase(2), "Connecting...");
    assert_eq!(status_line::connecting_label_for_phase(3), "Connecting..");
    // Cycle wraps so the next frame returns to a single dot.
    assert_eq!(status_line::connecting_label_for_phase(4), "Connecting.");
    assert_eq!(status_line::connecting_label_for_phase(7), "Connecting..");
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

fn state_downloading(bytes_per_second: u64) -> ShellState {
    let mut state = ShellState::default();
    state.set_connectivity_status(ConnectivityStatus::Connected);
    let start = Instant::now() - Duration::from_secs(2);
    let meter = state.download_speed_mut();
    meter.record_progress(1, MIN_TRACKED_FILE_SIZE, 0, start);
    meter.record_progress(
        1,
        MIN_TRACKED_FILE_SIZE,
        bytes_per_second,
        start + Duration::from_secs(1),
    );
    state
}

#[test]
fn status_line_shows_download_speed_after_connectivity_label() {
    let state = state_downloading(1_572_864);

    let line = status_line::status_line(&state, STATUS_WIDTH);
    let text = line_to_string(&line);

    assert!(text.contains("Connected  \u{2193} 1.5 MB/s"), "{text}");
}

#[test]
fn status_line_without_download_has_no_speed() {
    let mut state = ShellState::default();
    state.set_connectivity_status(ConnectivityStatus::Connected);

    let line = status_line::status_line(&state, STATUS_WIDTH);
    let text = line_to_string(&line);

    assert!(!text.contains("/s"));
}

#[test]
fn status_line_shows_download_speed_with_notification() {
    let mut state = state_downloading(1_572_864);
    state.set_notification("A".repeat(200));

    let line = status_line::status_line(&state, STATUS_WIDTH);
    let text = line_to_string(&line);

    assert!(text.contains("MB/s  AAA"), "{text}");
    assert!(text.ends_with("? for help "));
    assert_eq!(text.width(), STATUS_WIDTH);
}

#[test]
fn download_speed_below_one_kilobyte_is_shown_in_bytes() {
    assert_eq!(status_line::format_download_speed(0), "\u{2193} 0 B/s");
    assert_eq!(
        status_line::format_download_speed(1_023),
        "\u{2193} 1023 B/s"
    );
}

#[test]
fn download_speed_is_shown_in_binary_kilobytes_and_megabytes() {
    assert_eq!(
        status_line::format_download_speed(1_024),
        "\u{2193} 1.0 KB/s"
    );
    assert_eq!(
        status_line::format_download_speed(512 * 1_024),
        "\u{2193} 512 KB/s"
    );
    assert_eq!(
        status_line::format_download_speed(1_048_576),
        "\u{2193} 1.0 MB/s"
    );
    assert_eq!(
        status_line::format_download_speed(8 * 1_048_576),
        "\u{2193} 8.0 MB/s"
    );
}

#[test]
fn download_speed_has_one_truncated_decimal_up_to_99() {
    assert_eq!(
        status_line::format_download_speed(7_000_000),
        "\u{2193} 6.6 MB/s"
    );
    assert_eq!(
        status_line::format_download_speed(1_048_575),
        "\u{2193} 1023 KB/s"
    );
    assert_eq!(
        status_line::format_download_speed(100 * 1_024 - 1),
        "\u{2193} 99.9 KB/s"
    );
    assert_eq!(
        status_line::format_download_speed(150 * 1_048_576 + 900_000),
        "\u{2193} 150 MB/s"
    );
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
fn compute_input_height_counts_explicit_newlines() {
    assert_eq!(status_line::compute_input_height("a\nb\nc", 80), 3);
}

#[test]
fn compute_input_height_counts_trailing_newline() {
    assert_eq!(status_line::compute_input_height("a\n", 80), 2);
}

#[test]
fn compute_input_height_combines_newlines_and_wrapping() {
    let text = format!("{}\nb", "a".repeat(32));
    assert_eq!(status_line::compute_input_height(&text, 20), 3);
}

#[test]
fn compute_input_height_zero_width_returns_one() {
    assert_eq!(status_line::compute_input_height("hello", 0), 1);
}

#[test]
fn compute_input_height_narrow_width_returns_one() {
    assert_eq!(status_line::compute_input_height("hello", 4), 1);
}
