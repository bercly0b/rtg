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
