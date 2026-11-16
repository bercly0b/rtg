use chrono::NaiveDate;

use super::*;

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
    let mut state = FilePickerState::new(1, Some("/home/me".to_owned()));
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
    build_lines(state, width, list_height, now())
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

    let lines = build_lines(&state, 40, 2, now());

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

    let lines = build_lines(&state, 40, 4, now());

    let shown: Vec<String> = lines[2..6].iter().map(line_text).collect();
    assert!(shown[0].starts_with("/d/file5"));
    assert!(shown[3].starts_with("/d/file8"));
    assert!(is_highlighted(&lines[4]));
    assert_eq!(lines.len(), 8);
}

#[test]
fn says_listing_while_the_command_has_named_nothing_yet() {
    let state = FilePickerState::new(1, None);

    assert_eq!(texts(&state, 40, 3)[2], "Listing files...");
}

#[test]
fn says_no_files_found_when_the_command_named_nothing() {
    let mut state = FilePickerState::new(1, None);
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
    let mut state = FilePickerState::new(1, None);
    state.finish(Some("command not found: fd".to_owned()));

    let lines = build_lines(&state, 40, 3, now());

    assert_eq!(line_text(&lines[2]), "command not found: fd");
    assert_eq!(lines[2].spans[0].style.fg, Some(Color::Red));
}

#[test]
fn height_stays_the_same_whatever_the_list_holds() {
    let empty = FilePickerState::new(1, None);
    let full = picker(
        (0..20)
            .map(|i| entry(&format!("/d/file{i}"), 1, at(2026, 11, 16, 17, 30 - i)))
            .collect(),
    );

    assert_eq!(texts(&empty, 40, 5).len(), 5 + CHROME_ROWS);
    assert_eq!(texts(&full, 40, 5).len(), 5 + CHROME_ROWS);
}
