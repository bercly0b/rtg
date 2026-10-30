use ratatui::{
    backend::TestBackend,
    buffer::Buffer,
    style::{Color, Style},
    Terminal,
};

use super::{
    super::chat_list::{render_chat_list_panel, render_forum_topic_list_panel, ChatListLayout},
    chat, chat_with_pinned,
};
use crate::domain::{
    forum_topic::ForumTopicSummary,
    forum_topic_list_state::ForumTopicListState,
    shell_state::{ActivePane, ShellState},
};

const TEST_WIDTH: usize = 50;
const SCROLLED_INDEX: usize = 25;

fn buffer_text(buffer: &Buffer) -> String {
    let mut rendered = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            rendered.push_str(buffer[(x, y)].symbol());
        }
        rendered.push('\n');
    }
    rendered
}

fn has_highlighted_cell(buffer: &Buffer) -> bool {
    buffer.content().iter().any(|cell| cell.bg == Color::Gray)
}

fn state_scrolled_down() -> ShellState {
    let mut state = ShellState::default();
    let chats = (0..30)
        .map(|i| chat(i, &format!("Chat {i:02}"), 0, None))
        .collect();
    state.chat_list_mut().set_ready(chats);
    for _ in 0..SCROLLED_INDEX {
        state.chat_list_mut().select_next();
    }
    state
}

fn render_chat_list(state: &ShellState, active_pane: ActivePane) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(60, 10)).unwrap();
    terminal
        .draw(|frame| render_chat_list_panel(frame, frame.area(), state, active_pane))
        .unwrap();
    terminal.backend().buffer().clone()
}

fn topic(topic_id: i32, name: &str, order: i64) -> ForumTopicSummary {
    ForumTopicSummary {
        chat_id: 100,
        topic_id,
        name: name.to_owned(),
        is_general: false,
        is_closed: false,
        is_hidden: false,
        is_pinned: false,
        unread_count: 0,
        last_message_preview: None,
        last_message_unix_ms: None,
        last_message_id: None,
        order,
    }
}

fn forum_scrolled_down() -> ForumTopicListState {
    let mut forum = ForumTopicListState::loading(100, "My Forum".to_owned());
    let topics = (0..30)
        .map(|i| topic(i, &format!("Topic {i:02}"), i64::from(100 - i)))
        .collect();
    forum.set_ready(topics);
    for _ in 0..SCROLLED_INDEX {
        forum.select_next();
    }
    forum
}

fn render_forum(forum: &ForumTopicListState, is_active: bool) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(60, 10)).unwrap();
    terminal
        .draw(|frame| {
            render_forum_topic_list_panel(frame, frame.area(), forum, is_active, Style::default())
        })
        .unwrap();
    terminal.backend().buffer().clone()
}

#[test]
fn chat_list_keeps_scroll_position_when_focus_moves_to_messages() {
    let state = state_scrolled_down();

    let active = render_chat_list(&state, ActivePane::ChatList);
    let inactive = render_chat_list(&state, ActivePane::Messages);

    assert!(buffer_text(&inactive).contains("Chat 25"));
    assert_eq!(buffer_text(&inactive), buffer_text(&active));
}

#[test]
fn inactive_chat_list_does_not_highlight_selected_chat() {
    let state = state_scrolled_down();

    assert!(has_highlighted_cell(&render_chat_list(
        &state,
        ActivePane::ChatList
    )));
    assert!(!has_highlighted_cell(&render_chat_list(
        &state,
        ActivePane::Messages
    )));
}

#[test]
fn forum_topic_list_keeps_scroll_position_when_focus_moves_to_messages() {
    let forum = forum_scrolled_down();

    let active = render_forum(&forum, true);
    let inactive = render_forum(&forum, false);

    assert!(buffer_text(&inactive).contains("Topic 25"));
    assert_eq!(buffer_text(&inactive), buffer_text(&active));
}

#[test]
fn inactive_forum_topic_list_does_not_highlight_selected_topic() {
    let forum = forum_scrolled_down();

    assert!(has_highlighted_cell(&render_forum(&forum, true)));
    assert!(!has_highlighted_cell(&render_forum(&forum, false)));
}

#[test]
fn build_chat_list_items_creates_all_chats_section_for_regular_chats() {
    let chats = vec![chat(1, "General", 0, Some("Hello"))];
    let layout = ChatListLayout::new(&chats);
    let items = layout.build_items(&chats, TEST_WIDTH);

    assert_eq!(items.len(), 2);
}

#[test]
fn build_chat_list_items_creates_both_sections_when_pinned_exists() {
    let chats = vec![
        chat_with_pinned(1, "Pinned Chat", 0, Some("Hi"), true),
        chat(2, "Regular Chat", 0, Some("Hello")),
    ];
    let layout = ChatListLayout::new(&chats);
    let items = layout.build_items(&chats, TEST_WIDTH);

    assert_eq!(items.len(), 4);
}

