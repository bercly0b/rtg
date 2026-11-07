use ratatui::style::{Color, Modifier};

use crate::ui::styles::*;

#[test]
fn chat_info_popup_border_style_is_white() {
    let style = chat_info_popup_border_style();
    assert_eq!(style.fg, Some(Color::White));
}

#[test]
fn chat_info_popup_label_style_is_bold_yellow() {
    let style = chat_info_popup_label_style();
    assert_eq!(style.fg, Some(Color::Yellow));
    assert!(style.add_modifier.contains(Modifier::BOLD));
}

#[test]
fn chat_info_popup_value_style_is_white() {
    let style = chat_info_popup_value_style();
    assert_eq!(style.fg, Some(Color::White));
}

#[test]
fn chat_info_popup_title_style_is_bold_white() {
    let style = chat_info_popup_title_style();
    assert_eq!(style.fg, Some(Color::White));
    assert!(style.add_modifier.contains(Modifier::BOLD));
}

#[test]
fn chat_info_popup_status_style_is_gray() {
    assert_eq!(chat_info_popup_status_style().fg, Some(Color::Gray));
}

#[test]
fn chat_info_popup_field_style_is_gray() {
    assert_eq!(chat_info_popup_field_style().fg, Some(Color::Gray));
}

#[test]
fn chat_info_popup_placeholder_style_is_dark_gray() {
    assert_eq!(
        chat_info_popup_placeholder_style().fg,
        Some(Color::DarkGray)
    );
}

#[test]
fn chat_info_popup_warning_style_is_red() {
    assert_eq!(chat_info_popup_warning_style().fg, Some(Color::Red));
}
