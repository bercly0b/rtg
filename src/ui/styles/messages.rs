//! Style definitions for the message list and reply previews.

use ratatui::style::{Color, Modifier, Style};

use super::{name_to_color_index, SENDER_COLOR_PALETTE};

/// Style for a sender name, colored by identity.
///
/// - Outgoing messages ("You") are always Green + Bold.
/// - Other senders get a deterministic color from the palette based on their name.
pub fn sender_name_style(name: &str, is_outgoing: bool) -> Style {
    if is_outgoing {
        Style::default()
            .fg(Color::Green)
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

/// Style for hyperlink text in messages (underlined).
pub fn message_link_style() -> Style {
    Style::default()
        .fg(Color::Blue)
        .add_modifier(Modifier::UNDERLINED)
}

/// Style for media type indicators like [Photo], [Voice].
pub fn message_media_style() -> Style {
    Style::default().fg(Color::Cyan)
}

/// Style for date separator line.
pub fn date_separator_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

/// Style for the "sending..." status indicator on pending messages.
pub fn message_sending_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

/// Style for the "edited" indicator on edited messages.
pub fn message_edited_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

/// Style for reaction count on individual messages.
pub fn message_reaction_style() -> Style {
    Style::default().fg(Color::LightRed)
}

/// Style for the reply bar character (`|`).
pub fn reply_bar_style() -> Style {
    Style::default().fg(Color::LightBlue)
}

/// Style for the sender name in a reply preview.
///
/// Uses the same deterministic color as the message list so that a user's
/// name always appears in the same color regardless of context.
pub fn reply_sender_style(name: &str, is_outgoing: bool) -> Style {
    sender_name_style(name, is_outgoing)
}

/// Style for the reply text preview.
pub fn reply_text_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

pub fn forward_bar_style() -> Style {
    Style::default().fg(Color::LightGreen)
}

pub fn forward_label_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

pub fn forward_sender_style(name: &str) -> Style {
    let color = SENDER_COLOR_PALETTE[name_to_color_index(name)];
    Style::default().fg(color).add_modifier(Modifier::BOLD)
}
