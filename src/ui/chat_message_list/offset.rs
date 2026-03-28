use crate::domain::open_chat_state::ScrollOffset;

use super::ChatMessageListItem;

/// Computes the scroll offset so that the last item's bottom line aligns with
/// the bottom of the viewport, filling the entire viewport with content.
///
/// If all items fit in the viewport, returns `ScrollOffset::ZERO`.
pub(super) fn compute_bottom_aligned_offset(
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
            let lines_to_skip = h - remaining;
            return ScrollOffset {
                item: i,
                line: lines_to_skip,
            };
        }
        remaining -= h;
    }

    ScrollOffset::ZERO
}

/// Adjusts the offset so that the selected item is fully visible within the viewport,
/// respecting `scroll_padding` (minimum items above/below the selected item).
pub(super) fn ensure_selected_visible(
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

    let sel_start = absolute_line_start(items, selected);
    let sel_end = sel_start + items[selected].height();

    let viewport_top = absolute_line_start(items, current.item) + current.line;
    let viewport_bottom = viewport_top + viewport_height;

    let padding_above =
        items_height_range(items, selected.saturating_sub(scroll_padding), selected);
    let padding_below = items_height_range(
        items,
        (selected + 1).min(items.len()),
        (selected + 1 + scroll_padding).min(items.len()),
    );

    let desired_top = sel_start.saturating_sub(padding_above);
    let desired_bottom = sel_end + padding_below;

    // Guard: if the padded range exceeds the viewport, drop padding to avoid
    // oscillation (frame N scrolls up for padding_above, frame N+1 scrolls
    // down because selection fell off-screen, repeat).
    if desired_bottom.saturating_sub(desired_top) > viewport_height {
        if sel_start >= viewport_top && sel_end <= viewport_bottom {
            return current;
        }
        if sel_start < viewport_top {
            return absolute_line_to_offset(items, sel_start);
        }
        let new_viewport_top = sel_end.saturating_sub(viewport_height);
        return absolute_line_to_offset(items, new_viewport_top);
    }

    if desired_top >= viewport_top && desired_bottom <= viewport_bottom {
        return current;
    }

    if desired_top < viewport_top {
        return absolute_line_to_offset(items, desired_top);
    }

    let new_viewport_top = desired_bottom.saturating_sub(viewport_height);
    absolute_line_to_offset(items, new_viewport_top)
}

/// Clamps the offset so the viewport does not extend past the last item.
///
/// If the total content is shorter than the viewport, returns `ScrollOffset::ZERO`.
pub(super) fn clamp_offset(
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
pub(super) fn absolute_line_start(items: &[ChatMessageListItem<'_>], item_index: usize) -> usize {
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
    if let Some(last) = items.last() {
        ScrollOffset {
            item: items.len() - 1,
            line: last.height().saturating_sub(1),
        }
    } else {
        ScrollOffset::ZERO
    }
}
