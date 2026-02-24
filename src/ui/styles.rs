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

// =============================================================================
// Panel background colors
// =============================================================================

/// Darker navy for the chat list sidebar.
const CHAT_LIST_BG: Color = Color::Rgb(28, 31, 48);
/// Slightly brighter sidebar when active.
const CHAT_LIST_ACTIVE_BG: Color = Color::Rgb(33, 36, 55);

/// Lighter dark for the messages area.
const MESSAGES_BG: Color = Color::Rgb(36, 40, 58);
/// Slightly brighter messages area when active.
const MESSAGES_ACTIVE_BG: Color = Color::Rgb(40, 44, 64);

/// Background for the input field.
const INPUT_BG: Color = Color::Rgb(32, 36, 52);
/// Brighter input field when focused — distinct from messages background.
const INPUT_ACTIVE_BG: Color = Color::Rgb(40, 44, 62);

/// Darkest shade for the status bar.
const STATUS_BAR_BG: Color = Color::Rgb(22, 25, 38);
/// Status bar foreground text.
const STATUS_BAR_FG: Color = Color::Rgb(140, 145, 170);

// =============================================================================
// Panel styles
//
// Panel backgrounds are applied via Block::style(). Child widget styles
// (text spans, list items) should NOT set bg() to allow the panel
// background to show through via ratatui's style inheritance.
// =============================================================================

/// Style for the chat list panel background.
pub fn chat_list_panel_style(is_active: bool) -> Style {
    let bg = if is_active {
        CHAT_LIST_ACTIVE_BG
    } else {
        CHAT_LIST_BG
    };
    Style::default().bg(bg)
}

/// Style for the messages panel background.
pub fn messages_panel_style(is_active: bool) -> Style {
    let bg = if is_active {
        MESSAGES_ACTIVE_BG
    } else {
        MESSAGES_BG
    };
    Style::default().bg(bg)
}

/// Style for the message input panel background.
pub fn input_panel_style(is_active: bool) -> Style {
    let bg = if is_active { INPUT_ACTIVE_BG } else { INPUT_BG };
    Style::default().bg(bg)
}

/// Style for the status bar.
pub fn status_bar_style() -> Style {
    Style::default().fg(STATUS_BAR_FG).bg(STATUS_BAR_BG)
}

// =============================================================================
// Message input styles
// =============================================================================

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

    #[test]
    fn input_prompt_style_is_green() {
        let style = input_prompt_style();
        assert_eq!(style.fg, Some(Color::Green));
    }

    #[test]
    fn input_text_style_is_white() {
        let style = input_text_style();
        assert_eq!(style.fg, Some(Color::White));
    }

    #[test]
    fn input_placeholder_style_is_dark_gray() {
        let style = input_placeholder_style();
        assert_eq!(style.fg, Some(Color::DarkGray));
    }

    #[test]
    fn online_indicator_style_is_green() {
        let style = online_indicator_style();
        assert_eq!(style.fg, Some(Color::Green));
    }

    #[test]
    fn outgoing_unread_style_is_dark_gray() {
        let style = outgoing_unread_style();
        assert_eq!(style.fg, Some(Color::DarkGray));
    }

    #[test]
    fn outgoing_read_style_is_green() {
        let style = outgoing_read_style();
        assert_eq!(style.fg, Some(Color::Green));
    }

    #[test]
    fn group_sender_style_is_cyan() {
        let style = group_sender_style();
        assert_eq!(style.fg, Some(Color::Cyan));
    }

    #[test]
    fn chat_list_panel_style_has_background() {
        let active = chat_list_panel_style(true);
        let inactive = chat_list_panel_style(false);
        assert_eq!(active.bg, Some(CHAT_LIST_ACTIVE_BG));
        assert_eq!(inactive.bg, Some(CHAT_LIST_BG));
        assert_ne!(active.bg, inactive.bg);
    }

    #[test]
    fn messages_panel_style_has_background() {
        let active = messages_panel_style(true);
        let inactive = messages_panel_style(false);
        assert_eq!(active.bg, Some(MESSAGES_ACTIVE_BG));
        assert_eq!(inactive.bg, Some(MESSAGES_BG));
        assert_ne!(active.bg, inactive.bg);
    }

    #[test]
    fn input_panel_style_has_background() {
        let active = input_panel_style(true);
        let inactive = input_panel_style(false);
        assert_eq!(active.bg, Some(INPUT_ACTIVE_BG));
        assert_eq!(inactive.bg, Some(INPUT_BG));
        assert_ne!(active.bg, inactive.bg);
    }

    #[test]
    fn status_bar_style_has_background_and_foreground() {
        let style = status_bar_style();
        assert_eq!(style.bg, Some(STATUS_BAR_BG));
        assert_eq!(style.fg, Some(STATUS_BAR_FG));
    }
}
