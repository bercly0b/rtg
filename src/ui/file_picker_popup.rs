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
const FOOTER: &str = "Ctrl+J/Ctrl+K navigate, Enter send, Esc close";
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

    let home = dirs::home_dir();
    let lines = build_lines(
        state,
        usize::from(inner.width),
        usize::from(inner.height).saturating_sub(CHROME_ROWS),
        Local::now().naive_local(),
        home.as_deref().and_then(|home| home.to_str()),
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
    home: Option<&str>,
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
        None => file_rows(state, width, list_height, now, home),
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
    home: Option<&str>,
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
            Line::from(Span::styled(file_row(entry, path_width, now, home), style))
        })
        .collect()
}

fn file_row(
    entry: &FileEntry,
    path_width: usize,
    now: NaiveDateTime,
    home: Option<&str>,
) -> String {
    let path = keep_end(&shorten_home(&entry.path, home), path_width);
    let padding = " ".repeat(path_width.saturating_sub(path.width()));
    let size = format_file_size(entry.size);
    let modified = Local
        .timestamp_opt(entry.modified_unix_s, 0)
        .single()
        .map(|modified| format_modified(modified.naive_local(), now))
        .unwrap_or_default();
    format!("{path}{padding}{COLUMN_GAP}{size:>SIZE_WIDTH$}{COLUMN_GAP}{modified:>DATE_WIDTH$}")
}

fn shorten_home(path: &str, home: Option<&str>) -> String {
    home.and_then(|home| path.strip_prefix(home))
        .filter(|rest| rest.starts_with('/'))
        .map_or_else(|| path.to_owned(), |rest| format!("~{rest}"))
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
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn at(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(year, month, day)
            .unwrap()
            .and_hms_opt(hour, minute, 0)
            .unwrap()
    }

    fn now() -> NaiveDateTime {
        at(2026, 11, 16, 18, 0)
    }

    fn entry(path: &str, size: u64, modified: NaiveDateTime) -> FileEntry {
        FileEntry {
            path: path.to_owned(),
            size,
            modified_unix_s: Local.from_local_datetime(&modified).unwrap().timestamp(),
        }
    }

    fn picker(entries: Vec<FileEntry>) -> FilePickerState {
        let mut state = FilePickerState::new(1);
        state.add_entries(entries);
        state
    }

    fn line_text(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    }

    fn texts(state: &FilePickerState, width: usize, list_height: usize) -> Vec<String> {
        build_lines(state, width, list_height, now(), Some("/home/me"))
            .iter()
            .map(line_text)
            .collect()
    }

    fn is_highlighted(line: &Line<'_>) -> bool {
        line.spans[0].style.bg == Some(Color::White)
    }

    #[test]
    fn row_shows_path_size_and_date_in_columns() {
        let state = picker(vec![
            entry(
                "/home/me/Desktop/shot.png",
                1_200_000,
                at(2026, 11, 16, 14, 32),
            ),
            entry("/home/me/Downloads/report.pdf", 512, at(2026, 11, 12, 9, 5)),
            entry("/srv/old.txt", 3_400, at(2025, 3, 1, 9, 5)),
        ]);

        let lines = texts(&state, 52, 3);

        assert_eq!(lines[0], "> ");
        assert_eq!(lines[1], "");
        assert_eq!(
            lines[2],
            "~/Desktop/shot.png                1.2MB  Today 14:32"
        );
        assert_eq!(
            lines[3],
            "~/Downloads/report.pdf             512B       12 Nov"
        );
        assert_eq!(
            lines[4],
            "/srv/old.txt                      3.4KB   2025-03-01"
        );
        assert_eq!(lines[5], "");
        assert_eq!(lines[6], FOOTER);
    }

    #[test]
    fn long_path_is_cut_from_the_left_to_keep_the_file_name() {
        let state = picker(vec![entry(
            "/home/me/Documents/projects/reports/2026/summary.pdf",
            10,
            at(2026, 11, 16, 14, 32),
        )]);

        let lines = texts(&state, 42, 1);

        assert_eq!(lines[2], "…ts/2026/summary.pdf      10B  Today 14:32");
    }

    #[test]
    fn query_is_shown_after_the_prompt() {
        let mut state = picker(vec![entry("/home/me/a.png", 1, now())]);
        state.insert_char('p');
        state.insert_char('n');

        assert_eq!(texts(&state, 40, 1)[0], "> pn");
    }

    #[test]
    fn selected_row_is_the_only_one_highlighted() {
        let mut state = picker(vec![
            entry("/home/me/new.png", 1, at(2026, 11, 16, 14, 0)),
            entry("/home/me/old.png", 1, at(2026, 11, 15, 14, 0)),
        ]);
        state.select_next();

        let lines = build_lines(&state, 40, 2, now(), None);

        assert!(!is_highlighted(&lines[2]));
        assert!(is_highlighted(&lines[3]));
    }

    #[test]
    fn list_scrolls_to_keep_the_selected_file_in_view() {
        let mut state = picker(
            (0..10)
                .map(|i| entry(&format!("/d/file{i}"), 1, at(2026, 11, 16, 17, 50 - i)))
                .collect(),
        );
        (0..7).for_each(|_| state.select_next());

        let lines = build_lines(&state, 40, 4, now(), None);

        let shown: Vec<String> = lines[2..6].iter().map(line_text).collect();
        assert!(shown[0].starts_with("/d/file5"));
        assert!(shown[3].starts_with("/d/file8"));
        assert!(is_highlighted(&lines[4]));
        assert_eq!(lines.len(), 8);
    }

    #[test]
    fn says_listing_while_the_command_has_named_nothing_yet() {
        let state = FilePickerState::new(1);

        assert_eq!(texts(&state, 40, 3)[2], "Listing files...");
    }

    #[test]
    fn says_no_files_found_when_the_command_named_nothing() {
        let mut state = FilePickerState::new(1);
        state.finish(None);

        assert_eq!(texts(&state, 40, 3)[2], "No files found");
    }

    #[test]
    fn says_no_files_match_when_the_query_filters_everything_out() {
        let mut state = picker(vec![entry("/home/me/a.png", 1, now())]);
        state.finish(None);
        state.insert_char('z');

        assert_eq!(texts(&state, 40, 3)[2], "No files match");
    }

    #[test]
    fn shows_why_the_file_list_command_failed() {
        let mut state = FilePickerState::new(1);
        state.finish(Some("command not found: fd".to_owned()));

        let lines = build_lines(&state, 40, 3, now(), None);

        assert_eq!(line_text(&lines[2]), "command not found: fd");
        assert_eq!(lines[2].spans[0].style.fg, Some(Color::Red));
    }

    #[test]
    fn height_stays_the_same_whatever_the_list_holds() {
        let empty = FilePickerState::new(1);
        let full = picker(
            (0..20)
                .map(|i| entry(&format!("/d/file{i}"), 1, at(2026, 11, 16, 17, 30 - i)))
                .collect(),
        );

        assert_eq!(texts(&empty, 40, 5).len(), 5 + CHROME_ROWS);
        assert_eq!(texts(&full, 40, 5).len(), 5 + CHROME_ROWS);
    }

    #[test]
    fn home_is_shortened_only_as_a_whole_directory() {
        assert_eq!(shorten_home("/home/me/a.png", Some("/home/me")), "~/a.png");
        assert_eq!(
            shorten_home("/home/merlin/a.png", Some("/home/me")),
            "/home/merlin/a.png"
        );
    }
}
