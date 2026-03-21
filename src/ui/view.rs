use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Padding, Paragraph},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::domain::{
    chat::ChatSummary,
    chat_list_state::ChatListUiState,
    open_chat_state::{OpenChatUiState, SCROLL_MARGIN},
    shell_state::{ActivePane, ShellState},
};

use super::chat_message_list::{ChatMessageList, ChatMessageListState};
use super::message_input::render_message_input;
use super::message_rendering::{
    build_message_list_elements, element_to_text, message_index_to_element_index,
};
use super::styles;

pub fn render(frame: &mut Frame<'_>, state: &mut ShellState) {
    let [content_area, status_separator_area, status_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(frame.area());

    // Horizontal split: chat list | separator (1 char) | messages+input
    let [chats_area, separator_area, messages_with_input_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Length(1),
            Constraint::Percentage(70),
        ])
        .areas(content_area);

    // Compute dynamic input height based on text length and available width.
    let input_height =
        compute_input_height(state.message_input().text(), messages_with_input_area.width);

    // Split right panel into messages area, horizontal separator, and input field
    let [messages_area, input_separator_area, input_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(input_height),
        ])
        .areas(messages_with_input_area);

    let active_pane = state.active_pane();
    render_chat_list_panel(frame, chats_area, state, active_pane);
    render_vertical_separator(frame, separator_area);
    render_messages_panel(frame, messages_area, state, active_pane);
    render_horizontal_separator(frame, input_separator_area);
    render_message_input(frame, input_area, state.message_input(), active_pane);

    render_horizontal_separator(frame, status_separator_area);
    let status = Paragraph::new(status_line(state)).style(styles::status_bar_style());
    frame.render_widget(status, status_area);
}

fn render_chat_list_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &ShellState,
    active_pane: ActivePane,
) {
    let is_active = active_pane == ActivePane::ChatList;
    let title_style = panel_title_style(is_active);

    let chat_list = state.chat_list();
    match chat_list.ui_state() {
        ChatListUiState::Loading => {
            render_chat_list_message(frame, area, "Loading chats...", title_style)
        }
        ChatListUiState::Empty => render_chat_list_message(
            frame,
            area,
            "No chats yet. Press refresh to try again.",
            title_style,
        ),
        ChatListUiState::Error => render_chat_list_message(
            frame,
            area,
            "Failed to load chats. Check connection and retry.",
            title_style,
        ),
        ChatListUiState::Ready => {
            let chats = chat_list.chats();
            // Inner width = area width - 2 (horizontal padding)
            let inner_width = area.width.saturating_sub(2) as usize;
            let items = build_chat_list_items(chats, inner_width);
            let chat_count = chats.len();

            let title = format!("Chats ({})", chat_count);
            let highlight = if is_active {
                styles::highlight_style()
            } else {
                Style::default()
            };

            let list = List::new(items)
                .block(
                    Block::new()
                        .title(title)
                        .title_style(title_style)
                        .title_alignment(Alignment::Center)
                        .padding(Padding::horizontal(1)),
                )
                .highlight_style(highlight);

            let visual_index = if is_active {
                chat_list
                    .selected_index()
                    .map(|idx| compute_visual_index(chats, idx))
            } else {
                None
            };

            let mut list_state = ListState::default();
            list_state.select(visual_index);
            frame.render_stateful_widget(list, area, &mut list_state);
        }
    }
}

fn render_chat_list_message(frame: &mut Frame<'_>, area: Rect, message: &str, title_style: Style) {
    let message = Paragraph::new(message).block(
        Block::new()
            .title("Chats")
            .title_style(title_style)
            .title_alignment(Alignment::Center)
            .padding(Padding::horizontal(1)),
    );
    frame.render_widget(message, area);
}

/// Renders a vertical separator line between panels.
fn render_vertical_separator(frame: &mut Frame<'_>, area: Rect) {
    let sep_style = styles::panel_separator_style();
    let lines: Vec<Line<'_>> = (0..area.height)
        .map(|_| Line::styled("\u{2502}", sep_style))
        .collect();
    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}

fn render_horizontal_separator(frame: &mut Frame<'_>, area: Rect) {
    let sep_style = styles::panel_separator_style();
    let line_str: String = "\u{2500}".repeat(area.width as usize);
    let paragraph = Paragraph::new(Line::styled(line_str, sep_style));
    frame.render_widget(paragraph, area);
}

