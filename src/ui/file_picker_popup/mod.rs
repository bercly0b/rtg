use chrono::{Local, NaiveDateTime, TimeZone};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph},
    Frame,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::domain::{
    file_picker_state::{FileEntry, FileListing, FilePickerState},
    message::format_file_size,
};

use super::{popup_utils, styles};

const QUERY_PROMPT: &str = "> ";
const FOOTER: &str = "Ctrl+J/Ctrl+K navigate, Ctrl+L open, Enter send, Esc close";
/// Rows around the file list: the query and a blank line above it, a blank
/// line and the footer below.
const CHROME_ROWS: usize = 4;
const COLUMN_GAP: &str = "  ";
/// Widest size, `999.9MB`.
const SIZE_WIDTH: usize = 7;
/// Widest date, `Today 14:32`.
const DATE_WIDTH: usize = 11;
const HORIZONTAL_PADDING: u16 = 1;

pub fn render_file_picker(frame: &mut Frame<'_>, area: Rect, state: &FilePickerState) {
    let popup_area = popup_utils::centered_rect(area, 80, 70);
    let block = Block::default()
        .title(" Attach file ")
        .borders(Borders::ALL)
        .border_style(styles::help_popup_border_style())
        .padding(Padding::horizontal(HORIZONTAL_PADDING));
    let inner = block.inner(popup_area);

    let lines = build_lines(
        state,
        usize::from(inner.width),
        usize::from(inner.height).saturating_sub(CHROME_ROWS),
        Local::now().naive_local(),
    );

    frame.render_widget(Clear, popup_area);
    frame.render_widget(Paragraph::new(lines).block(block), popup_area);

    let cursor_x = inner.x + (QUERY_PROMPT.width() + state.query().width()) as u16;
    if cursor_x < inner.right() && inner.height > 0 {
        frame.set_cursor_position((cursor_x, inner.y));
    }
}

/// The popup content: the query, `list_height` rows of files scrolled to
/// keep the selected one in view, and the footer.
fn build_lines(
    state: &FilePickerState,
    width: usize,
    list_height: usize,
    now: NaiveDateTime,
) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(vec![
            Span::styled(QUERY_PROMPT, styles::help_popup_key_style()),
            Span::styled(state.query().to_owned(), styles::help_popup_action_style()),
        ]),
        Line::from(""),
    ];

    let mut list = match empty_list_message(state) {
        Some((message, style)) => vec![Line::from(Span::styled(message, style))],
        None => file_rows(state, width, list_height, now),
    };
    list.resize(list_height, Line::from(""));
    lines.extend(list);

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        FOOTER,
        styles::help_popup_footer_style(),
    )));
    lines
}

/// What to say instead of the list when no file is shown.
fn empty_list_message(state: &FilePickerState) -> Option<(String, Style)> {
    if state.visible_len() > 0 {
        return None;
    }
    let plain = styles::chat_info_popup_value_style();
    Some(match state.listing() {
        FileListing::Failed(error) => (error.clone(), styles::command_popup_error_style()),
        _ if !state.query().is_empty() => ("No files match".to_owned(), plain),
        FileListing::Running => ("Listing files...".to_owned(), plain),
        FileListing::Finished => ("No files found".to_owned(), plain),
    })
}

fn file_rows(
    state: &FilePickerState,
    width: usize,
    list_height: usize,
    now: NaiveDateTime,
) -> Vec<Line<'static>> {
    let selected = state.selected_index().unwrap_or(0);
    let last_offset = state.visible_len().saturating_sub(list_height);
    let offset = selected.saturating_sub(list_height / 2).min(last_offset);
    let path_width = width.saturating_sub(SIZE_WIDTH + DATE_WIDTH + 2 * COLUMN_GAP.len());

    (offset..state.visible_len())
        .take(list_height)
        .filter_map(|index| Some((index, state.visible_entry(index)?)))
        .map(|(index, entry)| {
            let style = if index == selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                styles::chat_info_popup_value_style()
            };
            Line::from(Span::styled(
                file_row(&state.display_path(entry), entry, path_width, now),
                style,
            ))
        })
        .collect()
}

fn file_row(shown_path: &str, entry: &FileEntry, path_width: usize, now: NaiveDateTime) -> String {
    let path = keep_end(shown_path, path_width);
    let padding = " ".repeat(path_width.saturating_sub(path.width()));
    let size = format_file_size(entry.size);
    let modified = Local
        .timestamp_opt(entry.modified_unix_s, 0)
        .single()
        .map(|modified| format_modified(modified.naive_local(), now))
        .unwrap_or_default();
    format!("{path}{padding}{COLUMN_GAP}{size:>SIZE_WIDTH$}{COLUMN_GAP}{modified:>DATE_WIDTH$}")
}

/// Cuts `text` from the left to fit `max_width` columns, so that the file
/// name at its end stays visible.
fn keep_end(text: &str, max_width: usize) -> String {
    if text.width() <= max_width {
        return text.to_owned();
    }
    if max_width == 0 {
        return String::new();
    }
    let mut kept = Vec::new();
    let mut width = '…'.width().unwrap_or(1);
    for ch in text.chars().rev() {
        width += ch.width().unwrap_or(0);
        if width > max_width {
            break;
        }
        kept.push(ch);
    }
    std::iter::once('…').chain(kept.into_iter().rev()).collect()
}

/// `Today 14:32` for today, `12 Nov` for this year, `2025-03-01` otherwise.
fn format_modified(modified: NaiveDateTime, now: NaiveDateTime) -> String {
    use chrono::Datelike;

    if modified.date() == now.date() {
        modified.format("Today %H:%M").to_string()
    } else if modified.year() == now.year() {
        modified.format("%-d %b").to_string()
    } else {
        modified.format("%Y-%m-%d").to_string()
    }
}

#[cfg(test)]
mod tests;
