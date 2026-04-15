use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Padding, Paragraph},
    Frame,
};

use crate::domain::{
    open_chat_state::{OpenChatUiState, SCROLL_MARGIN},
    shell_state::{ActivePane, ShellState},
};

use crate::ui::chat_message_list::{ChatMessageList, ChatMessageListState};
use crate::ui::message_rendering::{
    build_message_list_elements, element_to_text, message_index_to_element_index,
};
use crate::ui::styles;

use super::panel_title_style;

pub(super) fn render_messages_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &mut ShellState,
    active_pane: ActivePane,
) {
    let is_active = active_pane == ActivePane::Messages;

    let open_chat = state.open_chat();
    let title = open_chat_title(open_chat, is_active);
    let ui_state = open_chat.ui_state();

    let block = || {
        Block::new()
            .title(title.clone())
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

pub(super) fn open_chat_title(
    open_chat: &crate::domain::open_chat_state::OpenChatState,
    is_active: bool,
) -> Line<'static> {
    let title_style = panel_title_style(is_active);

    if !open_chat.is_open() {
        return Line::from(Span::styled("Messages".to_owned(), title_style));
    }

    let name = open_chat.chat_title().to_owned();

    if open_chat.is_refreshing() {
        return Line::from(Span::styled(
            format!("{} \u{00b7} updating...", name),
            title_style,
        ));
    }

    let typing_label = open_chat.typing_state().format_label(open_chat.chat_type());
    if !typing_label.is_empty() {
        return Line::from(vec![
            Span::styled(format!("{} \u{00b7} ", name), title_style),
            Span::styled(typing_label, styles::typing_style()),
        ]);
    }

    let subtitle = open_chat.chat_subtitle();
    let now = chrono::Local::now();
    let subtitle_text = subtitle.format(now);
    if subtitle_text.is_empty() {
        Line::from(Span::styled(name, title_style))
    } else {
        Line::from(Span::styled(
            format!("{} \u{00b7} {}", name, subtitle_text),
            title_style,
        ))
    }
}
