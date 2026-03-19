//! Custom chat message list widget with line-level scrolling.
//!
//! Unlike ratatui's built-in `List`, this widget supports **sub-item scrolling**:
//! it can display a partial first item (clipping its top lines) so that the
//! viewport is always fully filled with content. This is essential for
//! Telegram-style chat rendering where messages can be very tall (multi-line)
//! and the last message should be anchored to the bottom of the viewport.

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
    content: Text<'a>,
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
        // Render the block (border/title) first, then work in the inner area.
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

        // Resolve scroll offset.
        let mut offset = if state.offset.is_bottom_sentinel() {
            compute_bottom_aligned_offset(&self.items, viewport_height)
        } else {
            state.offset
        };

        // Ensure the selected item is visible, adjusting offset if needed.
        if let Some(selected) = state.selected {
            offset = ensure_selected_visible(
                &self.items,
                offset,
                selected,
                viewport_height,
                self.scroll_padding,
            );
        }

        // Clamp offset to valid range.
        offset = clamp_offset(&self.items, offset, viewport_height);

        // Render visible items.
        render_items(
            &self.items,
            offset,
            inner,
            buf,
            state.selected,
            self.highlight_style,
        );

        // Persist computed offset back to state.
        state.offset = offset;
    }
}

// ─── Rendering helpers ──────────────────────────────────────────────────────

/// Rendering context shared across item rendering calls.
struct RenderCtx {
    area: Rect,
    highlight_style: Style,
}

