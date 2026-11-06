use ratatui::{backend::TestBackend, text::Line, Terminal};

use super::*;
use crate::domain::chat_info_state::{ChatBlockInfo, ChatInfoKind, MuteState};

mod bot;
mod channel;
mod chat_block;
mod group;
mod layout;
mod user;
mod values;

/// Narrow enough for tests to see values wrap.
const VALUE_WIDTH: usize = 40;

fn loaded(status_line: &str, kind: ChatInfoKind) -> ChatInfoPopupState {
    loaded_with_chat(status_line, kind, chat_block())
}

fn loaded_with_chat(
    status_line: &str,
    kind: ChatInfoKind,
    chat: ChatBlockInfo,
) -> ChatInfoPopupState {
    ChatInfoPopupState::Loaded(Box::new(ChatInfo {
        title: "Title".into(),
        status_line: status_line.into(),
        kind,
        chat,
    }))
}

fn chat_block() -> ChatBlockInfo {
    ChatBlockInfo {
        chat_id: 42,
        folders: vec![],
        muted: Some(MuteState::Unmuted),
        auto_delete_seconds: 0,
    }
}

/// The lines `chat_block()` renders to, with the footer after them.
fn chat_block_and_footer() -> Vec<String> {
    vec![
        String::new(),
        "Chat".into(),
        row("Chat ID", "42"),
        row("Folders", "—"),
        row("Muted", "no"),
        row("Auto-delete", "off"),
        String::new(),
        "Press q, Esc or I to close".into(),
    ]
}

fn row(label: &str, value: &str) -> String {
    format!("  {label:<14}{value}")
}

/// A wrapped line of a row value, in the value column.
fn continuation(value: &str) -> String {
    format!("{:16}{value}", "")
}

fn line_text(line: &Line<'_>) -> String {
    line.spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>()
        .trim_end()
        .to_owned()
}

fn rendered_lines(state: &ChatInfoPopupState) -> Vec<String> {
    build_info_lines(state, VALUE_WIDTH)
        .iter()
        .map(line_text)
        .collect()
}

fn with_chat_block(mut lines: Vec<String>) -> Vec<String> {
    lines.extend(chat_block_and_footer());
    lines
}

fn rendered_screen(state: &ChatInfoPopupState, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| render_chat_info_popup(frame, frame.area(), state))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let mut screen = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            screen.push_str(buffer[(x, y)].symbol());
        }
        screen.push('\n');
    }
    screen
}
