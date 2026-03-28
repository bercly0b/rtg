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
