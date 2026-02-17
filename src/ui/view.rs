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
    let connectivity = state.connectivity_status().as_label();
    format!("mode: {mode} | connectivity: {connectivity} | q/Ctrl+C: quit")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::events::ConnectivityStatus;

    #[test]
    fn status_line_renders_connected_label() {
        let mut state = ShellState::default();
        state.set_connectivity_status(ConnectivityStatus::Connected);

        let line = status_line(&state);

        assert!(line.contains("connectivity: connected"));
    }

    #[test]
    fn status_line_renders_disconnected_label() {
        let mut state = ShellState::default();
        state.set_connectivity_status(ConnectivityStatus::Disconnected);

        let line = status_line(&state);

        assert!(line.contains("connectivity: disconnected"));
    }
}
