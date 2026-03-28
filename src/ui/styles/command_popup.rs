//! Style definitions for the command popup overlay.

use ratatui::style::{Color, Style};

/// Border style for the command popup overlay.
pub fn command_popup_border_style() -> Style {
    Style::default().fg(Color::White)
}

/// Style for command output text lines.
pub fn command_popup_output_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

/// Style for the footer hint in the command popup.
pub fn command_popup_footer_style() -> Style {
    Style::default().fg(Color::Yellow)
}

/// Style for error messages in the command popup (failed commands).
pub fn command_popup_error_style() -> Style {
    Style::default().fg(Color::Red)
}
