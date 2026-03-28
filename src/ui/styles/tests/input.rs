use ratatui::style::Color;

use crate::ui::styles::*;

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
