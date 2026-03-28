use std::sync::{mpsc, Arc};

use crate::domain::events::CommandEvent;

use super::super::command::{self, start_command};

#[test]
fn reader_gate_sends_exited_when_all_done() {
    let gate = Arc::new(command::ReaderGate::new());
    let (tx, rx) = mpsc::channel();
    gate.set_expected(2, &tx);

    // First reader finishes — should not send Exited yet.
    gate.on_reader_finished(&tx);
    assert!(rx.try_recv().is_err());

    // Second reader finishes — should send Exited.
    gate.on_reader_finished(&tx);
    let event = rx.try_recv().unwrap();
    assert_eq!(event, CommandEvent::Exited { success: true });
}

#[test]
fn reader_gate_single_reader() {
    let gate = Arc::new(command::ReaderGate::new());
    let (tx, rx) = mpsc::channel();
    gate.set_expected(1, &tx);

    gate.on_reader_finished(&tx);
    let event = rx.try_recv().unwrap();
    assert_eq!(event, CommandEvent::Exited { success: true });
}

#[test]
fn reader_gate_emits_exited_when_expected_is_set_after_finish() {
    let gate = Arc::new(command::ReaderGate::new());
    let (tx, rx) = mpsc::channel();

    gate.on_reader_finished(&tx);
    assert!(rx.try_recv().is_err());

    gate.set_expected(1, &tx);
    let event = rx.try_recv().unwrap();
    assert_eq!(event, CommandEvent::Exited { success: true });
}

#[cfg(unix)]
#[test]
fn child_inherits_no_high_fds_from_parent() {
    let (mut handle, rx) = start_command("ls /dev/fd/", "/dev/null").unwrap();

    std::thread::sleep(std::time::Duration::from_millis(200));
    assert!(handle.try_exit_success().is_some());

    let events: Vec<_> = rx.try_iter().collect();
    let open_fds: Vec<i32> = events
        .iter()
        .filter_map(|e| match e {
            CommandEvent::OutputLine { text, .. } => text.trim().parse::<i32>().ok(),
            _ => None,
        })
        .collect();

    let max_fd = open_fds.iter().copied().max().unwrap_or(0);
    assert!(
        max_fd <= 4,
        "inherited high FDs leaked to child: {:?}",
        open_fds
    );
}

#[cfg(unix)]
#[test]
fn stop_terminates_process_group_via_start_command() {
    let (mut handle, rx) = start_command("sleep 60", "/dev/null").unwrap();

    // Process should be running.
    assert_eq!(handle.try_exit_success(), None);

    handle.stop();

    // After stop, the process should be reaped.
    assert!(handle.try_exit_success().is_some());

    // Pipe readers should have finished, producing an Exited event.
    let events: Vec<_> = rx.try_iter().collect();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, CommandEvent::Exited { .. })),
        "expected Exited event after stop(), got: {:?}",
        events
    );
}
