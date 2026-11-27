use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::domain::{
    global_search_state::{FoundChat, FoundMessage, GlobalSearchState},
    local_time::LocalTime,
};

use super::{
    popup_utils, styles,
    view::text_utils::{normalize_preview_for_chat_row, truncate_to_display_width},
};

const QUERY_PROMPT: &str = "> ";
const FOOTER: &str = "Ctrl+J/Ctrl+K navigate, Enter open, Esc close";
const SEARCHING: &str = "Searching…";
const NOTHING_FOUND: &str = "Nothing found";
/// Rows around the results: the query and a blank line above them, the
/// status line and the footer below.
const CHROME_ROWS: usize = 4;
const ROW_INDENT: &str = "  ";
const PREVIEW_INDENT: &str = "    ";
const COLUMN_GAP: &str = "  ";
const HORIZONTAL_PADDING: u16 = 1;

/// A line of the results and the row it belongs to; section titles belong
/// to no row.
struct ResultLine {
    text: String,
    hit: Option<usize>,
}

pub fn render_global_search(frame: &mut Frame<'_>, area: Rect, state: &GlobalSearchState) {
    let popup_area = popup_utils::centered_rect(area, 80, 70);
    let block = Block::default()
        .title(" Global search ")
        .borders(Borders::ALL)
        .border_style(styles::help_popup_border_style())
        .padding(Padding::horizontal(HORIZONTAL_PADDING));
    let inner = block.inner(popup_area);

    let lines = build_lines(
        state,
        usize::from(inner.width),
        usize::from(inner.height).saturating_sub(CHROME_ROWS),
        LocalTime::now(),
    );

    frame.render_widget(Clear, popup_area);
    frame.render_widget(Paragraph::new(lines).block(block), popup_area);

    let cursor_x = inner.x + (QUERY_PROMPT.width() + state.query().width()) as u16;
    if cursor_x < inner.right() && inner.height > 0 {
        frame.set_cursor_position((cursor_x, inner.y));
    }
}

/// The popup content: the query, `list_height` rows of results scrolled to
/// keep the selected one in view, the status line and the footer.
fn build_lines(
    state: &GlobalSearchState,
    width: usize,
    list_height: usize,
    now: LocalTime,
) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(vec![
            Span::styled(QUERY_PROMPT, styles::help_popup_key_style()),
            Span::styled(state.query().to_owned(), styles::help_popup_action_style()),
        ]),
        Line::from(""),
    ];

    let mut list = visible_results(state, width, list_height, now);
    list.resize(list_height, Line::from(""));
    lines.extend(list);

    lines.push(Line::from(Span::styled(
        status(state),
        styles::chat_info_popup_value_style(),
    )));
    lines.push(Line::from(Span::styled(
        FOOTER,
        styles::help_popup_footer_style(),
    )));
    lines
}

/// What the line under the results says about the search itself.
fn status(state: &GlobalSearchState) -> &'static str {
    if state.is_searching() {
        SEARCHING
    } else if state.search_text().is_some() && state.selected_index().is_none() {
        NOTHING_FOUND
    } else {
        ""
    }
}

fn visible_results(
    state: &GlobalSearchState,
    width: usize,
    list_height: usize,
    now: LocalTime,
) -> Vec<Line<'static>> {
    let results = result_lines(state, width, now);
    let selected = state.selected_index();
    let first_selected = results
        .iter()
        .position(|line| line.hit.is_some() && line.hit == selected)
        .unwrap_or(0);
    let last_offset = results.len().saturating_sub(list_height);
    let offset = first_selected
        .saturating_sub(list_height / 2)
        .min(last_offset);

    results
        .into_iter()
        .skip(offset)
        .take(list_height)
        .map(|line| {
            let style = match line.hit {
                None => styles::help_popup_group_style(),
                Some(_) if line.hit == selected => Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD),
                Some(_) => styles::chat_info_popup_value_style(),
            };
            Line::from(Span::styled(line.text, style))
        })
        .collect()
}

/// Every section that found something, under its title. Rows are numbered
/// as the selection walks them.
fn result_lines(state: &GlobalSearchState, width: usize, now: LocalTime) -> Vec<ResultLine> {
    let mut lines = Vec::new();
    let mut hit = 0;
    let mut section = |title: &str, rows: Vec<Vec<String>>| {
        if rows.is_empty() {
            return;
        }
        lines.push(ResultLine {
            text: title.to_owned(),
            hit: None,
        });
        for row in rows {
            lines.extend(row.into_iter().map(|text| ResultLine {
                text,
                hit: Some(hit),
            }));
            hit += 1;
        }
    };

    let chat_rows = |chats: Vec<&FoundChat>| -> Vec<Vec<String>> {
        chats
            .into_iter()
            .map(|chat| vec![chat_row(chat, width)])
            .collect()
    };
    section("Chats", chat_rows(state.chats()));
    section("Global", chat_rows(state.public_chats()));
    section(
        "Messages",
        state
            .messages()
            .iter()
            .map(|message| message_row(message, width, now).to_vec())
            .collect(),
    );
    lines
}

fn chat_row(chat: &FoundChat, width: usize) -> String {
    let text = match &chat.username {
        Some(username) => format!("{ROW_INDENT}{}{COLUMN_GAP}@{username}", chat.title),
        None => format!("{ROW_INDENT}{}", chat.title),
    };
    fit(&text, width)
}

/// Two lines: the chat, the sender and the date, then the message text.
fn message_row(message: &FoundMessage, width: usize, now: LocalTime) -> [String; 2] {
    let date = LocalTime::from_unix_millis(message.timestamp_ms)
        .map(|sent| format_date(sent, now))
        .unwrap_or_default();
    let origin = if message.sender_name == message.chat.title {
        format!("{ROW_INDENT}{}", message.chat.title)
    } else {
        format!(
            "{ROW_INDENT}{} · {}",
            message.chat.title, message.sender_name
        )
    };
    let origin_width = width.saturating_sub(date.width() + COLUMN_GAP.len());
    let preview = normalize_preview_for_chat_row(&message.preview);

    [
        format!("{}{COLUMN_GAP}{date}", fit(&origin, origin_width)),
        fit(&format!("{PREVIEW_INDENT}{preview}"), width),
    ]
}

/// Cuts or pads `text` to exactly `width` columns, so that the highlight of
/// the selected row spans the popup.
fn fit(text: &str, width: usize) -> String {
    let (text, padding) = truncate_to_display_width(text, width);
    format!("{text}{}", " ".repeat(padding))
}

/// `14:32` for today, `12 Nov` for this year, `2025-03-01` otherwise.
fn format_date(sent: LocalTime, now: LocalTime) -> String {
    if sent.date() == now.date() {
        sent.format("%H:%M")
    } else if sent.year() == now.year() {
        sent.format("%-d %b")
    } else {
        sent.format("%Y-%m-%d")
    }
}

#[cfg(test)]
mod tests;
