use std::{
    io::{BufReader, Read},
    sync::mpsc,
};

use crate::domain::events::CommandEvent;

/// Streams command output and preserves carriage-return replacement semantics.
pub(super) fn stream_output<R: Read>(reader: R, tx: &mpsc::Sender<CommandEvent>) {
    let mut reader = BufReader::new(reader);
    let mut chunk = Vec::<u8>::new();
    let mut byte = [0_u8; 1];
    let mut pending_cr = false;
    let mut current_line_replaces_prev = false;

    loop {
        match reader.read(&mut byte) {
            Ok(0) => {
                if pending_cr {
                    let _ = send_output_chunk(tx, &chunk, true);
                } else if !chunk.is_empty() {
                    let _ = send_output_chunk(tx, &chunk, current_line_replaces_prev);
                }
                break;
            }
            Ok(_) => {
                let b = byte[0];

                if pending_cr {
                    if b == b'\n' {
                        if send_output_chunk(tx, &chunk, false).is_err() {
                            break;
                        }
                        chunk.clear();
                        current_line_replaces_prev = false;
                        pending_cr = false;
                        continue;
                    }

                    if send_output_chunk(tx, &chunk, true).is_err() {
                        break;
                    }
                    chunk.clear();
                    current_line_replaces_prev = true;
                    pending_cr = false;
                }

                match b {
                    b'\r' => {
                        pending_cr = true;
                    }
                    b'\n' => {
                        if send_output_chunk(tx, &chunk, current_line_replaces_prev).is_err() {
                            break;
                        }
                        chunk.clear();
                        current_line_replaces_prev = false;
                    }
                    _ => {
                        chunk.push(b);
                    }
                }
            }
            Err(_) => break,
        }
    }
}

fn send_output_chunk(
    tx: &mpsc::Sender<CommandEvent>,
    chunk: &[u8],
    replace_last: bool,
) -> Result<(), mpsc::SendError<CommandEvent>> {
    let text = sanitize_output(chunk);
    if text.is_empty() {
        return Ok(());
    }

    tx.send(CommandEvent::OutputLine { text, replace_last })
}

fn sanitize_output(chunk: &[u8]) -> String {
    let input = String::from_utf8_lossy(chunk);
    strip_ansi_csi(input.as_ref())
        .chars()
        .filter(|c| !c.is_ascii_control() || *c == '\t')
        .collect()
}

fn strip_ansi_csi(input: &str) -> String {
    #[derive(Clone, Copy)]
    enum State {
        Normal,
        Esc,
        Csi,
    }

    let mut out = String::with_capacity(input.len());
    let mut state = State::Normal;

    for ch in input.chars() {
        match state {
            State::Normal => {
                if ch == '\u{1b}' {
                    state = State::Esc;
                } else {
                    out.push(ch);
                }
            }
            State::Esc => {
                if ch == '[' {
                    state = State::Csi;
                } else {
                    state = State::Normal;
                }
            }
            State::Csi => {
                if ('@'..='~').contains(&ch) {
                    state = State::Normal;
                }
            }
        }
    }

    out
}
