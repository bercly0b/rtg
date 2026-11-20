use ratatui::style::{Color, Modifier};

use crate::ui::styles::*;

#[test]
fn help_popup_border_style_is_white() {
    let style = help_popup_border_style();
    assert_eq!(style.fg, Some(Color::White));
}

#[test]
fn help_popup_group_style_is_bold_yellow() {
    let style = help_popup_group_style();
    assert_eq!(style.fg, Some(Color::Yellow));
    assert!(style.add_modifier.contains(Modifier::BOLD));
}

#[test]
fn help_popup_key_style_is_bold_without_a_colour() {
    let style = help_popup_key_style();
    assert_eq!(style.fg, None);
    assert!(style.add_modifier.contains(Modifier::BOLD));
}

#[test]
fn help_popup_action_style_is_gray() {
    let style = help_popup_action_style();
    assert_eq!(style.fg, Some(Color::Gray));
}

#[test]
fn help_popup_footer_style_is_dark_gray() {
    let style = help_popup_footer_style();
    assert_eq!(style.fg, Some(Color::DarkGray));
}