/// Renders items into the viewport, clipping the first item's top lines as needed.
fn render_items(
    items: &[ChatMessageListItem<'_>],
    offset: ScrollOffset,
    area: Rect,
    buf: &mut Buffer,
    selected: Option<usize>,
    highlight_style: Style,
) {
    let ctx = RenderCtx {
        area,
        highlight_style,
    };
    let viewport_height = area.height as usize;
    let mut y = 0_usize; // lines rendered so far

    let mut item_idx = offset.item;
    let mut skip_lines = offset.line;

    while y < viewport_height && item_idx < items.len() {
        let item = &items[item_idx];
        let item_height = item.height();
        let visible_lines_in_item = item_height.saturating_sub(skip_lines);

        if visible_lines_in_item == 0 {
            item_idx += 1;
            skip_lines = 0;
            continue;
        }

        // How many lines of this item fit in the remaining viewport?
        let lines_to_render = visible_lines_in_item.min(viewport_height - y);

        // Determine if this item is selected (for highlight).
        let is_selected = selected == Some(item_idx);

        // Render the visible lines of this item.
        render_item_lines(item, skip_lines, lines_to_render, y, buf, is_selected, &ctx);

        y += lines_to_render;
        item_idx += 1;
        skip_lines = 0; // Only the first item can be partially clipped at top
    }
}

/// Renders `lines_to_render` lines from `item`, starting at line `skip` within the item.
///
/// Lines are placed starting at row `y_offset` within `ctx.area`.
fn render_item_lines(
    item: &ChatMessageListItem<'_>,
    skip: usize,
    lines_to_render: usize,
    y_offset: usize,
    buf: &mut Buffer,
    is_selected: bool,
    ctx: &RenderCtx,
) {
    let area = ctx.area;
    let content_lines = &item.content.lines;

    for (i, line) in content_lines
        .iter()
        .skip(skip)
        .take(lines_to_render)
        .enumerate()
    {
        let row = area
            .y
            .saturating_add(u16::try_from(y_offset + i).unwrap_or(u16::MAX));
        if row >= area.y + area.height {
            break;
        }

        let line_area = Rect {
            x: area.x,
            y: row,
            width: area.width,
            height: 1,
        };

        // If selected, fill the row background with highlight style first.
        if is_selected {
            buf.set_style(line_area, ctx.highlight_style);
        }

        // Compute alignment padding before rendering (single-pass).
        let alignment_padding = compute_alignment_padding(line, area.width as usize);

        // Render the line content with alignment offset applied.
        let mut x = area
            .x
            .saturating_add(u16::try_from(alignment_padding).unwrap_or(u16::MAX));
        for span in &line.spans {
            if x >= area.x + area.width {
                break;
            }
            let remaining_width = (area.x + area.width).saturating_sub(x) as usize;
            let (written_x, _) =
                buf.set_stringn(x, row, &span.content, remaining_width, span.style);
            x = written_x;
        }
    }
}

/// Computes the horizontal padding for a line based on its alignment.
fn compute_alignment_padding(line: &ratatui::text::Line<'_>, available_width: usize) -> usize {
    match line.alignment {
        Some(ratatui::layout::Alignment::Center) => {
            let line_width: usize = line
                .spans
                .iter()
                .map(|s| unicode_width::UnicodeWidthStr::width(s.content.as_ref()))
                .sum();
            available_width.saturating_sub(line_width) / 2
        }
        Some(ratatui::layout::Alignment::Right) => {
            let line_width: usize = line
                .spans
                .iter()
                .map(|s| unicode_width::UnicodeWidthStr::width(s.content.as_ref()))
                .sum();
            available_width.saturating_sub(line_width)
        }
        _ => 0,
    }
}

// ─── Offset computation ─────────────────────────────────────────────────────

/// Computes the scroll offset so that the last item's bottom line aligns with
/// the bottom of the viewport, filling the entire viewport with content.
///
/// If all items fit in the viewport, returns `ScrollOffset::ZERO`.
pub fn compute_bottom_aligned_offset(
    items: &[ChatMessageListItem<'_>],
    viewport_height: usize,
) -> ScrollOffset {
    if items.is_empty() || viewport_height == 0 {
        return ScrollOffset::ZERO;
    }

    let mut remaining = viewport_height;

    for i in (0..items.len()).rev() {
        let h = items[i].height();
        if h >= remaining {
            // This item fills or exceeds the remaining viewport.
            // Skip enough top lines so only `remaining` lines are visible.
            let lines_to_skip = h - remaining;
            return ScrollOffset {
                item: i,
                line: lines_to_skip,
            };
        }
        remaining -= h;
    }

    // All items fit — no scrolling needed.
    ScrollOffset::ZERO
}

/// Adjusts the offset so that the selected item is fully visible within the viewport,
/// respecting `scroll_padding` (minimum items above/below the selected item).
fn ensure_selected_visible(
    items: &[ChatMessageListItem<'_>],
    current: ScrollOffset,
    selected: usize,
    viewport_height: usize,
    scroll_padding: usize,
) -> ScrollOffset {
    if items.is_empty() || viewport_height == 0 {
        return current;
    }

    let selected = selected.min(items.len() - 1);

    // Compute the line range of the selected item relative to the viewport top.
    // First, compute the absolute line position of the selected item.
    let sel_start = absolute_line_start(items, selected);
    let sel_end = sel_start + items[selected].height(); // exclusive

    // Compute the absolute line of the viewport top.
    let viewport_top = absolute_line_start(items, current.item) + current.line;
    let viewport_bottom = viewport_top + viewport_height;

    // Compute padding in lines: sum heights of `scroll_padding` items above/below.
    let padding_above =
        items_height_range(items, selected.saturating_sub(scroll_padding), selected);
    let padding_below = items_height_range(
        items,
        (selected + 1).min(items.len()),
        (selected + 1 + scroll_padding).min(items.len()),
    );

    // Check if selected item (with padding) is already visible.
    let desired_top = sel_start.saturating_sub(padding_above);
    let desired_bottom = sel_end + padding_below;

    if desired_top >= viewport_top && desired_bottom <= viewport_bottom {
        // Already visible with padding — no adjustment needed.
        return current;
    }

    if desired_top < viewport_top {
        // Need to scroll up — selected item (+ padding) is above viewport.
        return absolute_line_to_offset(items, desired_top);
    }

    // Need to scroll down — selected item (+ padding) is below viewport.
    let new_viewport_top = desired_bottom.saturating_sub(viewport_height);
    absolute_line_to_offset(items, new_viewport_top)
}

/// Clamps the offset so the viewport does not extend past the last item.
///
/// If the total content is shorter than the viewport, returns `ScrollOffset::ZERO`.
fn clamp_offset(
    items: &[ChatMessageListItem<'_>],
    offset: ScrollOffset,
    viewport_height: usize,
) -> ScrollOffset {
    if items.is_empty() || viewport_height == 0 {
        return ScrollOffset::ZERO;
    }

    let total_lines: usize = items.iter().map(|i| i.height()).sum();

    if total_lines <= viewport_height {
        return ScrollOffset::ZERO;
    }

    let max_top_line = total_lines - viewport_height;

    // Validate offset bounds before using the values for arithmetic.
    if offset.item >= items.len() || offset.line >= items[offset.item].height() {
        return absolute_line_to_offset(items, max_top_line);
    }

    let current_top_line = absolute_line_start(items, offset.item) + offset.line;

    if current_top_line > max_top_line {
        return absolute_line_to_offset(items, max_top_line);
    }

    offset
}

// ─── Line math helpers ──────────────────────────────────────────────────────

/// Returns the absolute line number where `item_index` starts.
fn absolute_line_start(items: &[ChatMessageListItem<'_>], item_index: usize) -> usize {
    items.iter().take(item_index).map(|i| i.height()).sum()
}

/// Sum of heights of items in `[from..to)`.
fn items_height_range(items: &[ChatMessageListItem<'_>], from: usize, to: usize) -> usize {
    items
        .iter()
        .skip(from)
        .take(to - from)
        .map(|i| i.height())
        .sum()
}

/// Converts an absolute line number to a `ScrollOffset` (item index + lines to skip).
fn absolute_line_to_offset(items: &[ChatMessageListItem<'_>], abs_line: usize) -> ScrollOffset {
    let mut acc = 0;
    for (i, item) in items.iter().enumerate() {
        let h = item.height();
        if acc + h > abs_line {
            return ScrollOffset {
                item: i,
                line: abs_line - acc,
            };
        }
        acc += h;
    }
    // Past the end — return offset pointing at the last item's last line.
    if let Some(last) = items.last() {
        ScrollOffset {
            item: items.len() - 1,
            line: last.height().saturating_sub(1),
        }
    } else {
        ScrollOffset::ZERO
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::text::Line;

    fn make_item(num_lines: usize) -> ChatMessageListItem<'static> {
        let lines: Vec<Line<'static>> = (0..num_lines)
            .map(|i| Line::raw(format!("line-{}", i)))
            .collect();
        ChatMessageListItem::new(Text::from(lines))
    }

    fn make_items(heights: &[usize]) -> Vec<ChatMessageListItem<'static>> {
        heights.iter().map(|&h| make_item(h)).collect()
    }

    // ── compute_bottom_aligned_offset ──

    #[test]
    fn bottom_aligned_all_items_fit() {
        let items = make_items(&[2, 3, 2]); // total 7 lines
        let offset = compute_bottom_aligned_offset(&items, 10);
        assert_eq!(offset, ScrollOffset::ZERO);
    }

    #[test]
    fn bottom_aligned_exact_fit() {
        let items = make_items(&[3, 3, 4]); // total 10 lines
        let offset = compute_bottom_aligned_offset(&items, 10);
        assert_eq!(offset, ScrollOffset::ZERO);
    }

    #[test]
    fn bottom_aligned_clips_first_item() {
        // Items: 5 + 3 = 8 lines, viewport = 6
        // Need to skip 2 lines of first item to show: 3 lines of item0 + 3 lines of item1
        let items = make_items(&[5, 3]);
        let offset = compute_bottom_aligned_offset(&items, 6);
        assert_eq!(offset, ScrollOffset { item: 0, line: 2 });
    }

    #[test]
    fn bottom_aligned_starts_from_middle_item() {
        // Items: 3 + 3 + 5 = 11, viewport = 7
        // From end: item2(5) uses 5, remaining=2, item1(3) > 2 → skip 1 line of item1
        let items = make_items(&[3, 3, 5]);
        let offset = compute_bottom_aligned_offset(&items, 7);
        assert_eq!(offset, ScrollOffset { item: 1, line: 1 });
    }

    #[test]
    fn bottom_aligned_single_tall_item() {
        let items = make_items(&[20]);
        let offset = compute_bottom_aligned_offset(&items, 10);
        assert_eq!(offset, ScrollOffset { item: 0, line: 10 });
    }

    #[test]
    fn bottom_aligned_empty_items() {
        let items: Vec<ChatMessageListItem<'_>> = vec![];
        let offset = compute_bottom_aligned_offset(&items, 10);
        assert_eq!(offset, ScrollOffset::ZERO);
    }

    #[test]
    fn bottom_aligned_zero_viewport() {
        let items = make_items(&[5]);
        let offset = compute_bottom_aligned_offset(&items, 0);
        assert_eq!(offset, ScrollOffset::ZERO);
    }

    // ── clamp_offset ──

    #[test]
    fn clamp_offset_clamped_when_past_max() {
        let items = make_items(&[5, 5, 5]); // 15 lines
                                            // Offset at line 7 (item 1, line 2), max top = 15 - 10 = 5 → clamp to 5.
        let offset = ScrollOffset { item: 1, line: 2 };
        let clamped = clamp_offset(&items, offset, 10);
        assert_eq!(clamped, ScrollOffset { item: 1, line: 0 });
    }

    #[test]
    fn clamp_offset_within_bounds() {
        let items = make_items(&[5, 5, 5]); // 15 lines
                                            // Offset at line 3 (item 0, line 3), max top = 15 - 10 = 5 → no clamping.
        let offset = ScrollOffset { item: 0, line: 3 };
        let clamped = clamp_offset(&items, offset, 10);
        assert_eq!(clamped, offset);
    }

    #[test]
    fn clamp_offset_all_fit() {
        let items = make_items(&[3, 3]); // 6 lines
        let offset = ScrollOffset { item: 1, line: 0 };
        let clamped = clamp_offset(&items, offset, 10);
        assert_eq!(clamped, ScrollOffset::ZERO);
    }

    #[test]
    fn clamp_offset_past_end() {
        let items = make_items(&[5, 5, 5]); // 15 lines
        let offset = ScrollOffset { item: 10, line: 0 };
        let clamped = clamp_offset(&items, offset, 10);
        // max top = 15 - 10 = 5 → item 1, line 0
        assert_eq!(clamped, ScrollOffset { item: 1, line: 0 });
    }

    // ── absolute_line_start ──

    #[test]
    fn absolute_line_start_first_item() {
        let items = make_items(&[3, 5, 2]);
        assert_eq!(absolute_line_start(&items, 0), 0);
    }

    #[test]
    fn absolute_line_start_second_item() {
        let items = make_items(&[3, 5, 2]);
        assert_eq!(absolute_line_start(&items, 1), 3);
    }

    #[test]
    fn absolute_line_start_third_item() {
        let items = make_items(&[3, 5, 2]);
        assert_eq!(absolute_line_start(&items, 2), 8);
    }

    // ── absolute_line_to_offset ──

    #[test]
    fn abs_line_to_offset_at_item_boundary() {
        let items = make_items(&[3, 5, 2]);
        assert_eq!(
            absolute_line_to_offset(&items, 3),
            ScrollOffset { item: 1, line: 0 }
        );
    }

    #[test]
    fn abs_line_to_offset_mid_item() {
        let items = make_items(&[3, 5, 2]);
        assert_eq!(
            absolute_line_to_offset(&items, 5),
            ScrollOffset { item: 1, line: 2 }
        );
    }

    #[test]
    fn abs_line_to_offset_zero() {
        let items = make_items(&[3, 5]);
        assert_eq!(
            absolute_line_to_offset(&items, 0),
            ScrollOffset { item: 0, line: 0 }
        );
    }

    // ── ensure_selected_visible ──

    #[test]
    fn selected_already_visible_no_change() {
        let items = make_items(&[3, 3, 3]); // 9 lines
        let offset = ScrollOffset::ZERO;
        let result = ensure_selected_visible(&items, offset, 1, 9, 0);
        assert_eq!(result, ScrollOffset::ZERO);
    }

    #[test]
    fn selected_below_viewport_scrolls_down() {
        let items = make_items(&[5, 5, 5]); // 15 lines
        let offset = ScrollOffset::ZERO;
        // Select last item (starts at line 10), viewport is 8 lines
        let result = ensure_selected_visible(&items, offset, 2, 8, 0);
        // Selected item is lines 10..15, viewport must show it:
        // viewport top should be at least 15 - 8 = 7
        assert_eq!(result, ScrollOffset { item: 1, line: 2 });
    }

    #[test]
    fn selected_above_viewport_scrolls_up() {
        let items = make_items(&[5, 5, 5]); // 15 lines
        let offset = ScrollOffset { item: 2, line: 0 }; // viewport starts at item 2
                                                        // Select first item (starts at line 0)
        let result = ensure_selected_visible(&items, offset, 0, 8, 0);
        assert_eq!(result, ScrollOffset::ZERO);
    }

    // ── Integration: StatefulWidget render ──

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

        // Check that the content was rendered into the buffer.
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

    // ── ScrollOffset ──

    #[test]
    fn scroll_offset_bottom_sentinel() {
        assert!(ScrollOffset::BOTTOM.is_bottom_sentinel());
        assert!(!ScrollOffset::ZERO.is_bottom_sentinel());
    }
}
