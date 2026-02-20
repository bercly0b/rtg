//! Style definitions for the UI components.

use ratatui::style::{Color, Modifier, Style};

// =============================================================================
// Chat list styles
// =============================================================================

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

// =============================================================================
// Message list styles
// =============================================================================

/// Style for message sender name (white, bold).
pub fn message_sender_style() -> Style {
    Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

/// Style for message time in the messages panel.
pub fn message_time_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

/// Style for message text content.
pub fn message_text_style() -> Style {
    Style::default().fg(Color::White)
}

/// Style for media type indicators like [Photo], [Voice].
pub fn message_media_style() -> Style {
    Style::default().fg(Color::Cyan)
}

/// Style for date separator line.
pub fn date_separator_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_name_style_is_bold_white() {
        let style = chat_name_style();
        assert_eq!(style.fg, Some(Color::White));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn unread_count_style_is_green() {
        let style = unread_count_style();
        assert_eq!(style.fg, Some(Color::Green));
    }

    #[test]
    fn message_sender_style_is_bold_white() {
        let style = message_sender_style();
        assert_eq!(style.fg, Some(Color::White));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn message_media_style_is_cyan() {
        let style = message_media_style();
        assert_eq!(style.fg, Some(Color::Cyan));
    }

    #[test]
    fn date_separator_style_is_dark_gray() {
        let style = date_separator_style();
        assert_eq!(style.fg, Some(Color::DarkGray));
    }
}
