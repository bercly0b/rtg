use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::domain::chat_search_state::ChatSearchState;

use super::styles;

pub fn render_chat_search_popup(frame: &mut Frame<'_>, area: Rect, state: &ChatSearchState) {
    let popup_width = (area.width / 2).max(20).min(area.width);
    let popup_height = 3_u16.min(area.height);

    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(styles::help_popup_border_style())
        .padding(Padding::horizontal(1));

    let line = Line::from(vec![
        Span::styled("/", styles::help_popup_key_style()),
        Span::styled(state.query().to_owned(), styles::help_popup_action_style()),
    ]);

    let paragraph = Paragraph::new(line).block(block);
    frame.render_widget(paragraph, popup_area);

    let cursor_x = popup_area.x + 2 + 1 + UnicodeWidthStr::width(state.query()) as u16;
    let cursor_y = popup_area.y + 1;
    if cursor_x < popup_area.right() && cursor_y < popup_area.bottom() {
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}