/// Returns the appropriate title style for a panel based on active state.
fn panel_title_style(is_active: bool) -> Style {
    if is_active {
        styles::active_title_style()
    } else {
        styles::inactive_title_style()
    }
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

    // Build prefix segments (sender name for groups)
    let prefix_segments = build_preview_prefix_segments(chat);
    let prefix_total_width = prefix_segments_width(&prefix_segments);

    // Build suffix components: outgoing status, unread count, online indicator
    let outgoing_suffix = build_outgoing_status_suffix(chat);
    let outgoing_suffix_width = outgoing_suffix
        .as_ref()
        .map(|(t, _)| t.width())
        .unwrap_or(0);

    let unread_badge = if chat.unread_count > 0 {
        format!(" [{}]", chat.unread_count)
    } else {
        String::new()
    };

    let online_indicator =
        if chat.chat_type == ChatType::Private && !chat.is_bot && chat.is_online == Some(true) {
            " \u{2022}" // bullet
        } else {
            ""
        };

    // Calculate layout using display widths (handles emoji and wide chars correctly)
    let fixed_prefix_width = 5 + 3; // timestamp (5) + " | " (3)
    let suffix_width = outgoing_suffix_width + unread_badge.width() + online_indicator.width();
    let name_width = chat.title.width();

    // Total = fixed_prefix + name + 1 (space) + prefix_segments + preview + padding + suffix
    let content_width = fixed_prefix_width + name_width + 1 + prefix_total_width;
    let available_for_preview_and_padding = width.saturating_sub(content_width + suffix_width);

    // Truncate preview if needed and calculate padding
    let (display_preview, padding) =
        truncate_to_display_width(&raw_preview, available_for_preview_and_padding);

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

    if let Some((text, style)) = outgoing_suffix {
        spans.push(Span::styled(text, style));
    }

    if !online_indicator.is_empty() {
        spans.push(Span::styled(
            online_indicator.to_owned(),
            styles::online_indicator_style(),
        ));
    }

    if !unread_badge.is_empty() {
        spans.push(Span::styled(unread_badge, styles::unread_count_style()));
    }

    Line::from(spans)
}

/// A styled segment of the preview prefix.
struct PrefixSegment {
    text: String,
    style: Style,
}

/// Builds the prefix segments for the preview text based on chat type.
/// Returns a vector of styled segments that should be prepended to the preview.
/// Currently only includes sender name for group chats.
fn build_preview_prefix_segments(chat: &ChatSummary) -> Vec<PrefixSegment> {
    use crate::domain::chat::ChatType;

    let mut segments = Vec::new();

    // Add sender name for group chats
    if chat.chat_type == ChatType::Group {
        if let Some(ref sender) = chat.last_message_sender {
            segments.push(PrefixSegment {
                text: format!("{}: ", sender),
                style: styles::group_sender_style(),
            });
        }
    }

    segments
}

/// Builds the outgoing status suffix segment for the chat list item.
/// Returns `Some((text, style))` for outgoing messages, `None` for incoming.
fn build_outgoing_status_suffix(chat: &ChatSummary) -> Option<(String, Style)> {
    if chat.outgoing_status.is_outgoing {
        let (text, style) = if chat.outgoing_status.is_read {
            (" \u{2713}\u{2713}", styles::outgoing_read_style()) // double checkmark
        } else {
            (" \u{2713}", styles::outgoing_unread_style()) // single checkmark
        };
        Some((text.to_owned(), style))
    } else {
        None
    }
}

/// Calculates the total display width of all prefix segments.
fn prefix_segments_width(segments: &[PrefixSegment]) -> usize {
    segments.iter().map(|s| s.text.width()).sum()
}

