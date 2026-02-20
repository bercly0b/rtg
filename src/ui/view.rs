use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::domain::{
    chat::ChatSummary, chat_list_state::ChatListUiState, message::Message,
    open_chat_state::OpenChatUiState, shell_state::ShellState,
};

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
    render_messages_panel(frame, messages_area, state);

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
        .map(normalize_preview_for_chat_row)
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "No messages yet".to_owned());

    format!("{}{} — {}", chat.title, unread, preview)
}

fn normalize_preview_for_chat_row(preview: &str) -> String {
    preview.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn render_messages_panel(frame: &mut Frame<'_>, area: ratatui::layout::Rect, state: &ShellState) {
    let open_chat = state.open_chat();
    let title = open_chat_title(open_chat);

    match open_chat.ui_state() {
        OpenChatUiState::Empty => {
            let panel = Paragraph::new("Select a chat to view messages")
                .block(Block::default().title(title).borders(Borders::ALL));
            frame.render_widget(panel, area);
        }
        OpenChatUiState::Loading => {
            let panel = Paragraph::new("Loading messages...")
                .block(Block::default().title(title).borders(Borders::ALL));
            frame.render_widget(panel, area);
        }
        OpenChatUiState::Error => {
            let panel = Paragraph::new("Failed to load messages. Press Enter to retry.")
                .block(Block::default().title(title).borders(Borders::ALL));
            frame.render_widget(panel, area);
        }
        OpenChatUiState::Ready => {
            let messages = open_chat.messages();
            if messages.is_empty() {
                let panel = Paragraph::new("No messages in this chat")
                    .block(Block::default().title(title).borders(Borders::ALL));
                frame.render_widget(panel, area);
            } else {
                let items: Vec<ListItem<'static>> =
                    messages.iter().map(message_list_item).collect();

                let list =
                    List::new(items).block(Block::default().title(title).borders(Borders::ALL));

                frame.render_widget(list, area);
            }
        }
    }
}

fn open_chat_title(open_chat: &crate::domain::open_chat_state::OpenChatState) -> String {
    if open_chat.is_open() {
        format!("Messages — {}", open_chat.chat_title())
    } else {
        "Messages".to_owned()
    }
}

fn message_list_item(message: &Message) -> ListItem<'static> {
    let time = format_timestamp(message.timestamp_ms);
    let prefix = if message.is_outgoing {
        "You"
    } else {
        &message.sender_name
    };
    let text = message.text.lines().next().unwrap_or("");

    ListItem::new(format!("[{}] {}: {}", time, prefix, text))
}

fn format_timestamp(timestamp_ms: i64) -> String {
    use std::time::UNIX_EPOCH;

    let duration = UNIX_EPOCH + std::time::Duration::from_millis(timestamp_ms as u64);
    let datetime: chrono::DateTime<chrono::Local> = duration.into();

    datetime.format("%H:%M").to_string()
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

    fn message(id: i32, sender: &str, text: &str, outgoing: bool) -> Message {
        Message {
            id,
            sender_name: sender.to_owned(),
            text: text.to_owned(),
            timestamp_ms: 1000,
            is_outgoing: outgoing,
        }
    }

    fn format_message_text(message: &Message) -> String {
        let time = format_timestamp(message.timestamp_ms);
        let prefix = if message.is_outgoing {
            "You"
        } else {
            &message.sender_name
        };
        let text = message.text.lines().next().unwrap_or("");
        format!("[{}] {}: {}", time, prefix, text)
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
        let line = chat_list_item_text(&chat(1, "General", 0, Some("  \n\t  ")));

        assert_eq!(line, "General — No messages yet");
    }

    #[test]
    fn chat_list_item_replaces_newlines_with_spaces() {
        let line = chat_list_item_text(&chat(1, "General", 0, Some("Hello\nworld\r\n!")));

        assert_eq!(line, "General — Hello world !");
    }

    #[test]
    fn chat_list_item_normalizes_redundant_whitespace_to_one_line() {
        let line = chat_list_item_text(&chat(1, "General", 0, Some("  Hello\n\n  from\t\tRTG   ")));

        assert_eq!(line, "General — Hello from RTG");
    }

    #[test]
    fn message_list_item_formats_outgoing_message() {
        let msg = message(1, "User", "Hello world", true);
        let text = format_message_text(&msg);

        assert!(text.contains("["));
        assert!(text.contains("] You: Hello world"));
    }

    #[test]
    fn message_list_item_formats_incoming_message() {
        let msg = message(1, "Alice", "Hi there", false);
        let text = format_message_text(&msg);

        assert!(text.contains("] Alice: Hi there"));
    }

    #[test]
    fn message_list_item_truncates_to_first_line() {
        let msg = message(1, "User", "Line 1\nLine 2\nLine 3", false);
        let text = format_message_text(&msg);

        assert!(text.contains("Line 1"));
        assert!(!text.contains("Line 2"));
    }

    #[test]
    fn open_chat_title_empty_when_no_chat_selected() {
        let state = ShellState::default();

        let title = open_chat_title(state.open_chat());

        assert_eq!(title, "Messages");
    }

    #[test]
    fn open_chat_title_includes_chat_name_when_open() {
        let mut state = ShellState::default();
        state.open_chat_mut().set_loading(1, "General".to_owned());

        let title = open_chat_title(state.open_chat());

        assert_eq!(title, "Messages — General");
    }
}
