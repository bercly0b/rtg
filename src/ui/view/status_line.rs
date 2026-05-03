use std::sync::OnceLock;
use std::time::Instant;

use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use crate::domain::shell_state::ShellState;
use crate::ui::styles;

use super::text_utils::truncate_to_display_width;

/// Milliseconds spent on each frame of the Connecting dot animation.
const CONNECTING_FRAME_MS: u128 = 300;

pub(super) fn status_line<'a>(state: &'a ShellState, width: usize) -> Line<'a> {
    use crate::domain::events::ConnectivityStatus;

    let (dot_style, label) = match state.connectivity_status() {
        ConnectivityStatus::Connected => (styles::connectivity_dot_connected(), "Connected"),
        ConnectivityStatus::Connecting => (
            styles::connectivity_dot_connecting(),
            connecting_label_for_phase(current_connecting_phase()),
        ),
        ConnectivityStatus::Updating => (styles::connectivity_dot_updating(), "Updating"),
        ConnectivityStatus::Disconnected => {
            (styles::connectivity_dot_disconnected(), "Disconnected")
        }
    };

    let dot_text = " \u{25CF} ";
    let separator = "  ";
    let right_text = "? for help ";
    let fixed_width = dot_text.width() + label.width() + right_text.width();

    let mut spans: Vec<Span<'a>> = vec![
        Span::styled(dot_text, dot_style),
        Span::styled(label, styles::connectivity_label_style()),
    ];

    if let Some(notification) = state.active_notification() {
        let budget = width.saturating_sub(fixed_width + separator.width());
        if budget > 0 {
            let (truncated, _) = truncate_to_display_width(notification, budget);
            spans.push(Span::styled(separator, styles::status_bar_style()));
            spans.push(Span::styled(truncated, styles::notification_style()));
        }
    }

    let left_width: usize = spans.iter().map(|s| s.content.width()).sum();
    let padding = width.saturating_sub(left_width + right_text.width());

    spans.push(Span::styled(
        " ".repeat(padding),
        styles::status_bar_style(),
    ));
    spans.push(Span::styled(right_text, styles::help_hint_style()));

    Line::from(spans)
}

/// Returns the Connecting label for the given animation phase.
///
/// Cycles through 1, 2, 3, 2 trailing dots so the dot count visually
/// expands and contracts: `.` → `..` → `...` → `..` → (back to `.`).
pub(super) fn connecting_label_for_phase(phase: u8) -> &'static str {
    match phase % 4 {
        0 => "Connecting.",
        1 => "Connecting..",
        2 => "Connecting...",
        _ => "Connecting..",
    }
}

fn current_connecting_phase() -> u8 {
    static START: OnceLock<Instant> = OnceLock::new();
    let start = START.get_or_init(Instant::now);
    ((start.elapsed().as_millis() / CONNECTING_FRAME_MS) % 4) as u8
}

pub(super) fn compute_input_height(text: &str, available_width: u16) -> u16 {
    use unicode_width::UnicodeWidthStr;

    let effective_width = available_width.saturating_sub(2 + 2) as usize;
    if effective_width == 0 || text.is_empty() {
        return 1;
    }

    let text_width = text.width();
    let lines = text_width.div_ceil(effective_width);
    (lines as u16).clamp(1, 20)
}
