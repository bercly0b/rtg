use ratatui::style::{Color, Modifier};

use crate::ui::styles::*;

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

#[test]
fn connectivity_dot_connected_is_green_on_black() {
    let style = connectivity_dot_connected();
    assert_eq!(style.fg, Some(Color::Green));
    assert_eq!(style.bg, Some(Color::Black));
}

#[test]
fn connectivity_dot_connecting_is_yellow_on_black() {
    let style = connectivity_dot_connecting();
    assert_eq!(style.fg, Some(Color::Yellow));
    assert_eq!(style.bg, Some(Color::Black));
}

#[test]
fn connectivity_dot_disconnected_is_red_on_black() {
    let style = connectivity_dot_disconnected();
    assert_eq!(style.fg, Some(Color::Red));
    assert_eq!(style.bg, Some(Color::Black));
}

#[test]
fn connectivity_label_style_is_white_on_black() {
    let style = connectivity_label_style();
    assert_eq!(style.fg, Some(Color::White));
    assert_eq!(style.bg, Some(Color::Black));
}

#[test]
fn notification_style_is_yellow_italic_on_black() {
    let style = notification_style();
    assert_eq!(style.fg, Some(Color::Yellow));
    assert_eq!(style.bg, Some(Color::Black));
    assert!(style.add_modifier.contains(Modifier::ITALIC));
}

#[test]
fn help_hint_style_is_dark_gray_on_black() {
    let style = help_hint_style();
    assert_eq!(style.fg, Some(Color::DarkGray));
    assert_eq!(style.bg, Some(Color::Black));
}
