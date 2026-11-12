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
fn status_bar_styles_keep_the_terminal_background() {
    for style in [
        connectivity_dot_connected(),
        connectivity_dot_connecting(),
        connectivity_dot_updating(),
        connectivity_dot_disconnected(),
        connectivity_label_style(),
        notification_style(),
        update_notice_style(),
        help_hint_style(),
    ] {
        assert_eq!(style.bg, None);
    }
}

#[test]
fn connectivity_dot_connected_is_green() {
    let style = connectivity_dot_connected();
    assert_eq!(style.fg, Some(Color::Green));
}

#[test]
fn connectivity_dot_connecting_is_yellow() {
    let style = connectivity_dot_connecting();
    assert_eq!(style.fg, Some(Color::Yellow));
}

#[test]
fn connectivity_dot_updating_is_cyan() {
    let style = connectivity_dot_updating();
    assert_eq!(style.fg, Some(Color::Cyan));
}

#[test]
fn connectivity_dot_disconnected_is_red() {
    let style = connectivity_dot_disconnected();
    assert_eq!(style.fg, Some(Color::Red));
}

#[test]
fn connectivity_label_style_uses_the_terminal_foreground() {
    let style = connectivity_label_style();
    assert_eq!(style.fg, None);
}

#[test]
fn notification_style_is_yellow_italic() {
    let style = notification_style();
    assert_eq!(style.fg, Some(Color::Yellow));
    assert!(style.add_modifier.contains(Modifier::ITALIC));
}

#[test]
fn update_notice_style_is_cyan() {
    let style = update_notice_style();
    assert_eq!(style.fg, Some(Color::Cyan));
}

#[test]
fn help_hint_style_is_dark_gray() {
    let style = help_hint_style();
    assert_eq!(style.fg, Some(Color::DarkGray));
}
