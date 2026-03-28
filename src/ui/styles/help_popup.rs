//! Style definitions for the help popup overlay.

use ratatui::style::{Color, Modifier, Style};

/// Border style for the help popup overlay.
pub fn help_popup_border_style() -> Style {
    Style::default().fg(Color::White)
}

/// Style for hotkey labels in the help popup (e.g. "j", "Enter / l").
pub fn help_popup_key_style() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD)
}

/// Style for action names in the help popup (e.g. "select_next_chat").
pub fn help_popup_action_style() -> Style {
    Style::default().fg(Color::White)
}

/// Style for the footer hint in the help popup.
pub fn help_popup_footer_style() -> Style {
    Style::default().fg(Color::DarkGray)
}
