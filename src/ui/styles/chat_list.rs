//! Style definitions for the chat list panel.

use ratatui::style::{Color, Modifier, Style};

/// Style for chat name (bold, bright).
pub fn chat_name_style() -> Style {
    Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

/// Style for message preview text (dimmed).
pub fn chat_preview_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

/// Style for unread count badge (green).
pub fn unread_count_style() -> Style {
    Style::default().fg(Color::Green)
}

/// Style for unread reaction badge in the chat list (pink heart icon).
pub fn reaction_badge_style() -> Style {
    Style::default().fg(Color::LightRed)
}

/// Style for section headers like "-- Pinned --".
pub fn section_header_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

/// Style for timestamp column.
pub fn timestamp_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

/// Style for separator between timestamp and content.
pub fn separator_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

/// Style for online status indicator (green dot).
pub fn online_indicator_style() -> Style {
    Style::default().fg(Color::Green)
}

/// Style for unread outgoing message indicator (dot, dimmed).
pub fn outgoing_unread_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

/// Style for read outgoing message indicator (checkmark, green).
pub fn outgoing_read_style() -> Style {
    Style::default().fg(Color::Green)
}

/// Style for sender name prefix in group chats (cyan, slightly dimmed).
pub fn group_sender_style() -> Style {
    Style::default().fg(Color::Cyan)
}
