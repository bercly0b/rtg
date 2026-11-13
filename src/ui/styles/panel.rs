//! Style definitions for panels, status bar, and connectivity indicators.
//!
//! All panels use the terminal's default background (no bg override) so that
//! the TUI inherits whatever color scheme the user has configured. Foreground
//! accents use ANSI colors (0-15), which are controlled by the user's terminal
//! theme.
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

/// Green dot for "Connected" status.
pub fn connectivity_dot_connected() -> Style {
    Style::default().fg(Color::Green)
}

/// Yellow dot for "Connecting" status.
pub fn connectivity_dot_connecting() -> Style {
    Style::default().fg(Color::Yellow)
}

/// Cyan dot for "Updating" status — a working connection that is still
/// catching up on missed updates from the server.
pub fn connectivity_dot_updating() -> Style {
    Style::default().fg(Color::Cyan)
}

/// Red dot for "Disconnected" status.
pub fn connectivity_dot_disconnected() -> Style {
    Style::default().fg(Color::Red)
}

/// Connectivity label text in the status bar.
pub fn connectivity_label_style() -> Style {
    Style::default()
}

/// Transient notification text in the status bar.
pub fn notification_style() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::ITALIC)
}

/// Available update notice in the status bar: visible, but calmer than a
/// notification.
pub fn update_notice_style() -> Style {
    Style::default().fg(Color::Cyan)
}

/// Subtle help hint ("? for help") in the status bar.
pub fn help_hint_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

/// Style for typing indicator in the chat header.
pub fn typing_style() -> Style {
    Style::default()
        .fg(Color::Blue)
        .add_modifier(Modifier::BOLD)
}
