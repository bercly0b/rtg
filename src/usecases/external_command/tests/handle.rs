use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

use crate::domain::events::CommandEvent;

use super::super::*;

fn wait_for_exit(rx: &mpsc::Receiver<CommandEvent>, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(remaining).expect("command must exit") {
            CommandEvent::OutputLine { .. } => {}
            CommandEvent::Exited { success } => return success,
        }
    }
}

#[test]
fn stop_after_exit_returns_without_waiting() {
    let (mut handle, rx) = start_command("true", "/dev/null").unwrap();
    assert!(wait_for_exit(&rx, Duration::from_secs(5)));

    let started = Instant::now();
    handle.stop();

    assert!(
        started.elapsed() < Duration::from_secs(1),
        "stop() must not wait for an already reaped command"
    );
}

#[test]
fn drop_stops_running_command() {
    let (handle, rx) = start_command("sleep 60", "/dev/null").unwrap();

    drop(handle);

    assert!(!wait_for_exit(&rx, Duration::from_secs(5)));
}

#[cfg(unix)]
#[test]
fn stop_kills_process_group_that_ignores_sigterm() {
    let (mut handle, rx) = start_command(
        "sh -c 'trap \"\" TERM; sleep 60 & echo ready; wait'",
        "/dev/null",
    )
    .unwrap();
    let ready = rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(matches!(ready, CommandEvent::OutputLine { text, .. } if text == "ready"));

    handle.stop();

    // The background `sleep` holds the output open, so `Exited` arrives only
    // if the escalation killed the whole process group.
    assert!(!wait_for_exit(&rx, Duration::from_secs(5)));
}
