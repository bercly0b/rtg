//! Renders the chat info popup: a header with the chat title and status
//! line, then the blocks of the chat's info template.

mod format;
mod templates;

use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph, Wrap},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::domain::chat_info_state::{Badge, ChatInfo, ChatInfoPopupState};

use super::{popup_utils, styles};
use templates::{InfoRow, Value};

const MISSING: &str = "—";
const HIDDEN: &str = "hidden";
const ROW_INDENT: usize = 2;
/// Fits the longest row label, "Photo changed", and a space.
const LABEL_WIDTH: usize = 14;
/// Below this, wrapping at the value column leaves too few words per line;
/// the paragraph wraps whatever is still too long.
const MIN_VALUE_WIDTH: usize = 20;

/// Renders the chat info popup as an overlay on top of existing content.
pub fn render_chat_info_popup(frame: &mut Frame<'_>, area: Rect, state: &ChatInfoPopupState) {
    let title = " Chat Info ";
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(styles::chat_info_popup_border_style())
        .padding(Padding::new(2, 2, 1, 1));

    let max_width = usize::from(popup_utils::max_content_width(area, &block));
    let value_width = max_width
        .saturating_sub(ROW_INDENT + LABEL_WIDTH)
        .max(MIN_VALUE_WIDTH);
    let lines = build_info_lines(state, value_width);
    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    let popup_area = popup_utils::content_sized_rect(area, title, &block, &paragraph);

    frame.render_widget(Clear, popup_area);
    frame.render_widget(paragraph.block(block), popup_area);
}

/// `value_width` is where row values wrap, keeping their lines in the
/// value column.
fn build_info_lines(state: &ChatInfoPopupState, value_width: usize) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(Span::styled(
        state.title().to_owned(),
        styles::chat_info_popup_title_style(),
    ))];
    match state {
        ChatInfoPopupState::Loading { .. } => lines.push(Line::from(Span::styled(
            "Loading...",
            styles::chat_info_popup_value_style(),
        ))),
        ChatInfoPopupState::Error { .. } => lines.push(Line::from(Span::styled(
            "Failed to load chat info",
            styles::command_popup_error_style(),
        ))),
        ChatInfoPopupState::Loaded(info) => lines.extend(build_loaded_lines(info, value_width)),
    }
    lines
}

fn build_loaded_lines(info: &ChatInfo, value_width: usize) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(Span::styled(
        info.status_line.clone(),
        styles::chat_info_popup_status_style(),
    ))];

    for block in templates::blocks(info) {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            block.title,
            styles::chat_info_popup_label_style(),
        )));
        for row in &block.rows {
            lines.extend(row_lines(row, value_width));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press q, Esc or I to close",
        styles::help_popup_footer_style(),
    )));

    lines
}

fn row_lines(row: &InfoRow, value_width: usize) -> Vec<Line<'static>> {
    let label = Span::styled(
        format!("{:ROW_INDENT$}{:<LABEL_WIDTH$}", "", row.label),
        styles::chat_info_popup_field_style(),
    );
    let placeholder = |text: &'static str| {
        vec![Line::from(vec![
            label.clone(),
            Span::styled(text, styles::chat_info_popup_placeholder_style()),
        ])]
    };

    match &row.value {
        Value::Missing => placeholder(MISSING),
        Value::Hidden => placeholder(HIDDEN),
        Value::Badges(badges) => vec![Line::from(
            std::iter::once(label.clone())
                .chain(badge_spans(badges))
                .collect::<Vec<_>>(),
        )],
        Value::Text(text) => wrap(text, value_width)
            .into_iter()
            .enumerate()
            .map(|(index, part)| {
                let lead = if index == 0 {
                    label.clone()
                } else {
                    Span::raw(" ".repeat(ROW_INDENT + LABEL_WIDTH))
                };
                Line::from(vec![
                    lead,
                    Span::styled(part, styles::chat_info_popup_value_style()),
                ])
            })
            .collect(),
    }
}

fn badge_spans(badges: &[Badge]) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for (index, &badge) in badges.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(", ", styles::chat_info_popup_value_style()));
        }
        let style = if badge.is_warning() {
            styles::chat_info_popup_warning_style()
        } else {
            styles::chat_info_popup_value_style()
        };
        spans.push(Span::styled(format::badge(badge), style));
    }
    spans
}

/// Greedy word wrap that keeps the text's own line breaks; a word wider
/// than `width` stays whole.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.lines() {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            if !line.is_empty() && line.width() + 1 + word.width() > width {
                lines.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests;
