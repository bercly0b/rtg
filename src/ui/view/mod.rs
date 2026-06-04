mod chat_list;
mod chat_list_item;
mod forum_topic_list_item;
mod messages_panel;
mod status_line;
mod text_utils;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::Line,
    widgets::Paragraph,
    Frame,
};

use crate::domain::{chat::ChatType, keymap::HelpEntry, shell_state::ShellState};

use super::chat_info_popup;
use super::chat_search_popup;
use super::command_popup;
use super::help_popup;
use super::message_info_popup;
use super::message_input::{
    render_message_input, reply_preview_height, CHANNEL_READONLY_PLACEHOLDER, PLACEHOLDER_TEXT,
    TOPIC_CLOSED_PLACEHOLDER,
};
use super::reaction_picker_popup;
use super::styles;

pub fn render(frame: &mut Frame<'_>, state: &mut ShellState, help_entries: &[HelpEntry]) {
    let [content_area, status_separator_area, status_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(frame.area());

    // Horizontal split: chat list | separator (1 char) | messages+input
    let [chats_area, separator_area, messages_with_input_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Length(1),
            Constraint::Percentage(70),
        ])
        .areas(content_area);

    let active_pane = state.active_pane();
    chat_list::render_chat_list_panel(frame, chats_area, state, active_pane);
    render_vertical_separator(frame, separator_area);

    if state.open_chat().is_open() {
        // Compute dynamic input height based on text length and available width.
        let input_height = status_line::compute_input_height(
            state.message_input().text(),
            messages_with_input_area.width,
        )
        .saturating_add(reply_preview_height(state.message_input()));

        // Split right panel into messages area, horizontal separator, and input field
        let [messages_area, input_separator_area, input_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(1),
                Constraint::Length(input_height),
            ])
            .areas(messages_with_input_area);

        messages_panel::render_messages_panel(frame, messages_area, state, active_pane);
        render_horizontal_separator(frame, input_separator_area);
        let placeholder = if state.open_chat().chat_type() == ChatType::Channel {
            CHANNEL_READONLY_PLACEHOLDER
        } else if state.open_topic_is_closed() {
            TOPIC_CLOSED_PLACEHOLDER
        } else {
            PLACEHOLDER_TEXT
        };
        render_message_input(
            frame,
            input_area,
            state.message_input(),
            active_pane,
            placeholder,
        );
    } else {
        // No chat open: hide the input field entirely and give the whole right
        // panel to the (empty) messages panel.
        messages_panel::render_messages_panel(frame, messages_with_input_area, state, active_pane);
    }

    render_horizontal_separator(frame, status_separator_area);
    let status = Paragraph::new(status_line::status_line(state, status_area.width as usize))
        .style(styles::status_bar_style());
    frame.render_widget(status, status_area);

    if let Some(popup_state) = state.command_popup() {
        command_popup::render_command_popup(frame, frame.area(), popup_state);
    }

    if state.help_visible() {
        help_popup::render_help_popup(frame, frame.area(), active_pane, help_entries);
    }

    if let Some(info_state) = state.chat_info_popup() {
        chat_info_popup::render_chat_info_popup(frame, frame.area(), info_state);
    }

    if let Some(msg_info_state) = state.message_info_popup() {
        message_info_popup::render_message_info_popup(frame, frame.area(), msg_info_state);
    }

    if let Some(picker_state) = state.reaction_picker() {
        reaction_picker_popup::render_reaction_picker(frame, frame.area(), picker_state);
    }

    if let Some(search_state) = state.chat_search() {
        chat_search_popup::render_chat_search_popup(frame, frame.area(), search_state);
    }
}

/// Renders a vertical separator line between panels.
fn render_vertical_separator(frame: &mut Frame<'_>, area: Rect) {
    let sep_style = styles::panel_separator_style();
    let lines: Vec<Line<'_>> = (0..area.height)
        .map(|_| Line::styled("\u{2502}", sep_style))
        .collect();
    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}

fn render_horizontal_separator(frame: &mut Frame<'_>, area: Rect) {
    let sep_style = styles::panel_separator_style();
    let line_str: String = "\u{2500}".repeat(area.width as usize);
    let paragraph = Paragraph::new(Line::styled(line_str, sep_style));
    frame.render_widget(paragraph, area);
}

/// Returns the appropriate title style for a panel based on active state.
fn panel_title_style(is_active: bool) -> Style {
    if is_active {
        styles::active_title_style()
    } else {
        styles::inactive_title_style()
    }
}

#[cfg(test)]
mod tests;
