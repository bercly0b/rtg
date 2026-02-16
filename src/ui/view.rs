use ratatui::{
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::domain::shell_state::ShellState;

pub fn render(frame: &mut Frame<'_>, state: &ShellState) {
    let [content_area, status_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .areas(frame.area());

    let [chats_area, messages_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .areas(content_area);

    let chats = Block::default().title("Chats").borders(Borders::ALL);
    let messages = Block::default().title("Messages").borders(Borders::ALL);

    frame.render_widget(chats, chats_area);
    frame.render_widget(messages, messages_area);

    let status = Paragraph::new(status_line(state));
    frame.render_widget(status, status_area);
}

fn status_line(state: &ShellState) -> String {
    let mode = if state.is_running() {
        "running"
    } else {
        "stopping"
    };
    format!("mode: {mode} | q/Ctrl+C: quit")
}
