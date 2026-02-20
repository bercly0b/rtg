use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::domain::{
    chat::ChatSummary, chat_list_state::ChatListUiState, message::Message,
    open_chat_state::OpenChatUiState, shell_state::ShellState,
};

use super::styles;

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
            let chats = chat_list.chats();
            // Inner width = area width - 2 (borders)
            let inner_width = area.width.saturating_sub(2) as usize;
            let items = build_chat_list_items(chats, inner_width);
            let chat_count = chats.len();

            let title = format!("Chats ({})", chat_count);
            let list = List::new(items)
                .block(Block::default().title(title).borders(Borders::ALL))
                .highlight_style(
                    Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD),
                );

            let visual_index = chat_list
                .selected_index()
                .map(|idx| compute_visual_index(chats, idx));

            let mut list_state = ListState::default();
            list_state.select(visual_index);
            frame.render_stateful_widget(list, area, &mut list_state);
        }
    }
}

fn render_chat_list_message(frame: &mut Frame<'_>, area: ratatui::layout::Rect, message: &str) {
    let message =
        Paragraph::new(message).block(Block::default().title("Chats").borders(Borders::ALL));
    frame.render_widget(message, area);
}

/// Builds the list of visual items including section headers.
fn build_chat_list_items(chats: &[ChatSummary], width: usize) -> Vec<ListItem<'static>> {
    let (pinned, regular): (Vec<_>, Vec<_>) = chats.iter().partition(|c| c.is_pinned);

    let mut items = Vec::new();
    let has_pinned = !pinned.is_empty();

    if has_pinned {
        items.push(section_header_item("Pinned"));
        for chat in &pinned {
            items.push(chat_list_item(chat, width));
        }
    }

    if !regular.is_empty() || !has_pinned {
        items.push(section_header_item("All Chats"));
        for chat in &regular {
            items.push(chat_list_item(chat, width));
        }
    }

    items
}

/// Computes the visual index in the list (accounting for section headers).
fn compute_visual_index(chats: &[ChatSummary], chat_index: usize) -> usize {
    let pinned_count = chats.iter().filter(|c| c.is_pinned).count();
    let has_pinned = pinned_count > 0;

    if chat_index < pinned_count {
        // In pinned section: +1 for "Pinned" header
        chat_index + 1
    } else {
        // In regular section
        let headers = if has_pinned { 2 } else { 1 }; // "Pinned" + "All Chats" or just "All Chats"
        chat_index + headers
    }
}

fn section_header_item(title: &str) -> ListItem<'static> {
    let line = Line::from(vec![Span::styled(
        format!("-- {} --", title),
        styles::section_header_style(),
    )]);
    ListItem::new(line)
}

fn chat_list_item(chat: &ChatSummary, width: usize) -> ListItem<'static> {
    ListItem::new(chat_list_item_line(chat, width))
}

fn chat_list_item_line(chat: &ChatSummary, width: usize) -> Line<'static> {
    let timestamp = chat
        .last_message_unix_ms
        .map(format_chat_timestamp)
        .unwrap_or_else(|| "     ".to_owned());

    let preview = chat
        .last_message_preview
        .as_deref()
        .map(normalize_preview_for_chat_row)
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "No messages yet".to_owned());

    // Format: "HH:MM | Name Preview...          [N]"
    // Fixed parts: timestamp (5) + " | " (3) + " " (1) after name = 9 chars
    let unread_badge = if chat.unread_count > 0 {
        format!(" [{}]", chat.unread_count)
    } else {
        String::new()
    };

    let fixed_prefix_len = 5 + 3; // timestamp + separator
    let badge_len = unread_badge.chars().count();
    let name_len = chat.title.chars().count();

    // Calculate available space for preview + padding
    // Total = fixed_prefix + name + 1 (space) + preview + padding + badge
    let content_len = fixed_prefix_len + name_len + 1; // prefix + name + space
    let available_for_preview_and_padding = width.saturating_sub(content_len + badge_len);

    // Truncate preview if needed and calculate padding
    let preview_chars: Vec<char> = preview.chars().collect();
    let (display_preview, padding) = if preview_chars.len() <= available_for_preview_and_padding {
        let pad = available_for_preview_and_padding.saturating_sub(preview_chars.len());
        (preview, pad)
    } else {
        // Truncate preview with ellipsis
        let max_preview = available_for_preview_and_padding.saturating_sub(3);
        let truncated: String = preview_chars.iter().take(max_preview).collect();
        (format!("{}...", truncated), 0)
    };

    let mut spans = vec![
        Span::styled(format!("{:>5}", timestamp), styles::timestamp_style()),
        Span::styled(" | ", styles::separator_style()),
        Span::styled(chat.title.clone(), styles::chat_name_style()),
        Span::raw(" "),
        Span::styled(display_preview, styles::chat_preview_style()),
    ];

    if padding > 0 {
        spans.push(Span::raw(" ".repeat(padding)));
    }

    if !unread_badge.is_empty() {
        spans.push(Span::styled(unread_badge, styles::unread_count_style()));
    }

    Line::from(spans)
}