#[test]
fn compute_visual_index_accounts_for_headers() {
    let chats = vec![
        chat_with_pinned(1, "Pinned", 0, None, true),
        chat(2, "Regular", 0, None),
    ];
    let layout = ChatListLayout::new(&chats);

    assert_eq!(layout.visual_index(&chats, 0), Some(1));
    assert_eq!(layout.visual_index(&chats, 1), Some(3));
}

#[test]
fn compute_visual_index_with_no_pinned() {
    let chats = vec![chat(1, "Chat1", 0, None), chat(2, "Chat2", 0, None)];
    let layout = ChatListLayout::new(&chats);

    assert_eq!(layout.visual_index(&chats, 0), Some(1));
    assert_eq!(layout.visual_index(&chats, 1), Some(2));
}

#[test]
fn compute_visual_index_with_all_pinned() {
    let chats = vec![
        chat_with_pinned(1, "Pinned1", 0, None, true),
        chat_with_pinned(2, "Pinned2", 0, None, true),
    ];
    let layout = ChatListLayout::new(&chats);

    assert_eq!(layout.visual_index(&chats, 0), Some(1));
    assert_eq!(layout.visual_index(&chats, 1), Some(2));
}

#[test]
fn compute_visual_index_uses_position_within_group_for_interleaved_pinned() {
    let chats = vec![
        chat(1, "Regular1", 0, None),
        chat_with_pinned(2, "Pinned1", 0, None, true),
        chat(3, "Regular2", 0, None),
        chat_with_pinned(4, "Pinned2", 0, None, true),
    ];
    let layout = ChatListLayout::new(&chats);

    // Rendered rows: Pinned header, Pinned1, Pinned2, All Chats header, Regular1, Regular2.
    assert_eq!(layout.visual_index(&chats, 0), Some(4));
    assert_eq!(layout.visual_index(&chats, 1), Some(1));
    assert_eq!(layout.visual_index(&chats, 2), Some(5));
    assert_eq!(layout.visual_index(&chats, 3), Some(2));
}

#[test]
fn compute_visual_index_is_none_out_of_range() {
    let chats = vec![chat(1, "Chat1", 0, None)];
    let layout = ChatListLayout::new(&chats);

    assert_eq!(layout.visual_index(&chats, 1), None);
}

#[test]
fn interleaved_pinned_chat_is_highlighted_in_pinned_section() {
    let mut state = ShellState::default();
    state.chat_list_mut().set_ready(vec![
        chat(1, "Regular", 0, None),
        chat_with_pinned(2, "Pinned", 0, None, true),
    ]);
    state.chat_list_mut().select_next();

    let buffer = render_chat_list(&state, ActivePane::ChatList);

    let highlighted_row = (0..buffer.area.height)
        .find(|&y| (0..buffer.area.width).any(|x| buffer[(x, y)].bg == Color::Gray))
        .expect("a row is highlighted");
    let row_text: String = (0..buffer.area.width)
        .map(|x| buffer[(x, highlighted_row)].symbol())
        .collect();
    assert!(row_text.contains("Pinned"), "highlighted row: {row_text:?}");
    assert!(!row_text.contains("--"), "header must not be highlighted");
}

#[test]
fn build_chat_list_items_shows_all_chats_header_when_all_pinned() {
    let chats = vec![
        chat_with_pinned(1, "Pinned1", 0, None, true),
        chat_with_pinned(2, "Pinned2", 0, None, true),
    ];
    let layout = ChatListLayout::new(&chats);
    let items = layout.build_items(&chats, TEST_WIDTH);

    assert_eq!(items.len(), 3);
}

#[test]
fn layout_pinned_count_is_consistent() {
    let chats = vec![
        chat_with_pinned(1, "Pinned", 0, None, true),
        chat(2, "Regular", 0, None),
    ];
    let layout = ChatListLayout::new(&chats);

    assert_eq!(layout.pinned_count, 1);
    assert!(layout.has_pinned());
}

#[test]
fn layout_no_pinned() {
    let chats = vec![chat(1, "Chat1", 0, None)];
    let layout = ChatListLayout::new(&chats);

    assert_eq!(layout.pinned_count, 0);
    assert!(!layout.has_pinned());
}

#[test]
fn forum_panel_loading_state_shows_forum_title_not_chats() {
    let backend = TestBackend::new(40, 5);
    let mut terminal = Terminal::new(backend).unwrap();

    let forum = ForumTopicListState::loading(100, "My Forum".to_owned());

    terminal
        .draw(|frame| {
            let area = frame.area();
            render_forum_topic_list_panel(frame, area, &forum, true, Style::default());
        })
        .unwrap();

    let buffer = terminal.backend().buffer().clone();
    let mut rendered = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            rendered.push_str(buffer[(x, y)].symbol());
        }
    }

    assert!(
        rendered.contains("My Forum"),
        "panel title in Loading state must be the forum name, got: {rendered}"
    );
    assert!(
        !rendered.contains("Chats"),
        "panel title in Loading state must not fall back to 'Chats', got: {rendered}"
    );
}
