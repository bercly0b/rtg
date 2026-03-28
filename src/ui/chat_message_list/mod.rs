//! Custom chat message list widget with line-level scrolling.
//!
//! Unlike ratatui's built-in `List`, this widget supports **sub-item scrolling**:
//! it can display a partial first item (clipping its top lines) so that the
//! viewport is always fully filled with content. This is essential for
//! Telegram-style chat rendering where messages can be very tall (multi-line)
//! and the last message should be anchored to the bottom of the viewport.

mod offset;
mod rendering;

#[cfg(test)]
mod tests;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::Text,
    widgets::{Block, StatefulWidget, Widget},
};

pub use crate::domain::open_chat_state::ScrollOffset;

// ─── State ──────────────────────────────────────────────────────────────────

/// State for [`ChatMessageList`].
///
/// Tracks scroll position (line-level) and the currently selected item.
#[derive(Debug, Clone)]
pub struct ChatMessageListState {
    offset: ScrollOffset,
    selected: Option<usize>,
}

impl ChatMessageListState {
    pub fn new(offset: ScrollOffset, selected: Option<usize>) -> Self {
        Self { offset, selected }
    }

    pub fn offset(&self) -> ScrollOffset {
        self.offset
    }

    #[allow(dead_code)]
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }
}

// ─── Widget ─────────────────────────────────────────────────────────────────

/// A chat message list widget that supports line-level (sub-item) scrolling.
///
/// Items are rendered top-to-bottom. When the viewport cannot fit all items,
/// the topmost visible item may be partially clipped (its first N lines hidden)
/// to ensure the viewport is fully filled.
pub struct ChatMessageList<'a> {
    items: Vec<ChatMessageListItem<'a>>,
    block: Option<Block<'a>>,
    highlight_style: Style,
    scroll_padding: usize,
}

/// A single item in the chat message list.
///
/// Wraps `ratatui::text::Text` and delegates height computation.
pub struct ChatMessageListItem<'a> {
    pub(super) content: Text<'a>,
}

impl<'a> ChatMessageListItem<'a> {
    pub fn new(content: Text<'a>) -> Self {
        Self { content }
    }

    pub fn height(&self) -> usize {
        self.content.height()
    }
}

impl<'a> From<Text<'a>> for ChatMessageListItem<'a> {
    fn from(text: Text<'a>) -> Self {
        Self::new(text)
    }
}

impl<'a> ChatMessageList<'a> {
    pub fn new<I>(items: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<ChatMessageListItem<'a>>,
    {
        Self {
            items: items.into_iter().map(Into::into).collect(),
            block: None,
            highlight_style: Style::default(),
            scroll_padding: 0,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn highlight_style(mut self, style: Style) -> Self {
        self.highlight_style = style;
        self
    }

    pub fn scroll_padding(mut self, padding: usize) -> Self {
        self.scroll_padding = padding;
        self
    }
}

impl<'a> StatefulWidget for ChatMessageList<'a> {
    type State = ChatMessageListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let inner = if let Some(block) = &self.block {
            let inner = block.inner(area);
            block.clone().render(area, buf);
            inner
        } else {
            area
        };

        if inner.is_empty() || self.items.is_empty() {
            return;
        }

        let viewport_height = inner.height as usize;

        let mut scroll = if state.offset.is_bottom_sentinel() {
            offset::compute_bottom_aligned_offset(&self.items, viewport_height)
        } else {
            state.offset
        };

        if let Some(selected) = state.selected {
            scroll = offset::ensure_selected_visible(
                &self.items,
                scroll,
                selected,
                viewport_height,
                self.scroll_padding,
            );
        }

        scroll = offset::clamp_offset(&self.items, scroll, viewport_height);

        rendering::render_items(
            &self.items,
            scroll,
            inner,
            buf,
            state.selected,
            self.highlight_style,
        );

        state.offset = scroll;
    }
}
