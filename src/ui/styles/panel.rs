//! Style definitions for panels, status bar, and connectivity indicators.
//!
//! All panels use the terminal's default background (no bg override) so that
//! the TUI inherits whatever color scheme the user has configured. Only the
//! status bar and the panel separator use explicit ANSI colors (0-15), which
//! are controlled by the user's terminal theme.
//!
//! Active panel is indicated by a green title, matching the existing green
//! accent used for unread badges, online indicators, and the input prompt.

use ratatui::style::{Color, Modifier, Style};

/// Style for the highlighted (selected) item in a list.
/// Uses a uniform background and foreground so the entire row looks consistent.
pub fn highlight_style() -> Style {
    Style::default().fg(Color::Black).bg(Color::Gray)
}

/// Style for the panel title when the panel is active.
pub fn active_title_style() -> Style {
    Style::default()
        .fg(Color::Green)
        .add_modifier(Modifier::BOLD)
}

/// Style for the panel title when the panel is inactive.
pub fn inactive_title_style() -> Style {
    Style::default()
        .fg(Color::DarkGray)
        .add_modifier(Modifier::BOLD)
}

/// Style for the vertical separator between panels.
pub fn panel_separator_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

/// Style for the status bar (ANSI Black bg, default fg).
/// Uses ANSI color 0 for background — controlled by the terminal theme.
pub fn status_bar_style() -> Style {
    Style::default().bg(Color::Black)
}

/// Green dot for "Connected" status.
pub fn connectivity_dot_connected() -> Style {
    Style::default().fg(Color::Green).bg(Color::Black)
}

/// Yellow dot for "Connecting" status.
pub fn connectivity_dot_connecting() -> Style {
    Style::default().fg(Color::Yellow).bg(Color::Black)
}

/// Red dot for "Disconnected" status.
pub fn connectivity_dot_disconnected() -> Style {
    Style::default().fg(Color::Red).bg(Color::Black)
}

/// Connectivity label text in the status bar.
pub fn connectivity_label_style() -> Style {
    Style::default().fg(Color::White).bg(Color::Black)
}

/// Transient notification text in the status bar.
pub fn notification_style() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .bg(Color::Black)
        .add_modifier(Modifier::ITALIC)
}

/// Subtle help hint ("? for help") in the status bar.
pub fn help_hint_style() -> Style {
    Style::default().fg(Color::DarkGray).bg(Color::Black)
}
