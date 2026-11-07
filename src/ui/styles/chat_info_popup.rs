//! Style definitions for the chat info popup overlay.

use ratatui::style::{Color, Modifier, Style};

/// Border style for the chat info popup overlay.
pub fn chat_info_popup_border_style() -> Style {
    Style::default().fg(Color::White)
}

/// Style for field labels in the chat info popup (e.g. "Status:", "Description").
pub fn chat_info_popup_label_style() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD)
}

/// Style for field values in the chat info popup.
pub fn chat_info_popup_value_style() -> Style {
    Style::default().fg(Color::White)
}

/// Style for the chat title heading the chat info popup.
pub fn chat_info_popup_title_style() -> Style {
    Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

/// Style for the status line under the chat title.
pub fn chat_info_popup_status_style() -> Style {
    Style::default().fg(Color::Gray)
}

/// Style for row labels inside a chat info block.
pub fn chat_info_popup_field_style() -> Style {
    Style::default().fg(Color::Gray)
}

/// Style for `—` and `hidden` in place of a missing value.
pub fn chat_info_popup_placeholder_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

/// Style for values that warn the user, such as a scam badge.
pub fn chat_info_popup_warning_style() -> Style {
    Style::default().fg(Color::Red)
}
