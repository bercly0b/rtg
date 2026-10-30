use std::sync::mpsc;

use crate::domain::events::CommandEvent;

use super::super::command::start_command;

/// Collects output lines until `Exited` and returns them with the exit status.
fn run_until_exit(rx: &mpsc::Receiver<CommandEvent>) -> (Vec<String>, bool) {
    let mut lines = Vec::new();
    loop {
        match rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("command must exit")
        {
            CommandEvent::OutputLine { text, .. } => lines.push(text),
            CommandEvent::Exited { success } => return (lines, success),
        }
    }
}

fn exit_status_of(cmd_template: &str, file_path: &str) -> bool {
    let (_handle, rx) = start_command(cmd_template, file_path).unwrap();
    run_until_exit(&rx).1
}

#[test]
fn exit_event_reports_zero_exit_code_as_success() {
    assert!(exit_status_of("true", "/dev/null"));
}

#[test]
fn exit_event_reports_nonzero_exit_code_as_failure() {
    assert!(!exit_status_of("false", "/dev/null"));
    assert!(!exit_status_of("sh -c 'exit 3'", "/dev/null"));
}

#[cfg(unix)]
#[test]
fn exit_event_waits_for_process_that_closes_output_early() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("exited");

    let success = exit_status_of(
        "sh -c 'exec >/dev/null 2>&1; sleep 0.2; touch \"$0\"' {file_path}",
        marker.to_str().unwrap(),
    );

    assert!(success);
    assert!(
        marker.exists(),
        "Exited must not be sent before the process finishes"
    );
}

#[cfg(unix)]
#[test]
fn exit_event_keeps_failure_of_process_that_closes_output_early() {
    assert!(!exit_status_of(
        "sh -c 'exec >/dev/null 2>&1; sleep 0.2; exit 3'",
        "/dev/null"
    ));
}

#[test]
fn start_command_reports_missing_binary_as_start_error() {
    let Err(error) = start_command("rtg-test-missing-binary {file_path}", "/dev/null") else {
        panic!("missing binary must fail to start");
    };

    assert!(
        format!("{error:#}").contains("failed to start `rtg-test-missing-binary`"),
        "{error:#}"
    );
    assert_eq!(
        error
            .root_cause()
            .downcast_ref::<std::io::Error>()
            .map(std::io::Error::kind),
        Some(std::io::ErrorKind::NotFound)
    );
}

/// Lists the descriptors open in a command started via `start_command`.
#[cfg(unix)]
fn child_open_fds() -> Vec<i32> {
    let (_handle, rx) = start_command("ls /dev/fd/", "/dev/null").unwrap();
    let (lines, _) = run_until_exit(&rx);
    // On a terminal `ls` prints several entries per line.
    lines
        .iter()
        .flat_map(|line| line.split_whitespace())
        .map(|fd| fd.parse().unwrap())
        .collect()
}

#[cfg(unix)]
#[test]
fn child_inherits_no_high_fds_from_parent() {
    let open_fds = child_open_fds();

    let max_fd = open_fds.iter().copied().max().unwrap_or(0);
    assert!(
        max_fd <= 4,
        "inherited high FDs leaked to child: {:?}",
        open_fds
    );
}

#[cfg(unix)]
#[test]
fn child_inherits_no_fds_above_1024() {
    use std::os::fd::AsRawFd;

    const MIN_FD: libc::c_int = 1500;

    let mut limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    // SAFETY: `limit` is a valid, writable `rlimit`; raising the soft limit
    // up to the hard one is harmless for the rest of the test process.
    unsafe {
        assert_eq!(libc::getrlimit(libc::RLIMIT_NOFILE, &mut limit), 0);
        if limit.rlim_cur <= MIN_FD as libc::rlim_t {
            limit.rlim_cur = limit.rlim_max.min(2 * MIN_FD as libc::rlim_t);
            assert_eq!(libc::setrlimit(libc::RLIMIT_NOFILE, &limit), 0);
        }
    }

    let dev_null = std::fs::File::open("/dev/null").unwrap();
    // SAFETY: duplicates a descriptor we own; F_DUPFD leaves the duplicate
    // without close-on-exec, like a descriptor leaked by a library.
    let high_fd = unsafe { libc::fcntl(dev_null.as_raw_fd(), libc::F_DUPFD, MIN_FD) };
    assert!(high_fd >= MIN_FD, "{}", std::io::Error::last_os_error());

    let open_fds = child_open_fds();
    // SAFETY: `high_fd` was opened above and is not used anywhere else.
    unsafe {
        libc::close(high_fd);
    }

    assert!(
        !open_fds.contains(&high_fd),
        "fd {high_fd} leaked to child: {open_fds:?}"
    );
}

#[cfg(unix)]
#[test]
fn stop_terminates_process_group_via_start_command() {
    let (mut handle, rx) = start_command("sleep 60", "/dev/null").unwrap();

    handle.stop();

    let (_, success) = run_until_exit(&rx);
    assert!(!success, "a terminated command must not report success");
}

#[cfg(unix)]
#[test]
fn quoted_arguments_and_file_path_reach_child_as_single_tokens() {
    let (_handle, rx) = start_command(
        r"printf '[%s]\n' ':MacBook Pro Microphone' {file_path}",
        "/tmp/my voice.oga",
    )
    .unwrap();

    assert_eq!(
        run_until_exit(&rx).0,
        ["[:MacBook Pro Microphone]", "[/tmp/my voice.oga]"]
    );
}

#[test]
fn start_command_rejects_unbalanced_quote() {
    let Err(error) = start_command("ffmpeg -i ':0 {file_path}", "/dev/null") else {
        panic!("unbalanced quote must be rejected");
    };

    assert!(
        error.to_string().contains("missing closing quote"),
        "{error}"
    );
}

#[test]
fn start_command_rejects_empty_template() {
    let Err(error) = start_command("  ", "/dev/null") else {
        panic!("empty template must be rejected");
    };

    assert_eq!(error.to_string(), "empty command");
}
