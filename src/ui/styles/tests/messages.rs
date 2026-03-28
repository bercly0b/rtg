use ratatui::style::{Color, Modifier};

use crate::ui::styles::{self, *};

#[test]
fn sender_name_style_outgoing_is_green_bold() {
    let style = sender_name_style("You", true);
    assert_eq!(style.fg, Some(Color::Green));
    assert!(style.add_modifier.contains(Modifier::BOLD));
}

#[test]
fn sender_name_style_incoming_is_bold_and_avoids_reserved_colors() {
    let style = sender_name_style("Alice", false);
    assert!(style.add_modifier.contains(Modifier::BOLD));
    assert_ne!(
        style.fg,
        Some(Color::Green),
        "Should not use Green (outgoing)"
    );
    assert_ne!(style.fg, Some(Color::Cyan), "Should not use Cyan (media)");
}

#[test]
fn sender_name_style_is_deterministic() {
    let style1 = sender_name_style("Alice", false);
    let style2 = sender_name_style("Alice", false);
    assert_eq!(style1.fg, style2.fg);
}

#[test]
fn sender_name_style_different_names_can_differ() {
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
fn sender_palette_avoids_reserved_colors() {
    for color in styles::SENDER_COLOR_PALETTE {
        assert_ne!(
            *color,
            Color::Cyan,
            "Palette must not contain Cyan (media indicators)"
        );
        assert_ne!(
            *color,
            Color::Green,
            "Palette must not contain Green (outgoing sender)"
        );
    }
}

#[test]
fn sender_palette_avoids_green_like_colors() {
    for color in styles::SENDER_COLOR_PALETTE {
        assert_ne!(
            *color,
            Color::LightGreen,
            "Palette must not contain LightGreen (too close to Green/You)"
        );
    }
}

#[test]
fn name_to_color_index_stays_in_bounds() {
    let names = ["", "a", "Alice", "Bob", "Very Long Name With Spaces"];
    for name in &names {
        let idx = styles::name_to_color_index(name);
        assert!(
            idx < styles::SENDER_COLOR_PALETTE.len(),
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
fn message_edited_style_is_dark_gray() {
    let style = message_edited_style();
    assert_eq!(style.fg, Some(Color::DarkGray));
}

#[test]
fn message_reaction_style_is_light_red() {
    let style = message_reaction_style();
    assert_eq!(style.fg, Some(Color::LightRed));
}

#[test]
fn reply_bar_style_is_light_blue() {
    let style = reply_bar_style();
    assert_eq!(style.fg, Some(Color::LightBlue));
}

#[test]
fn reply_sender_style_matches_message_list_color() {
    let names = ["Alice", "Bob", "Charlie", "Diana"];
    for name in &names {
        let reply = reply_sender_style(name, false);
        let message = sender_name_style(name, false);
        assert_eq!(
            reply.fg, message.fg,
            "Color mismatch for '{}': reply {:?} vs message {:?}",
            name, reply.fg, message.fg,
        );
        assert!(reply.add_modifier.contains(Modifier::BOLD));
    }
}

#[test]
fn reply_sender_style_outgoing_is_green() {
    let style = reply_sender_style("You", true);
    assert_eq!(style.fg, Some(Color::Green));
    assert!(style.add_modifier.contains(Modifier::BOLD));
}

#[test]
fn reply_sender_style_is_deterministic() {
    let a = reply_sender_style("Alice", false);
    let b = reply_sender_style("Alice", false);
    assert_eq!(a.fg, b.fg);
}

#[test]
fn reply_text_style_is_dark_gray() {
    let style = reply_text_style();
    assert_eq!(style.fg, Some(Color::DarkGray));
}

#[test]
fn message_time_style_is_dark_gray() {
    let style = message_time_style();
    assert_eq!(style.fg, Some(Color::DarkGray));
}

#[test]
fn message_text_style_is_white() {
    let style = message_text_style();
    assert_eq!(style.fg, Some(Color::White));
}

#[test]
fn message_link_style_is_blue_underlined() {
    let style = message_link_style();
    assert_eq!(style.fg, Some(Color::Blue));
    assert!(style.add_modifier.contains(Modifier::UNDERLINED));
}

#[test]
fn message_sending_style_is_dark_gray() {
    let style = message_sending_style();
    assert_eq!(style.fg, Some(Color::DarkGray));
}
