use ratatui::style::Color;

use crate::ui::styles::*;

#[test]
fn command_popup_border_style_is_white() {
    let style = command_popup_border_style();
    assert_eq!(style.fg, Some(Color::White));
}

#[test]
fn command_popup_output_style_is_dark_gray() {
    let style = command_popup_output_style();
    assert_eq!(style.fg, Some(Color::DarkGray));
}

#[test]
fn command_popup_footer_style_is_yellow() {
    let style = command_popup_footer_style();
    assert_eq!(style.fg, Some(Color::Yellow));
}

#[test]
fn command_popup_error_style_is_red() {
    let style = command_popup_error_style();
    assert_eq!(style.fg, Some(Color::Red));
}