/// Truncates a string to fit within a given display width.
///
/// Returns `(display_text, padding)`:
/// - If the text fits, returns the original text with remaining padding.
/// - If it doesn't fit, truncates at a character boundary and appends "...".
///
/// Uses Unicode display width so that emoji and wide characters are measured
/// correctly (e.g. 🚀 counts as 2 cells, not 1).
fn truncate_to_display_width(text: &str, max_width: usize) -> (String, usize) {
    use unicode_width::UnicodeWidthChar;

    let text_width = text.width();
    if text_width <= max_width {
        return (text.to_owned(), max_width.saturating_sub(text_width));
    }

    // If we can't even fit "...", return empty string
    if max_width < 3 {
        return (String::new(), max_width);
    }

    // Need to truncate: reserve 3 cells for "..."
    let target_width = max_width - 3;
    let mut current_width = 0;
    let mut truncated = String::new();

    for ch in text.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if current_width + ch_width > target_width {
            break;
        }
        truncated.push(ch);
        current_width += ch_width;
    }

    (format!("{}...", truncated), 0)
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
    area: Rect,
    state: &mut ShellState,
    active_pane: ActivePane,
) {
    let is_active = active_pane == ActivePane::Messages;
    let title_style = panel_title_style(is_active);

    let open_chat = state.open_chat();
    let title = open_chat_title(open_chat);
    let ui_state = open_chat.ui_state();

    let block = || {
        Block::new()
            .title(title.clone())
            .title_style(title_style)
            .title_alignment(Alignment::Center)
            .padding(Padding::horizontal(1))
    };

    match ui_state {
        OpenChatUiState::Empty => {
            let panel = Paragraph::new("Select a chat to view messages").block(block());
            frame.render_widget(panel, area);
        }
        OpenChatUiState::Loading => {
            let panel = Paragraph::new("Loading messages...").block(block());
            frame.render_widget(panel, area);
        }
        OpenChatUiState::Error => {
            let panel =
                Paragraph::new("Failed to load messages. Press Enter to retry.").block(block());
            frame.render_widget(panel, area);
        }
        OpenChatUiState::Ready => {
            let messages = state.open_chat().messages();
            if messages.is_empty() {
                let panel = Paragraph::new("No messages in this chat").block(block());
                frame.render_widget(panel, area);
            } else {
                let elements = build_message_list_elements(messages);

                // Map message index to element index (accounting for date separators)
                let element_index = state
                    .open_chat()
                    .selected_index()
                    .and_then(|msg_idx| message_index_to_element_index(&elements, msg_idx));

                let highlight = if is_active {
                    styles::highlight_style()
                } else {
                    Style::default()
                };

                let element_index = if is_active { element_index } else { None };

                // Compute available content width for text wrapping.
                // Subtract block padding (1 left + 1 right = 2).
                let content_width = area.width.saturating_sub(2) as usize;

                let texts: Vec<ratatui::text::Text<'static>> = elements
                    .iter()
                    .map(|e| element_to_text(e, content_width))
                    .collect();

                let list = ChatMessageList::new(texts)
                    .block(block())
                    .highlight_style(highlight)
                    .scroll_padding(SCROLL_MARGIN);

                let scroll_offset = state.open_chat().scroll_offset();
                let mut list_state = ChatMessageListState::new(scroll_offset, element_index);
                frame.render_stateful_widget(list, area, &mut list_state);

                // Persist the offset computed by the widget for the next frame.
                // Only update when active to prevent scroll drift when the pane
                // is inactive and has no selected item.
                if is_active {
                    state.open_chat_mut().set_scroll_offset(list_state.offset());
                }
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

/// Computes the dynamic height for the message input area (1 to 5 lines).
fn compute_input_height(text: &str, available_width: u16) -> u16 {
    use unicode_width::UnicodeWidthStr;

    // Account for horizontal padding (1 left + 1 right) and prompt symbol "> "
    let effective_width = available_width.saturating_sub(2 + 2) as usize; // padding + prompt
    if effective_width == 0 || text.is_empty() {
        return 1;
    }

    let text_width = text.width();
    let lines = text_width.div_ceil(effective_width);
    (lines as u16).clamp(1, 5)
}

fn status_line(state: &ShellState) -> String {
    let mode = if state.is_running() {
        "running"
    } else {
        "stopping"
    };
    let connectivity = state.connectivity_status().as_label();
    let nav_hint = match state.active_pane() {
        ActivePane::ChatList => {
            "j/k: navigate | l/Enter: open chat | r: mark read | R: refresh | q: quit"
        }
        ActivePane::Messages => {
            "j/k: navigate | y: copy | i: compose | h/Esc: back to chats | q: quit"
        }
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
            is_bot: false,
            outgoing_status: OutgoingReadStatus::default(),
            last_message_id: None,
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
            is_bot: false,
            outgoing_status: OutgoingReadStatus::default(),
            last_message_id: None,
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
            is_bot: false,
            outgoing_status: OutgoingReadStatus::default(),
            last_message_id: None,
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
            is_bot: false,
            outgoing_status: OutgoingReadStatus {
                is_outgoing: true,
                is_read,
            },
            last_message_id: None,
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
            is_bot: false,
            outgoing_status: OutgoingReadStatus {
                is_outgoing: true,
                is_read,
            },
            last_message_id: None,
        }
    }

    #[test]
    fn group_chat_outgoing_delivered_shows_single_check_after_preview() {
        let line = chat_list_item_line(
            &group_chat_outgoing(1, "Dev Team", Some("I fixed it"), Some("You"), false),
            70, // wider to fit all content
        );
        let text = line_to_string(&line);

        assert!(text.contains("Dev Team"));
        assert!(text.contains("You: ")); // sender name in prefix
        assert!(text.contains(" \u{2713}")); // single checkmark in suffix
        assert!(!text.contains("\u{2713}\u{2713}")); // NOT double checkmark
        assert!(text.contains("I fixed it"));
        // Verify order: preview before checkmark (status is now a suffix)
        let preview_pos = text.find("I fixed it").unwrap();
        let check_pos = text.find("\u{2713}").unwrap();
        assert!(
            preview_pos < check_pos,
            "Preview should come before status indicator"
        );
    }

    #[test]
    fn group_chat_outgoing_read_shows_double_check_after_preview() {
        let line = chat_list_item_line(
            &group_chat_outgoing(1, "Dev Team", Some("Done"), Some("You"), true),
            70,
        );
        let text = line_to_string(&line);

        assert!(text.contains("Dev Team"));
        assert!(text.contains("You: ")); // sender name in prefix
        assert!(text.contains(" \u{2713}\u{2713}")); // double checkmark in suffix
        assert!(text.contains("Done"));
        // Verify order: preview before checkmark (status is now a suffix)
        let preview_pos = text.find("Done").unwrap();
        let check_pos = text.find("\u{2713}").unwrap();
        assert!(
            preview_pos < check_pos,
            "Preview should come before status indicator"
        );
    }

    #[test]
    fn group_chat_outgoing_narrow_width_still_shows_status() {
        // Simulate a narrow chat list panel (30% of 80-col terminal, minus padding)
        let line = chat_list_item_line(
            &group_chat_outgoing(1, "Dev Team", Some("I fixed the bug"), Some("Alex"), true),
            34, // narrow width
        );
        let text = line_to_string(&line);

        assert!(text.contains("Dev Team"));
        assert!(text.contains("Alex: "));
        assert!(
            text.contains("\u{2713}"),
            "Status indicator must be present even at narrow width. Got: '{}'",
            text
        );
    }

    #[test]
    fn group_chat_emoji_in_sender_name_shows_status_indicator() {
        // Regression: emoji in sender name takes 2 terminal cells but .chars().count()
        // treated it as 1, causing the line to overflow and clip the status indicator.
        let line = chat_list_item_line(
            &group_chat_outgoing(
                1,
                "Group",
                Some("hello"),
                Some("\u{1F680} vlad"), // "🚀 vlad" — emoji is 2 cells wide
                true,
            ),
            40,
        );
        let text = line_to_string(&line);

        assert!(
            text.contains("\u{2713}"),
            "Status indicator must be present with emoji sender. Got: '{}'",
            text
        );
    }

    #[test]
    fn group_chat_emoji_in_title_shows_status_indicator() {
        // Emoji in chat title should also be measured by display width
        let line = chat_list_item_line(
            &group_chat_outgoing(
                1,
                "\u{1F525} Fire Chat", // "🔥 Fire Chat" — emoji is 2 cells wide
                Some("done"),
                Some("Alex"),
                true,
            ),
            50,
        );
        let text = line_to_string(&line);

        assert!(
            text.contains("\u{2713}"),
            "Status indicator must be present with emoji title. Got: '{}'",
            text
        );
    }

    #[test]
    fn truncate_to_display_width_fits_ascii() {
        let (text, padding) = truncate_to_display_width("hello", 10);
        assert_eq!(text, "hello");
        assert_eq!(padding, 5);
    }

    #[test]
    fn truncate_to_display_width_truncates_with_ellipsis() {
        let (text, padding) = truncate_to_display_width("hello world", 8);
        assert_eq!(text, "hello...");
        assert_eq!(padding, 0);
    }

    #[test]
    fn truncate_to_display_width_counts_emoji_as_double_width() {
        // "🚀 hi" = 2+1+1+1 = 5 display cells
        let (text, padding) = truncate_to_display_width("\u{1F680} hi", 5);
        assert_eq!(text, "\u{1F680} hi");
        assert_eq!(padding, 0);
    }

    #[test]
    fn truncate_to_display_width_truncates_emoji_correctly() {
        // "🚀🚀🚀" = 6 display cells; max_width=5 → target=2 → "🚀..."
        let (text, padding) = truncate_to_display_width("\u{1F680}\u{1F680}\u{1F680}", 5);
        assert_eq!(text, "\u{1F680}...");
        assert_eq!(padding, 0);
    }

    #[test]
    fn truncate_to_display_width_exact_fit() {
        let (text, padding) = truncate_to_display_width("abc", 3);
        assert_eq!(text, "abc");
        assert_eq!(padding, 0);
    }

    #[test]
    fn truncate_to_display_width_zero_width_returns_empty() {
        let (text, padding) = truncate_to_display_width("hello", 0);
        assert_eq!(text, "");
        assert_eq!(padding, 0);
    }

    #[test]
    fn truncate_to_display_width_less_than_ellipsis_returns_empty() {
        let (text, padding) = truncate_to_display_width("hello", 2);
        assert_eq!(text, "");
        assert_eq!(padding, 2);
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
            is_bot: false,
            outgoing_status: OutgoingReadStatus {
                is_outgoing: true,
                is_read,
            },
            last_message_id: None,
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
        assert!(text.contains(" \u{2713}\u{2713}")); // double checkmark in suffix
        assert!(text.contains("New post"));
    }

    #[test]
    fn channel_outgoing_delivered_shows_single_check() {
        let line = chat_list_item_line(
            &channel_chat_outgoing(1, "My Channel", Some("Draft post"), false),
            TEST_WIDTH,
        );
        let text = line_to_string(&line);

        assert!(text.contains("My Channel"));
        assert!(text.contains(" \u{2713}")); // single checkmark in suffix
        assert!(!text.contains("\u{2713}\u{2713}")); // NOT double
        assert!(text.contains("Draft post"));
    }

    #[test]
    fn private_chat_online_shows_bullet() {
        let line = chat_list_item_line(
            &private_chat_online(1, "John", Some("Hey there"), true),
            TEST_WIDTH,
        );
        let text = line_to_string(&line);

        assert!(text.contains("John"));
        assert!(text.contains("Hey there"));
        assert!(text.contains("\u{2022}")); // bullet
    }

    #[test]
    fn private_chat_offline_no_bullet() {
        let line = chat_list_item_line(
            &private_chat_online(1, "John", Some("Hey there"), false),
            TEST_WIDTH,
        );
        let text = line_to_string(&line);

        assert!(text.contains("John"));
        // Should not contain the online bullet indicator at the end
        // (Note: the text might contain \u{2022} from an outgoing status,
        // but this chat has default outgoing_status so no indicators at all)
        assert!(!text.contains("\u{2022}"));
    }

    #[test]
    fn private_chat_outgoing_delivered_shows_single_check() {
        let line = chat_list_item_line(
            &private_chat_outgoing(1, "Jane", Some("See you tomorrow"), false),
            TEST_WIDTH,
        );
        let text = line_to_string(&line);

        assert!(text.contains("Jane"));
        assert!(text.contains(" \u{2713}")); // space + single checkmark (suffix)
        assert!(!text.contains("\u{2713}\u{2713}")); // NOT double
        assert!(text.contains("See you tomorrow"));
    }

    #[test]
    fn private_chat_outgoing_read_shows_double_check() {
        let line = chat_list_item_line(
            &private_chat_outgoing(1, "Jane", Some("Got it"), true),
            TEST_WIDTH,
        );
        let text = line_to_string(&line);

        assert!(text.contains("Jane"));
        assert!(text.contains(" \u{2713}\u{2713}")); // space + double checkmark (suffix)
        assert!(text.contains("Got it"));
    }

    #[test]
    fn private_chat_incoming_message_no_indicator() {
        let line = chat_list_item_line(&chat(1, "Bob", 0, Some("Hello!")), TEST_WIDTH);
        let text = line_to_string(&line);

        assert!(text.contains("Hello!"));
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
            is_bot: false,
            outgoing_status: OutgoingReadStatus::default(),
            last_message_id: None,
        };

        let line = chat_list_item_line(&chat, 70);
        let text = line_to_string(&line);

        assert!(text.contains("[5]"));
        assert!(text.contains("\u{2022}")); // online bullet
    }

    #[test]
    fn bot_chat_online_does_not_show_online_indicator() {
        use crate::domain::chat::{ChatType, OutgoingReadStatus};
        let chat = ChatSummary {
            chat_id: 1,
            title: "BotName".to_owned(),
            unread_count: 0,
            last_message_preview: Some("Hello".to_owned()),
            last_message_unix_ms: None,
            is_pinned: false,
            chat_type: ChatType::Private,
            last_message_sender: None,
            is_online: Some(true),
            is_bot: true,
            outgoing_status: OutgoingReadStatus::default(),
            last_message_id: None,
        };

        let line = chat_list_item_line(&chat, 70);
        let text = line_to_string(&line);

        assert!(
            !text.contains("\u{2022}"),
            "online bullet must not appear for bots"
        );
    }
}
