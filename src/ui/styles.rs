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

/// Palette of distinguishable colors for sender names on dark backgrounds.
const SENDER_COLOR_PALETTE: &[Color] = &[
    Color::Red,
    Color::Magenta,
    Color::Yellow,
    Color::LightBlue,
    Color::LightCyan,
    Color::LightMagenta,
    Color::LightGreen,
    Color::LightRed,
];

/// Deterministic hash of a name to a palette index.
///
/// Uses FNV-1a-inspired hashing for stable, well-distributed results.
fn name_to_color_index(name: &str) -> usize {
    let mut hash: u32 = 2_166_136_261;
    for byte in name.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16_777_619);
    }
    (hash as usize) % SENDER_COLOR_PALETTE.len()
}

/// Style for a sender name, colored by identity.
///
/// - Outgoing messages ("You") are always Cyan + Bold.
/// - Other senders get a deterministic color from the palette based on their name.
pub fn sender_name_style(name: &str, is_outgoing: bool) -> Style {
    if is_outgoing {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        let color = SENDER_COLOR_PALETTE[name_to_color_index(name)];
        Style::default().fg(color).add_modifier(Modifier::BOLD)
    }
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
// Panel styles
//
// All panels use the terminal's default background (no bg override) so that
// the TUI inherits whatever color scheme the user has configured. Only the
// status bar and the panel separator use explicit ANSI colors (0-15), which
// are controlled by the user's terminal theme.
//
// Active panel is indicated by a green title, matching the existing green
// accent used for unread badges, online indicators, and the input prompt.
// =============================================================================

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
    fn sender_name_style_outgoing_is_cyan_bold() {
        let style = sender_name_style("You", true);
        assert_eq!(style.fg, Some(Color::Cyan));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn sender_name_style_incoming_is_bold() {
        let style = sender_name_style("Alice", false);
        assert!(style.add_modifier.contains(Modifier::BOLD));
        // Should not be cyan (that's reserved for outgoing)
        assert_ne!(style.fg, Some(Color::White));
    }

    #[test]
    fn sender_name_style_is_deterministic() {
        let style1 = sender_name_style("Alice", false);
        let style2 = sender_name_style("Alice", false);
        assert_eq!(style1.fg, style2.fg);
    }

    #[test]
    fn sender_name_style_different_names_can_differ() {
        // With 8 colors and different names, at least some should differ
        let names = ["Alice", "Bob", "Charlie", "Diana", "Eve", "Frank"];
        let colors: Vec<_> = names
            .iter()
            .map(|n| sender_name_style(n, false).fg)
            .collect();
        let unique: std::collections::HashSet<_> = colors.iter().collect();
        assert!(
            unique.len() > 1,
            "Expected different colors for different names"
        );
    }

    #[test]
    fn name_to_color_index_stays_in_bounds() {
        let names = ["", "a", "Alice", "Bob", "Very Long Name With Spaces"];
        for name in &names {
            let idx = name_to_color_index(name);
            assert!(
                idx < SENDER_COLOR_PALETTE.len(),
                "Index out of bounds for '{}'",
                name
            );
        }
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
    fn highlight_style_is_gray_bg_black_fg() {
        let style = highlight_style();
        assert_eq!(style.fg, Some(Color::Black));
        assert_eq!(style.bg, Some(Color::Gray));
    }

    #[test]
    fn active_title_style_is_green_bold() {
        let style = active_title_style();
        assert_eq!(style.fg, Some(Color::Green));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn inactive_title_style_is_dark_gray_bold() {
        let style = inactive_title_style();
        assert_eq!(style.fg, Some(Color::DarkGray));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn panel_separator_style_is_dark_gray() {
        let style = panel_separator_style();
        assert_eq!(style.fg, Some(Color::DarkGray));
    }

    #[test]
    fn status_bar_style_uses_ansi_black_bg() {
        let style = status_bar_style();
        assert_eq!(style.bg, Some(Color::Black));
    }
}
