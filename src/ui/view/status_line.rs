use std::sync::OnceLock;
use std::time::Instant;

use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use crate::domain::shell_state::ShellState;
use crate::domain::update_check::{AvailableUpdate, SelfUpdateProgress};
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

    let download_speed = state
        .download_speed()
        .bytes_per_second(Instant::now())
        .map(format_download_speed);

    let dot_text = " \u{25CF} ";
    let separator = "  ";
    let right_text = "? for help ";
    let speed_width = download_speed
        .as_ref()
        .map_or(0, |speed| separator.width() + speed.width());
    let fixed_width = dot_text.width() + label.width() + speed_width + right_text.width();

    let mut spans: Vec<Span<'a>> = vec![
        Span::styled(dot_text, dot_style),
        Span::styled(label, styles::connectivity_label_style()),
    ];

    if let Some(speed) = download_speed {
        spans.push(Span::raw(separator));
        spans.push(Span::styled(speed, styles::connectivity_label_style()));
    }

    if let Some(notification) = state.active_notification() {
        let budget = width.saturating_sub(fixed_width + separator.width());
        if budget > 0 {
            let (truncated, _) = truncate_to_display_width(notification, budget);
            spans.push(Span::raw(separator));
            spans.push(Span::styled(truncated, styles::notification_style()));
        }
    }

    let left_width: usize = spans.iter().map(|s| s.content.width()).sum();

    // A notification is short-lived, so it may take the room the update
    // notice needs; the notice comes back once the notification expires.
    let update_notice = state
        .self_update()
        .map(format_self_update_notice)
        .or_else(|| state.available_update().map(format_update_notice))
        .filter(|notice| {
            left_width + notice.width() + separator.width() + right_text.width() <= width
        });
    let notice_width = update_notice
        .as_ref()
        .map_or(0, |notice| notice.width() + separator.width());
    let padding = width.saturating_sub(left_width + notice_width + right_text.width());

    spans.push(Span::raw(" ".repeat(padding)));
    if let Some(notice) = update_notice {
        spans.push(Span::styled(notice, styles::update_notice_style()));
        spans.push(Span::raw(separator));
    }
    spans.push(Span::styled(right_text, styles::help_hint_style()));

    Line::from(spans)
}

pub(super) fn format_update_notice(update: &AvailableUpdate) -> String {
    format!(
        "v{} available: {}",
        update.version,
        update.channel.update_hint()
    )
}

pub(super) fn format_self_update_notice(progress: &SelfUpdateProgress) -> String {
    match progress {
        SelfUpdateProgress::Running => "Updating rtg...".to_owned(),
        SelfUpdateProgress::Installed { version } => format!("Updated to v{version}, restart rtg"),
    }
}

/// Formats a download speed in bytes per second with binary (1024) multiples,
/// the way tmux-net-speed and similar system monitors show it, so the numbers
/// can be compared side by side. The decimal is truncated rather than rounded
/// so a value never rounds up past its unit (e.g. to `100.0`).
pub(super) fn format_download_speed(bytes_per_second: u64) -> String {
    const UNITS: [&str; 3] = ["B", "KB", "MB"];

    let mut unit = 0;
    let mut divisor = 1;
    while bytes_per_second / divisor >= 1_024 && unit < UNITS.len() - 1 {
        divisor *= 1_024;
        unit += 1;
    }

    let whole = bytes_per_second / divisor;
    if unit == 0 || whole > 99 {
        format!("\u{2193} {whole} {}/s", UNITS[unit])
    } else {
        let tenth = bytes_per_second % divisor * 10 / divisor;
        format!("\u{2193} {whole}.{tenth} {}/s", UNITS[unit])
    }
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

    let lines: usize = text
        .split('\n')
        .map(|line| line.width().div_ceil(effective_width).max(1))
        .sum();
    (lines as u16).clamp(1, 20)
}
