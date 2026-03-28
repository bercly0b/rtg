//! Style definitions for the message input area.

use ratatui::style::{Color, Style};

/// Style for the input prompt symbol (>).
pub fn input_prompt_style() -> Style {
    Style::default().fg(Color::Green)
}

/// Style for the input text being typed.
pub fn input_text_style() -> Style {
    Style::default().fg(Color::White)
}

/// Style for the placeholder text in unfocused empty input.
pub fn input_placeholder_style() -> Style {
    Style::default().fg(Color::DarkGray)
}
