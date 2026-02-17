use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::domain::{chat::ChatSummary, chat_list_state::ChatListUiState, shell_state::ShellState};

pub fn render(frame: &mut Frame<'_>, state: &ShellState) {
    let [content_area, status_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .areas(frame.area());

    let [chats_area, messages_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .areas(content_area);

    render_chat_list_panel(frame, chats_area, state);

    let messages = Block::default().title("Messages").borders(Borders::ALL);
    frame.render_widget(messages, messages_area);

    let status = Paragraph::new(status_line(state));
    frame.render_widget(status, status_area);
}

fn render_chat_list_panel(frame: &mut Frame<'_>, area: ratatui::layout::Rect, state: &ShellState) {
    let chat_list = state.chat_list();
    match chat_list.ui_state() {
        ChatListUiState::Loading => render_chat_list_message(frame, area, "Loading chats..."),
        ChatListUiState::Empty => {
            render_chat_list_message(frame, area, "No chats yet. Press refresh to try again.")
        }
        ChatListUiState::Error => render_chat_list_message(
            frame,
            area,
            "Failed to load chats. Check connection and retry.",
        ),
        ChatListUiState::Ready => {
            let items = chat_list
                .chats()
                .iter()
                .map(chat_list_item)
                .collect::<Vec<_>>();

            let list = List::new(items)
                .block(Block::default().title("Chats").borders(Borders::ALL))
                .highlight_style(Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD))
                .highlight_symbol("> ");

            let mut list_state = ListState::default();
            list_state.select(chat_list.selected_index());
            frame.render_stateful_widget(list, area, &mut list_state);
        }
    }
}

fn render_chat_list_message(frame: &mut Frame<'_>, area: ratatui::layout::Rect, message: &str) {
    let message =
        Paragraph::new(message).block(Block::default().title("Chats").borders(Borders::ALL));
    frame.render_widget(message, area);
}

fn chat_list_item(chat: &ChatSummary) -> ListItem<'static> {
    ListItem::new(chat_list_item_text(chat))
}

fn chat_list_item_text(chat: &ChatSummary) -> String {
    let unread = if chat.unread_count > 0 {
        format!(" [{}]", chat.unread_count)
    } else {
        String::new()
    };

    let preview = chat
        .last_message_preview
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .unwrap_or("No messages yet");

    format!("{}{} — {}", chat.title, unread, preview)
}

fn status_line(state: &ShellState) -> String {
    let mode = if state.is_running() {
        "running"
    } else {
        "stopping"
    };
    let connectivity = state.connectivity_status().as_label();
    format!("mode: {mode} | connectivity: {connectivity} | r: refresh | q/Ctrl+C: quit")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::events::ConnectivityStatus;

    fn chat(chat_id: i64, title: &str, unread_count: u32, preview: Option<&str>) -> ChatSummary {
        ChatSummary {
            chat_id,
            title: title.to_owned(),
            unread_count,
            last_message_preview: preview.map(ToOwned::to_owned),
            last_message_unix_ms: None,
        }
    }

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

    #[test]
    fn chat_list_item_includes_unread_counter_and_preview() {
        let line = chat_list_item_text(&chat(1, "General", 3, Some("Hello")));

        assert_eq!(line, "General [3] — Hello");
    }

    #[test]
    fn chat_list_item_falls_back_to_placeholder_preview() {
        let line = chat_list_item_text(&chat(1, "General", 0, Some("  ")));

        assert_eq!(line, "General — No messages yet");
    }
}
