use ratatui::style::{Color, Modifier};

use crate::ui::styles::*;

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
fn reaction_badge_style_is_light_red() {
    let style = reaction_badge_style();
    assert_eq!(style.fg, Some(Color::LightRed));
}

#[test]
fn chat_preview_style_is_dark_gray() {
    let style = chat_preview_style();
    assert_eq!(style.fg, Some(Color::DarkGray));
}

#[test]
fn section_header_style_is_dark_gray() {
    let style = section_header_style();
    assert_eq!(style.fg, Some(Color::DarkGray));
}

#[test]
fn timestamp_style_is_dark_gray() {
    let style = timestamp_style();
    assert_eq!(style.fg, Some(Color::DarkGray));
}

#[test]
fn separator_style_is_dark_gray() {
    let style = separator_style();
    assert_eq!(style.fg, Some(Color::DarkGray));
}