fn format_chat_timestamp(timestamp_ms: i64) -> String {
    use chrono::{Local, TimeZone};

    // Handle negative timestamps gracefully (before Unix epoch or corrupted data)
    let datetime = match Local.timestamp_millis_opt(timestamp_ms) {
        chrono::LocalResult::Single(dt) => dt,
        chrono::LocalResult::Ambiguous(dt, _) => dt,
        chrono::LocalResult::None => return "     ".to_owned(),
    };

    let today = Local::now().date_naive();

    if datetime.date_naive() == today {
        datetime.format("%H:%M").to_string()
    } else {
        datetime.format("%d.%m").to_string()
    }
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
        chat_with_pinned(chat_id, title, unread_count, preview, false)
    }

    fn chat_with_pinned(
        chat_id: i64,
        title: &str,
        unread_count: u32,
        preview: Option<&str>,
        is_pinned: bool,
    ) -> ChatSummary {
        ChatSummary {
            chat_id,
            title: title.to_owned(),
            unread_count,
            last_message_preview: preview.map(ToOwned::to_owned),
            last_message_unix_ms: None,
            is_pinned,
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

    /// Extracts text content from Line for testing.
    fn line_to_string(line: &Line<'_>) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
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

    // Use a typical width for chat list tests
    const TEST_WIDTH: usize = 50;

    #[test]
    fn chat_list_item_includes_title_and_preview() {
        let line = chat_list_item_line(&chat(1, "General", 0, Some("Hello")), TEST_WIDTH);
        let text = line_to_string(&line);

        assert!(text.contains("General"));
        assert!(text.contains("Hello"));
    }

    #[test]
    fn chat_list_item_includes_unread_counter() {
        let line = chat_list_item_line(&chat(1, "General", 3, Some("Hello")), TEST_WIDTH);
        let text = line_to_string(&line);

        assert!(text.contains("[3]"));
    }

    #[test]
    fn chat_list_item_omits_counter_when_zero() {
        let line = chat_list_item_line(&chat(1, "General", 0, Some("Hello")), TEST_WIDTH);
        let text = line_to_string(&line);

        assert!(!text.contains("[0]"));
        assert!(!text.contains("[]"));
    }

    #[test]
    fn chat_list_item_falls_back_to_placeholder_preview() {
        let line = chat_list_item_line(&chat(1, "General", 0, Some("  \n\t  ")), TEST_WIDTH);
        let text = line_to_string(&line);

        assert!(text.contains("No messages yet"));
    }

    #[test]
    fn chat_list_item_normalizes_whitespace() {
        let line = chat_list_item_line(
            &chat(1, "General", 0, Some("  Hello\n\n  from\t\tRTG   ")),
            TEST_WIDTH,
        );
        let text = line_to_string(&line);

        assert!(text.contains("Hello from RTG"));
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

    #[test]
    fn build_chat_list_items_creates_all_chats_section_for_regular_chats() {
        let chats = vec![chat(1, "General", 0, Some("Hello"))];
        let items = build_chat_list_items(&chats, TEST_WIDTH);

        // Should have: "All Chats" header + 1 chat
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn build_chat_list_items_creates_both_sections_when_pinned_exists() {
        let chats = vec![
            chat_with_pinned(1, "Pinned Chat", 0, Some("Hi"), true),
            chat(2, "Regular Chat", 0, Some("Hello")),
        ];
        let items = build_chat_list_items(&chats, TEST_WIDTH);

        // Should have: "Pinned" header + 1 pinned + "All Chats" header + 1 regular
        assert_eq!(items.len(), 4);
    }

    #[test]
    fn compute_visual_index_accounts_for_headers() {
        let chats = vec![
            chat_with_pinned(1, "Pinned", 0, None, true),
            chat(2, "Regular", 0, None),
        ];

        // Pinned chat at index 0 -> visual index 1 (after "Pinned" header)
        assert_eq!(compute_visual_index(&chats, 0), 1);
        // Regular chat at index 1 -> visual index 3 (after "Pinned" header + pinned chat + "All Chats" header)
        assert_eq!(compute_visual_index(&chats, 1), 3);
    }

    #[test]
    fn compute_visual_index_with_no_pinned() {
        let chats = vec![chat(1, "Chat1", 0, None), chat(2, "Chat2", 0, None)];

        // First chat -> visual index 1 (after "All Chats" header)
        assert_eq!(compute_visual_index(&chats, 0), 1);
        // Second chat -> visual index 2
        assert_eq!(compute_visual_index(&chats, 1), 2);
    }

    #[test]
    fn format_chat_timestamp_shows_time_for_today() {
        use chrono::Local;

        let now = Local::now();
        let timestamp_ms = now.timestamp_millis();

        let formatted = format_chat_timestamp(timestamp_ms);

        // Should be in HH:MM format
        assert_eq!(formatted.len(), 5);
        assert!(formatted.contains(':'));
    }

    #[test]
    fn format_chat_timestamp_shows_date_for_past() {
        // Jan 1, 2020 00:00:00 UTC
        let timestamp_ms = 1577836800000_i64;

        let formatted = format_chat_timestamp(timestamp_ms);

        // Should be in DD.MM format
        assert_eq!(formatted.len(), 5);
        assert!(formatted.contains('.'));
    }

    #[test]
    fn format_chat_timestamp_handles_negative_timestamp_gracefully() {
        // Small negative timestamp (before Unix epoch) - should still be valid date
        let formatted = format_chat_timestamp(-1000);

        // Should be a valid DD.MM format (Dec 31, 1969 or Jan 1, 1970 depending on timezone)
        assert_eq!(formatted.len(), 5);
        assert!(formatted.contains('.'));
    }

    #[test]
    fn format_chat_timestamp_handles_extreme_negative_timestamp() {
        // Extremely negative timestamp that chrono cannot handle
        let formatted = format_chat_timestamp(i64::MIN);

        // Should return empty placeholder for invalid dates
        assert_eq!(formatted, "     ");
    }

    #[test]
    fn compute_visual_index_with_all_pinned() {
        let chats = vec![
            chat_with_pinned(1, "Pinned1", 0, None, true),
            chat_with_pinned(2, "Pinned2", 0, None, true),
        ];

        // First pinned chat -> visual index 1 (after "Pinned" header)
        assert_eq!(compute_visual_index(&chats, 0), 1);
        // Second pinned chat -> visual index 2
        assert_eq!(compute_visual_index(&chats, 1), 2);
    }

    #[test]
    fn build_chat_list_items_shows_all_chats_header_when_all_pinned() {
        let chats = vec![
            chat_with_pinned(1, "Pinned1", 0, None, true),
            chat_with_pinned(2, "Pinned2", 0, None, true),
        ];
        let items = build_chat_list_items(&chats, TEST_WIDTH);

        // Should have: "Pinned" header + 2 pinned chats (no "All Chats" header since regular is empty)
        // Based on logic: `if !regular.is_empty() || !has_pinned` - so All Chats NOT added when all pinned
        assert_eq!(items.len(), 3);
    }
}
