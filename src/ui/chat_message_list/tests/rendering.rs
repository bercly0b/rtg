use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Text},
    widgets::StatefulWidget,
};

use crate::ui::chat_message_list::{ChatMessageList, ChatMessageListState, ScrollOffset};

use super::make_items;

#[test]
fn render_fills_viewport_bottom_aligned() {
    // Simulate: 2 tall messages, viewport of 10 lines
    let items = make_items(&[8, 8]); // 16 lines total
    let widget = ChatMessageList::new(items.into_iter().map(|i| i.content).collect::<Vec<_>>());

    let area = Rect::new(0, 0, 20, 10);
    let mut buf = Buffer::empty(area);
    let mut state = ChatMessageListState::new(ScrollOffset::BOTTOM, Some(1));

    widget.render(area, &mut buf, &mut state);

    // After render, offset should be bottom-aligned:
    // Total 16 lines, viewport 10 → skip 6 lines → item 0, line 6
    assert_eq!(state.offset(), ScrollOffset { item: 0, line: 6 });
}

#[test]
fn render_all_items_fit_no_clipping() {
    let items = make_items(&[3, 3]); // 6 lines total
    let widget = ChatMessageList::new(items.into_iter().map(|i| i.content).collect::<Vec<_>>());

    let area = Rect::new(0, 0, 20, 10);
    let mut buf = Buffer::empty(area);
    let mut state = ChatMessageListState::new(ScrollOffset::BOTTOM, Some(1));

    widget.render(area, &mut buf, &mut state);

    // All items fit, should start at 0,0
    assert_eq!(state.offset(), ScrollOffset::ZERO);
}

#[test]
fn render_content_appears_in_buffer() {
    let text = Text::from(vec![Line::raw("Hello world")]);
    let widget = ChatMessageList::new(vec![text]);

    let area = Rect::new(0, 0, 20, 5);
    let mut buf = Buffer::empty(area);
    let mut state = ChatMessageListState::new(ScrollOffset::BOTTOM, Some(0));

    widget.render(area, &mut buf, &mut state);

    let cell = &buf[(0, 0)];
    assert_eq!(cell.symbol(), "H");
}

#[test]
fn render_partial_first_item_skips_top_lines() {
    let text = Text::from(vec![
        Line::raw("line-0"),
        Line::raw("line-1"),
        Line::raw("line-2"),
        Line::raw("line-3"),
    ]);
    let widget = ChatMessageList::new(vec![text]);

    let area = Rect::new(0, 0, 20, 2);
    let mut buf = Buffer::empty(area);
    // Skip first 2 lines, show lines 2-3
    let mut state = ChatMessageListState::new(ScrollOffset { item: 0, line: 2 }, None);

    widget.render(area, &mut buf, &mut state);

    // Row 0 should show "line-2", row 1 should show "line-3"
    let row0: String = (0..6)
        .map(|x| buf[(x, 0)].symbol().to_string())
        .collect::<String>();
    let row1: String = (0..6)
        .map(|x| buf[(x, 1)].symbol().to_string())
        .collect::<String>();
    assert_eq!(row0, "line-2");
    assert_eq!(row1, "line-3");
}

#[test]
fn scroll_offset_bottom_sentinel() {
    assert!(ScrollOffset::BOTTOM.is_bottom_sentinel());
    assert!(!ScrollOffset::ZERO.is_bottom_sentinel());
}

#[test]
fn highlight_style_overrides_span_colors() {
    use ratatui::style::Color;

    let line = Line::from(vec![
        ratatui::text::Span::styled("Hello", Style::default().fg(Color::Red)),
        ratatui::text::Span::styled(" world", Style::default().fg(Color::Blue)),
    ]);
    let text = Text::from(vec![line]);
    let highlight = Style::default().fg(Color::Black).bg(Color::Gray);
    let widget = ChatMessageList::new(vec![text]).highlight_style(highlight);

    let area = Rect::new(0, 0, 20, 3);
    let mut buf = Buffer::empty(area);
    let mut state = ChatMessageListState::new(ScrollOffset::ZERO, Some(0));

    widget.render(area, &mut buf, &mut state);

    // Every cell in the rendered row should have highlight fg/bg, not the
    // original span fg.
    let cell_h = &buf[(0, 0)]; // 'H' from "Hello"
    assert_eq!(cell_h.fg, Color::Black);
    assert_eq!(cell_h.bg, Color::Gray);

    let cell_w = &buf[(6, 0)]; // 'w' from " world"
    assert_eq!(cell_w.fg, Color::Black);
    assert_eq!(cell_w.bg, Color::Gray);
}
