use std::{
    fs,
    sync::mpsc,
    time::{Duration, Instant, UNIX_EPOCH},
};

use crate::domain::file_picker_state::FileEntry;

use super::super::file_list::{existing_dirs, start_file_list, FileListEvent, FileListHandle};

const TIMEOUT: Duration = Duration::from_secs(10);

fn start(
    template: &str,
    dirs: &[String],
) -> (Option<FileListHandle>, mpsc::Receiver<FileListEvent>) {
    let (tx, rx) = mpsc::channel();
    let handle = start_file_list(template, dirs, move |event| {
        let _ = tx.send(event);
    });
    (handle, rx)
}

/// Runs the command to the end: the files it listed and its final error.
fn run(template: &str, dirs: &[String]) -> (Vec<FileEntry>, Option<String>) {
    let (_handle, rx) = start(template, dirs);
    let mut listed = Vec::new();
    loop {
        match rx.recv_timeout(TIMEOUT).expect("listing must finish") {
            FileListEvent::Entries(entries) => listed.extend(entries),
            FileListEvent::Finished { error } => return (listed, error),
        }
    }
}

fn dir_arg(dir: &tempfile::TempDir) -> Vec<String> {
    vec![dir.path().to_str().unwrap().to_owned()]
}

#[test]
fn lists_files_with_their_size_and_modification_time() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("shot.png");
    fs::write(&path, b"12345").unwrap();
    fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(UNIX_EPOCH + Duration::from_secs(1_700_000_000))
        .unwrap();

    let (listed, error) = run("find {dirs} -type f", &dir_arg(&dir));

    assert_eq!(error, None);
    assert_eq!(
        listed,
        [FileEntry {
            path: path.to_str().unwrap().to_owned(),
            size: 5,
            modified_unix_s: 1_700_000_000,
        }]
    );
}

#[test]
fn lines_that_are_not_existing_files_are_dropped() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), b"a").unwrap();

    let (listed, error) = run(
        r#"sh -c 'echo "$0"; echo "$0/missing.txt"; echo "not a path"; echo "$0/a.txt"' {dirs}"#,
        &dir_arg(&dir),
    );

    assert_eq!(error, None);
    let paths: Vec<_> = listed.iter().map(|entry| entry.path.as_str()).collect();
    assert_eq!(paths, [dir.path().join("a.txt").to_str().unwrap()]);
}

#[test]
fn command_that_does_not_exist_is_reported_by_name() {
    let dir = tempfile::tempdir().unwrap();

    let (listed, error) = run("rtg-no-such-lister {dirs}", &dir_arg(&dir));

    assert!(listed.is_empty());
    assert_eq!(
        error.as_deref(),
        Some("command not found: rtg-no-such-lister")
    );
}

#[test]
fn failed_command_reports_what_it_wrote_to_stderr() {
    let (listed, error) = run("sh -c 'echo boom >&2; exit 3'", &[]);

    assert!(listed.is_empty());
    assert_eq!(error.as_deref(), Some("boom"));
}

#[test]
fn files_named_before_a_failure_are_kept_without_an_error() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), b"a").unwrap();

    let (listed, error) = run(
        r#"sh -c 'echo "$0/a.txt"; echo denied >&2; exit 1' {dirs}"#,
        &dir_arg(&dir),
    );

    assert_eq!(listed.len(), 1);
    assert_eq!(error, None);
}

#[test]
fn command_that_lists_nothing_ends_without_an_error() {
    let dir = tempfile::tempdir().unwrap();

    let (listed, error) = run("find {dirs} -type f", &dir_arg(&dir));

    assert!(listed.is_empty());
    assert_eq!(error, None);
}

#[test]
fn nothing_runs_when_none_of_the_dirs_exist() {
    let (listed, error) = run(
        "find {dirs} -type f",
        &["/rtg-no-such-dir-for-tests".to_owned()],
    );

    assert!(listed.is_empty());
    assert_eq!(error.as_deref(), Some("None of the [attach] dirs exist"));
}

#[test]
fn dropping_the_handle_kills_the_command() {
    let (handle, rx) = start("sleep 60", &[]);
    assert!(handle.is_some());

    let started = Instant::now();
    drop(handle);

    assert_eq!(
        rx.recv_timeout(TIMEOUT)
            .expect("killed command must finish"),
        FileListEvent::Finished {
            error: Some("The file list command failed".to_owned())
        }
    );
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn tilde_stands_for_the_home_directory_and_missing_dirs_are_skipped() {
    let home = tempfile::tempdir().unwrap();
    fs::create_dir(home.path().join("Desktop")).unwrap();
    let home_path = home.path().to_str().unwrap();

    let dirs = existing_dirs(
        &[
            "~/Desktop".to_owned(),
            "~/Downloads".to_owned(),
            "~".to_owned(),
            home_path.to_owned(),
        ],
        Some(home.path()),
    );

    assert_eq!(
        dirs,
        [
            format!("{home_path}/Desktop"),
            home_path.to_owned(),
            home_path.to_owned()
        ]
    );
}
