use std::sync::mpsc;

use crate::domain::events::CommandEvent;

use super::super::output::stream_output;

#[test]
fn stream_output_sends_output_lines() {
    let input = b"line 1\nline 2\nline 3\n";
    let (tx, rx) = mpsc::channel();

    stream_output(&input[..], &tx);
    drop(tx);

    let events: Vec<_> = rx.iter().collect();
    assert_eq!(events.len(), 3);
    assert_eq!(
        events[0],
        CommandEvent::OutputLine {
            text: "line 1".into(),
            replace_last: false,
        }
    );
    assert_eq!(
        events[1],
        CommandEvent::OutputLine {
            text: "line 2".into(),
            replace_last: false,
        }
    );
    assert_eq!(
        events[2],
        CommandEvent::OutputLine {
            text: "line 3".into(),
            replace_last: false,
        }
    );
}

#[test]
fn stream_output_handles_empty_input() {
    let input = b"";
    let (tx, rx) = mpsc::channel();

    stream_output(&input[..], &tx);
    drop(tx);

    let events: Vec<_> = rx.iter().collect();
    assert!(events.is_empty());
}

#[test]
fn stream_output_marks_carriage_return_as_replace() {
    let input = b"A: 00:00:01 / 00:00:03\rA: 00:00:02 / 00:00:03\r";
    let (tx, rx) = mpsc::channel();

    stream_output(&input[..], &tx);
    drop(tx);

    let events: Vec<_> = rx.iter().collect();
    assert_eq!(events.len(), 2);
    assert_eq!(
        events[0],
        CommandEvent::OutputLine {
            text: "A: 00:00:01 / 00:00:03".into(),
            replace_last: true,
        }
    );
    assert_eq!(
        events[1],
        CommandEvent::OutputLine {
            text: "A: 00:00:02 / 00:00:03".into(),
            replace_last: true,
        }
    );
}

#[test]
fn stream_output_treats_crlf_as_normal_newline() {
    let input = b"line 1\r\nline 2\r\n";
    let (tx, rx) = mpsc::channel();

    stream_output(&input[..], &tx);
    drop(tx);

    let events: Vec<_> = rx.iter().collect();
    assert_eq!(events.len(), 2);
    assert_eq!(
        events[0],
        CommandEvent::OutputLine {
            text: "line 1".into(),
            replace_last: false,
        }
    );
    assert_eq!(
        events[1],
        CommandEvent::OutputLine {
            text: "line 2".into(),
            replace_last: false,
        }
    );
}

#[test]
fn stream_output_keeps_replace_semantics_for_cr_then_lf() {
    let input = b"foo\rbar\n";
    let (tx, rx) = mpsc::channel();

    stream_output(&input[..], &tx);
    drop(tx);

    let events: Vec<_> = rx.iter().collect();
    assert_eq!(events.len(), 2);
    assert_eq!(
        events[0],
        CommandEvent::OutputLine {
            text: "foo".into(),
            replace_last: true,
        }
    );
    assert_eq!(
        events[1],
        CommandEvent::OutputLine {
            text: "bar".into(),
            replace_last: true,
        }
    );
}

#[test]
fn stream_output_strips_ansi_sequences() {
    let input = b"\x1b[33mwarn\x1b[0m\n";
    let (tx, rx) = mpsc::channel();

    stream_output(&input[..], &tx);
    drop(tx);

    let events: Vec<_> = rx.iter().collect();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0],
        CommandEvent::OutputLine {
            text: "warn".into(),
            replace_last: false,
        }
    );
}
