use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Padding, Paragraph},
    Frame,
};

use crate::domain::{
    chat::ChatSummary,
    chat_list_state::ChatListUiState,
    forum_topic_list_state::ForumTopicListUiState,
    shell_state::{ActivePane, ShellState},
};

use super::chat_list_item::chat_list_item_line;
use super::forum_topic_list_item::forum_topic_list_item_line;
use super::{panel_title_style, styles};

pub(super) fn render_chat_list_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &ShellState,
    active_pane: ActivePane,
) {
    let is_active = active_pane == ActivePane::ChatList;
    let title_style = panel_title_style(is_active);

    if let Some(forum_list) = state.forum_topic_list() {
        render_forum_topic_list_panel(frame, area, forum_list, is_active, title_style);
        return;
    }

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
            let inner_width = area.width.saturating_sub(2) as usize;
            let layout = ChatListLayout::new(chats);
            let items = layout.build_items(chats, inner_width);
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
                    .map(|idx| layout.visual_index(idx))
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

pub(super) struct ChatListLayout {
    pub pinned_count: usize,
}

impl ChatListLayout {
    pub fn new(chats: &[ChatSummary]) -> Self {
        let pinned_count = chats.iter().filter(|c| c.is_pinned).count();
        Self { pinned_count }
    }

    pub fn has_pinned(&self) -> bool {
        self.pinned_count > 0
    }

    pub fn build_items(&self, chats: &[ChatSummary], width: usize) -> Vec<ListItem<'static>> {
        let (pinned, regular): (Vec<_>, Vec<_>) = chats.iter().partition(|c| c.is_pinned);

        let mut items = Vec::new();

        if self.has_pinned() {
            items.push(section_header_item("Pinned"));
            for chat in &pinned {
                items.push(ListItem::new(chat_list_item_line(chat, width)));
            }
        }

        if !regular.is_empty() || !self.has_pinned() {
            items.push(section_header_item("All Chats"));
            for chat in &regular {
                items.push(ListItem::new(chat_list_item_line(chat, width)));
            }
        }

        items
    }

    pub fn visual_index(&self, chat_index: usize) -> usize {
        if chat_index < self.pinned_count {
            chat_index + 1
        } else {
            let headers = if self.has_pinned() { 2 } else { 1 };
            chat_index + headers
        }
    }
}

fn section_header_item(title: &str) -> ListItem<'static> {
    let line = Line::from(vec![Span::styled(
        format!("-- {} --", title),
        styles::section_header_style(),
    )]);
    ListItem::new(line)
}

fn render_forum_topic_list_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    forum_list: &crate::domain::forum_topic_list_state::ForumTopicListState,
    is_active: bool,
    title_style: Style,
) {
    let panel_title = forum_list.parent_chat_title().to_owned();
    match forum_list.ui_state() {
        ForumTopicListUiState::Loading => {
            render_chat_list_message(frame, area, "Loading topics...", title_style)
        }
        ForumTopicListUiState::Empty => {
            render_chat_list_message(frame, area, "No topics yet.", title_style)
        }
        ForumTopicListUiState::Error => render_chat_list_message(
            frame,
            area,
            "Failed to load topics. Press h to go back.",
            title_style,
        ),
        ForumTopicListUiState::Ready => {
            let topics = forum_list.topics();
            let inner_width = area.width.saturating_sub(2) as usize;
            let mut items: Vec<ListItem<'static>> = Vec::with_capacity(topics.len());
            for topic in topics {
                items.push(ListItem::new(forum_topic_list_item_line(
                    topic,
                    inner_width,
                )));
            }

            let title = format!("{} ({})", panel_title, topics.len());
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

            let mut list_state = ListState::default();
            if is_active {
                list_state.select(forum_list.selected_index());
            }
            frame.render_stateful_widget(list, area, &mut list_state);
        }
    }
}
