mod offset;
mod rendering;

use ratatui::text::{Line, Text};

use super::ChatMessageListItem;

fn make_item(num_lines: usize) -> ChatMessageListItem<'static> {
    let lines: Vec<Line<'static>> = (0..num_lines)
        .map(|i| Line::raw(format!("line-{}", i)))
        .collect();
    ChatMessageListItem::new(Text::from(lines))
}

fn make_items(heights: &[usize]) -> Vec<ChatMessageListItem<'static>> {
    heights.iter().map(|&h| make_item(h)).collect()
}
