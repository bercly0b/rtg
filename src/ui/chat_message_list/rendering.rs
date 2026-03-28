use ratatui::{buffer::Buffer, layout::Rect, style::Style};

use crate::domain::open_chat_state::ScrollOffset;

use super::ChatMessageListItem;

/// Rendering context shared across item rendering calls.
struct RenderCtx {
    area: Rect,
    highlight_style: Style,
}

/// Renders items into the viewport, clipping the first item's top lines as needed.
pub(super) fn render_items(
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
    let mut y = 0_usize;

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

        let lines_to_render = visible_lines_in_item.min(viewport_height - y);
        let is_selected = selected == Some(item_idx);

        render_item_lines(item, skip_lines, lines_to_render, y, buf, is_selected, &ctx);

        y += lines_to_render;
        item_idx += 1;
        skip_lines = 0;
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

        let alignment_padding = compute_alignment_padding(line, area.width as usize);

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

        // Apply highlight AFTER content so it overrides text colors (matches
        // ratatui List behavior: selected row gets uniform fg/bg).
        if is_selected {
            buf.set_style(line_area, ctx.highlight_style);
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
