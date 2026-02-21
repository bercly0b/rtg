use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::domain::{
    chat::ChatSummary,
    chat_list_state::ChatListUiState,
    open_chat_state::OpenChatUiState,
    shell_state::{ActivePane, ShellState},
};

use super::message_input::render_message_input;
use super::message_rendering::{
    build_message_list_elements, element_to_list_item, message_index_to_element_index,
};
use super::styles;

pub fn render(frame: &mut Frame<'_>, state: &mut ShellState) {
    let [content_area, status_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .areas(frame.area());

    let [chats_area, messages_with_input_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .areas(content_area);

    // Split right panel into messages area and input field (3 lines for input: 1 border + 1 text + 1 border)
    let [messages_area, input_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .areas(messages_with_input_area);

    let active_pane = state.active_pane();
    render_chat_list_panel(frame, chats_area, state, active_pane);
    render_messages_panel(frame, messages_area, state, active_pane);
    render_message_input(frame, input_area, state.message_input(), active_pane);

    let status = Paragraph::new(status_line(state));
    frame.render_widget(status, status_area);
}

fn render_chat_list_panel(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    state: &ShellState,
    active_pane: ActivePane,
) {
    let is_active = active_pane == ActivePane::ChatList;
    let border_style = if is_active {
        styles::active_panel_border_style()
    } else {
        styles::inactive_panel_border_style()
    };

    let chat_list = state.chat_list();
    match chat_list.ui_state() {
        ChatListUiState::Loading => {
            render_chat_list_message(frame, area, "Loading chats...", border_style)
        }
        ChatListUiState::Empty => render_chat_list_message(
            frame,
            area,
            "No chats yet. Press refresh to try again.",
            border_style,
        ),
        ChatListUiState::Error => render_chat_list_message(
            frame,
            area,
            "Failed to load chats. Check connection and retry.",
            border_style,
        ),
        ChatListUiState::Ready => {
            let chats = chat_list.chats();
            // Inner width = area width - 2 (borders)
            let inner_width = area.width.saturating_sub(2) as usize;
            let items = build_chat_list_items(chats, inner_width);
            let chat_count = chats.len();

            let title = format!("Chats ({})", chat_count);
            let list = List::new(items)
                .block(
                    Block::default()
                        .title(title)
                        .borders(Borders::ALL)
                        .border_style(border_style),
                )
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

fn render_chat_list_message(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    message: &str,
    border_style: Style,
) {
    let message = Paragraph::new(message).block(
        Block::default()
            .title("Chats")
            .borders(Borders::ALL)
            .border_style(border_style),
    );
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
    use crate::domain::chat::ChatType;

    let timestamp = chat
        .last_message_unix_ms
        .map(format_chat_timestamp)
        .unwrap_or_else(|| "     ".to_owned());

    let raw_preview = chat
        .last_message_preview
        .as_deref()
        .map(normalize_preview_for_chat_row)
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "No messages yet".to_owned());

    // Build prefix segments (read indicator, sender name, etc.)
    let prefix_segments = build_preview_prefix_segments(chat);
    let prefix_total_len = prefix_segments_len(&prefix_segments);

    // Build suffix badges: unread count and online indicator
    let unread_badge = if chat.unread_count > 0 {
        format!(" [{}]", chat.unread_count)
    } else {
        String::new()
    };

    let online_indicator = if chat.chat_type == ChatType::Private && chat.is_online == Some(true) {
        " \u{25CF}" // Unicode filled circle
    } else {
        ""
    };

    let suffix = format!("{}{}", unread_badge, online_indicator);

    // Calculate layout
    let fixed_prefix_len = 5 + 3; // timestamp (5) + " | " (3)
    let suffix_len = suffix.chars().count();
    let name_len = chat.title.chars().count();

    // Total = fixed_prefix + name + 1 (space) + prefix_segments + preview + padding + suffix
    let content_len = fixed_prefix_len + name_len + 1 + prefix_total_len;
    let available_for_preview_and_padding = width.saturating_sub(content_len + suffix_len);

    // Truncate preview if needed and calculate padding
    let preview_chars: Vec<char> = raw_preview.chars().collect();
    let (display_preview, padding) = if preview_chars.len() <= available_for_preview_and_padding {
        let pad = available_for_preview_and_padding.saturating_sub(preview_chars.len());
        (raw_preview, pad)
    } else {
        // Truncate preview with ellipsis
        let max_preview = available_for_preview_and_padding.saturating_sub(3);
        let truncated: String = preview_chars.iter().take(max_preview).collect();
        (format!("{}...", truncated), 0)
    };

    // Build spans
    let mut spans = vec![
        Span::styled(format!("{:>5}", timestamp), styles::timestamp_style()),
        Span::styled(" | ", styles::separator_style()),
        Span::styled(chat.title.clone(), styles::chat_name_style()),
        Span::raw(" "),
    ];

    // Add prefix segments with their individual styles
    for segment in prefix_segments {
        spans.push(Span::styled(segment.text, segment.style));
    }

    // Add the preview text
    spans.push(Span::styled(display_preview, styles::chat_preview_style()));

    if padding > 0 {
        spans.push(Span::raw(" ".repeat(padding)));
    }

    // Add unread badge
    if !unread_badge.is_empty() {
        spans.push(Span::styled(unread_badge, styles::unread_count_style()));
    }

    // Add online indicator
    if !online_indicator.is_empty() {
        spans.push(Span::styled(
            online_indicator.to_owned(),
            styles::online_indicator_style(),
        ));
    }

    Line::from(spans)
}

/// A styled segment of the preview prefix.
struct PrefixSegment {
    text: String,
    style: Style,
}

/// Builds the prefix segments for the preview text based on chat type and message status.
/// Returns a vector of styled segments that should be prepended to the preview.
/// Order: sender name first (for groups), then read indicator (for outgoing messages).
fn build_preview_prefix_segments(chat: &ChatSummary) -> Vec<PrefixSegment> {
    use crate::domain::chat::ChatType;

    let mut segments = Vec::new();

    // Add sender name for group chats (comes first)
    if chat.chat_type == ChatType::Group {
        if let Some(ref sender) = chat.last_message_sender {
            segments.push(PrefixSegment {
                text: format!("{}: ", sender),
                style: styles::group_sender_style(),
            });
        }
    }

    // Add read status indicator for outgoing messages (all chat types, after sender name)
    if chat.outgoing_status.is_outgoing {
        let (text, style) = if chat.outgoing_status.is_read {
            ("\u{2713} ", styles::outgoing_read_style()) // checkmark
        } else {
            ("\u{2022} ", styles::outgoing_unread_style()) // bullet
        };
        segments.push(PrefixSegment {
            text: text.to_owned(),
            style,
        });
    }

    segments
}

/// Calculates the total character length of all prefix segments.
fn prefix_segments_len(segments: &[PrefixSegment]) -> usize {
    segments.iter().map(|s| s.text.chars().count()).sum()
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

fn render_messages_panel(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    state: &mut ShellState,
    active_pane: ActivePane,
) {
    let is_active = active_pane == ActivePane::Messages;
    let border_style = if is_active {
        styles::active_panel_border_style()
    } else {
        styles::inactive_panel_border_style()
    };

    let open_chat = state.open_chat();
    let title = open_chat_title(open_chat);
    let ui_state = open_chat.ui_state();

    match ui_state {
        OpenChatUiState::Empty => {
            let panel = Paragraph::new("Select a chat to view messages").block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(border_style),
            );
            frame.render_widget(panel, area);
        }
        OpenChatUiState::Loading => {
            let panel = Paragraph::new("Loading messages...").block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(border_style),
            );
            frame.render_widget(panel, area);
        }
        OpenChatUiState::Error => {
            let panel = Paragraph::new("Failed to load messages. Press Enter to retry.").block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(border_style),
            );
            frame.render_widget(panel, area);
        }
        OpenChatUiState::Ready => {
            let messages = state.open_chat().messages();
            if messages.is_empty() {
                let panel = Paragraph::new("No messages in this chat").block(
                    Block::default()
                        .title(title)
                        .borders(Borders::ALL)
                        .border_style(border_style),
                );
                frame.render_widget(panel, area);
            } else {
                let elements = build_message_list_elements(messages);
                let items: Vec<ListItem<'static>> =
                    elements.iter().map(element_to_list_item).collect();

                // Calculate viewport height (area height minus borders)
                let viewport_height = area.height.saturating_sub(2) as usize;

                // Map message index to element index (accounting for date separators)
                let element_index = state
                    .open_chat()
                    .selected_index()
                    .and_then(|msg_idx| message_index_to_element_index(&elements, msg_idx));

                // Update scroll offset based on selection and viewport
                if let Some(idx) = element_index {
                    state
                        .open_chat_mut()
                        .update_scroll_offset(idx, viewport_height);
                }

                let scroll_offset = state.open_chat().scroll_offset();

                let list = List::new(items)
                    .block(
                        Block::default()
                            .title(title)
                            .borders(Borders::ALL)
                            .border_style(border_style),
                    )
                    .highlight_style(
                        Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD),
                    );

                let mut list_state = ListState::default();
                list_state.select(element_index);
                *list_state.offset_mut() = scroll_offset;
                frame.render_stateful_widget(list, area, &mut list_state);
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

fn status_line(state: &ShellState) -> String {
    let mode = if state.is_running() {
        "running"
    } else {
        "stopping"
    };
    let connectivity = state.connectivity_status().as_label();
    let nav_hint = match state.active_pane() {
        ActivePane::ChatList => "j/k: navigate | l/Enter: open chat | r: refresh | q: quit",
        ActivePane::Messages => "j/k: navigate | i: compose | h/Esc: back to chats | q: quit",
        ActivePane::MessageInput => "Esc: cancel | type your message",
    };
    format!("mode: {mode} | connectivity: {connectivity} | {nav_hint}")
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
        use crate::domain::chat::{ChatType, OutgoingReadStatus};
        ChatSummary {
            chat_id,
            title: title.to_owned(),
            unread_count,
            last_message_preview: preview.map(ToOwned::to_owned),
            last_message_unix_ms: None,
            is_pinned,
            chat_type: ChatType::Private,
            last_message_sender: None,
            is_online: None,
            outgoing_status: OutgoingReadStatus::default(),
        }
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

    // =========================================================================
    // Tests for new chat list features
    // =========================================================================

    fn group_chat(
        chat_id: i64,
        title: &str,
        preview: Option<&str>,
        sender: Option<&str>,
    ) -> ChatSummary {
        use crate::domain::chat::{ChatType, OutgoingReadStatus};
        ChatSummary {
            chat_id,
            title: title.to_owned(),
            unread_count: 0,
            last_message_preview: preview.map(ToOwned::to_owned),
            last_message_unix_ms: None,
            is_pinned: false,
            chat_type: ChatType::Group,
            last_message_sender: sender.map(ToOwned::to_owned),
            is_online: None,
            outgoing_status: OutgoingReadStatus::default(),
        }
    }

    fn private_chat_online(
        chat_id: i64,
        title: &str,
        preview: Option<&str>,
        is_online: bool,
    ) -> ChatSummary {
        use crate::domain::chat::{ChatType, OutgoingReadStatus};
        ChatSummary {
            chat_id,
            title: title.to_owned(),
            unread_count: 0,
            last_message_preview: preview.map(ToOwned::to_owned),
            last_message_unix_ms: None,
            is_pinned: false,
            chat_type: ChatType::Private,
            last_message_sender: None,
            is_online: Some(is_online),
            outgoing_status: OutgoingReadStatus::default(),
        }
    }

    fn private_chat_outgoing(
        chat_id: i64,
        title: &str,
        preview: Option<&str>,
        is_read: bool,
    ) -> ChatSummary {
        use crate::domain::chat::{ChatType, OutgoingReadStatus};
        ChatSummary {
            chat_id,
            title: title.to_owned(),
            unread_count: 0,
            last_message_preview: preview.map(ToOwned::to_owned),
            last_message_unix_ms: None,
            is_pinned: false,
            chat_type: ChatType::Private,
            last_message_sender: None,
            is_online: None,
            outgoing_status: OutgoingReadStatus {
                is_outgoing: true,
                is_read,
            },
        }
    }

    #[test]
    fn group_chat_shows_sender_name_before_preview() {
        let line = chat_list_item_line(
            &group_chat(1, "Dev Team", Some("Fixed the bug"), Some("Alex")),
            TEST_WIDTH,
        );
        let text = line_to_string(&line);

        assert!(text.contains("Dev Team"));
        assert!(text.contains("Alex: "));
        assert!(text.contains("Fixed the bug"));
    }

    #[test]
    fn group_chat_without_sender_shows_plain_preview() {
        let line = chat_list_item_line(
            &group_chat(1, "Dev Team", Some("Hello everyone"), None),
            TEST_WIDTH,
        );
        let text = line_to_string(&line);

        assert!(text.contains("Hello everyone"));
        assert!(!text.contains(": "));
    }

    fn group_chat_outgoing(
        chat_id: i64,
        title: &str,
        preview: Option<&str>,
        sender: Option<&str>,
        is_read: bool,
    ) -> ChatSummary {
        use crate::domain::chat::{ChatType, OutgoingReadStatus};
        ChatSummary {
            chat_id,
            title: title.to_owned(),
            unread_count: 0,
            last_message_preview: preview.map(ToOwned::to_owned),
            last_message_unix_ms: None,
            is_pinned: false,
            chat_type: ChatType::Group,
            last_message_sender: sender.map(ToOwned::to_owned),
            is_online: None,
            outgoing_status: OutgoingReadStatus {
                is_outgoing: true,
                is_read,
            },
        }
    }

    #[test]
    fn group_chat_outgoing_unread_shows_sender_then_bullet() {
        let line = chat_list_item_line(
            &group_chat_outgoing(1, "Dev Team", Some("I fixed it"), Some("You"), false),
            70, // wider to fit all content
        );
        let text = line_to_string(&line);

        assert!(text.contains("Dev Team"));
        assert!(text.contains("You: ")); // sender name comes first
        assert!(text.contains("\u{2022} ")); // bullet indicator after sender
        assert!(text.contains("I fixed it"));
        // Verify order: sender before bullet
        let sender_pos = text.find("You: ").unwrap();
        let bullet_pos = text.find("\u{2022}").unwrap();
        assert!(
            sender_pos < bullet_pos,
            "Sender name should come before read indicator"
        );
    }

    #[test]
    fn group_chat_outgoing_read_shows_sender_then_checkmark() {
        let line = chat_list_item_line(
            &group_chat_outgoing(1, "Dev Team", Some("Done"), Some("You"), true),
            70,
        );
        let text = line_to_string(&line);

        assert!(text.contains("Dev Team"));
        assert!(text.contains("You: ")); // sender name comes first
        assert!(text.contains("\u{2713} ")); // checkmark indicator after sender
        assert!(text.contains("Done"));
        // Verify order: sender before checkmark
        let sender_pos = text.find("You: ").unwrap();
        let check_pos = text.find("\u{2713}").unwrap();
        assert!(
            sender_pos < check_pos,
            "Sender name should come before read indicator"
        );
    }

    fn channel_chat_outgoing(
        chat_id: i64,
        title: &str,
        preview: Option<&str>,
        is_read: bool,
    ) -> ChatSummary {
        use crate::domain::chat::{ChatType, OutgoingReadStatus};
        ChatSummary {
            chat_id,
            title: title.to_owned(),
            unread_count: 0,
            last_message_preview: preview.map(ToOwned::to_owned),
            last_message_unix_ms: None,
            is_pinned: false,
            chat_type: ChatType::Channel,
            last_message_sender: None,
            is_online: None,
            outgoing_status: OutgoingReadStatus {
                is_outgoing: true,
                is_read,
            },
        }
    }

    #[test]
    fn channel_outgoing_shows_read_indicator() {
        let line = chat_list_item_line(
            &channel_chat_outgoing(1, "My Channel", Some("New post"), true),
            TEST_WIDTH,
        );
        let text = line_to_string(&line);

        assert!(text.contains("My Channel"));
        assert!(text.contains("\u{2713} ")); // checkmark
        assert!(text.contains("New post"));
    }

    #[test]
    fn channel_outgoing_unread_shows_bullet() {
        let line = chat_list_item_line(
            &channel_chat_outgoing(1, "My Channel", Some("Draft post"), false),
            TEST_WIDTH,
        );
        let text = line_to_string(&line);

        assert!(text.contains("My Channel"));
        assert!(text.contains("\u{2022} ")); // bullet
        assert!(text.contains("Draft post"));
    }

    #[test]
    fn private_chat_online_shows_green_dot() {
        let line = chat_list_item_line(
            &private_chat_online(1, "John", Some("Hey there"), true),
            TEST_WIDTH,
        );
        let text = line_to_string(&line);

        assert!(text.contains("John"));
        assert!(text.contains("Hey there"));
        assert!(text.contains("\u{25CF}")); // Unicode filled circle
    }

    #[test]
    fn private_chat_offline_no_dot() {
        let line = chat_list_item_line(
            &private_chat_online(1, "John", Some("Hey there"), false),
            TEST_WIDTH,
        );
        let text = line_to_string(&line);

        assert!(text.contains("John"));
        assert!(!text.contains("\u{25CF}"));
    }

    #[test]
    fn private_chat_outgoing_unread_shows_bullet() {
        let line = chat_list_item_line(
            &private_chat_outgoing(1, "Jane", Some("See you tomorrow"), false),
            TEST_WIDTH,
        );
        let text = line_to_string(&line);

        assert!(text.contains("Jane"));
        assert!(text.contains("\u{2022} ")); // bullet + space
        assert!(text.contains("See you tomorrow"));
    }

    #[test]
    fn private_chat_outgoing_read_shows_checkmark() {
        let line = chat_list_item_line(
            &private_chat_outgoing(1, "Jane", Some("Got it"), true),
            TEST_WIDTH,
        );
        let text = line_to_string(&line);

        assert!(text.contains("Jane"));
        assert!(text.contains("\u{2713} ")); // checkmark + space
        assert!(text.contains("Got it"));
    }

    #[test]
    fn private_chat_incoming_message_no_indicator() {
        let line = chat_list_item_line(&chat(1, "Bob", 0, Some("Hello!")), TEST_WIDTH);
        let text = line_to_string(&line);

        assert!(text.contains("Hello!"));
        assert!(!text.contains("\u{2022}")); // no bullet
        assert!(!text.contains("\u{2713}")); // no checkmark
    }

    #[test]
    fn chat_with_unread_and_online_shows_both() {
        use crate::domain::chat::{ChatType, OutgoingReadStatus};
        let chat = ChatSummary {
            chat_id: 1,
            title: "Alice".to_owned(),
            unread_count: 5,
            last_message_preview: Some("New message".to_owned()),
            last_message_unix_ms: None,
            is_pinned: false,
            chat_type: ChatType::Private,
            last_message_sender: None,
            is_online: Some(true),
            outgoing_status: OutgoingReadStatus::default(),
        };

        let line = chat_list_item_line(&chat, 70);
        let text = line_to_string(&line);

        assert!(text.contains("[5]"));
        assert!(text.contains("\u{25CF}"));
    }
}
