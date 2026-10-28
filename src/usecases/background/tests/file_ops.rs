use std::{sync::mpsc, time::Duration};

use crate::{domain::events::BackgroundTaskResult, usecases::background::file_ops};

fn open_file_failure(cmd_template: &str, file_path: &str) -> String {
    let (tx, rx) = mpsc::channel();
    file_ops::dispatch_open_file(&tx, cmd_template.to_owned(), file_path.to_owned());

    match rx
        .recv_timeout(Duration::from_secs(5))
        .expect("open command must report failure")
    {
        BackgroundTaskResult::OpenFileFailed { stderr } => stderr,
        other => panic!("unexpected result: {other:?}"),
    }
}

#[cfg(unix)]
#[test]
fn open_file_passes_quoted_arguments_and_file_path_as_single_tokens() {
    let stderr = open_file_failure(
        r#"sh -c 'printf "%s|%s" "$1" "$2" >&2; exit 1' sh 'Built-in Microphone' {file_path}"#,
        "/tmp/my photo.jpg",
    );

    assert_eq!(stderr, "Built-in Microphone|/tmp/my photo.jpg");
}

#[test]
fn open_file_reports_unbalanced_quote() {
    let stderr = open_file_failure("open '{file_path}", "/tmp/photo.jpg");

    assert!(stderr.contains("missing closing quote"), "{stderr}");
}

#[test]
fn open_file_reports_empty_command() {
    let stderr = open_file_failure("   ", "/tmp/photo.jpg");

    assert_eq!(stderr, "empty command");
}
