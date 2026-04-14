use crate::ui::chat_message_list::{
    offset::{
        absolute_line_start, clamp_offset, compute_bottom_aligned_offset, ensure_selected_visible,
    },
    ChatMessageListItem, ScrollOffset,
};

use super::make_items;

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

// ── Bug fix: scroll oscillation ──

#[test]
fn ensure_selected_visible_no_oscillation_with_tall_padding() {
    // Scenario: 3 items, the padding items are tall, viewport is small.
    // Items: [10 lines, 10 lines, 4 lines], viewport = 8, scroll_padding = 1
    // Selecting item 2: padding_above = item1(10), selection = item2(4)
    // desired range = 10 + 4 = 14 > viewport(8) → should fall back.
    let items = make_items(&[10, 10, 4]);
    let offset = ScrollOffset { item: 2, line: 0 }; // viewport starts at item 2

    let result = ensure_selected_visible(&items, offset, 2, 8, 1);

    // Call again with the result — must be idempotent (no oscillation).
    let result2 = ensure_selected_visible(&items, result, 2, 8, 1);
    assert_eq!(
        result, result2,
        "ensure_selected_visible must be idempotent"
    );

    // The selected item (starts at line 20, 4 lines tall) must be visible.
    let sel_start = absolute_line_start(&items, 2);
    let sel_end = sel_start + items[2].height();
    let vp_top = absolute_line_start(&items, result.item) + result.line;
    let vp_bottom = vp_top + 8;
    assert!(
        sel_start >= vp_top && sel_end <= vp_bottom,
        "selected item must be fully visible: sel={sel_start}..{sel_end}, vp={vp_top}..{vp_bottom}"
    );
}

#[test]
fn ensure_selected_visible_idempotent_last_item_large_padding() {
    // Edge case: last item selected, padding items are very tall.
    // Items: [20 lines, 20 lines, 3 lines], viewport = 10, padding = 2
    let items = make_items(&[20, 20, 3]);
    let offset = compute_bottom_aligned_offset(&items, 10);

    let r1 = ensure_selected_visible(&items, offset, 2, 10, 2);
    let r2 = ensure_selected_visible(&items, r1, 2, 10, 2);
    let r3 = ensure_selected_visible(&items, r2, 2, 10, 2);
    assert_eq!(r1, r2, "first re-call must be stable");
    assert_eq!(r2, r3, "second re-call must be stable");
}

#[test]
fn ensure_selected_visible_no_oscillation_message_taller_than_viewport() {
    // A single message taller than the viewport must not oscillate between
    // top-aligned and bottom-aligned positions across frames.
    // Items: [3, 30, 3], viewport = 10, padding = 1
    // Message 1 (30 lines) cannot fit; once partially visible, offset must stabilize.
    let items = make_items(&[3, 30, 3]);

    // Start with message 1 top-aligned.
    let offset = ScrollOffset { item: 1, line: 0 };
    let r1 = ensure_selected_visible(&items, offset, 1, 10, 1);
    let r2 = ensure_selected_visible(&items, r1, 1, 10, 1);
    assert_eq!(
        r1, r2,
        "must be stable when message is taller than viewport"
    );

    // Start with message 1 bottom-aligned.
    let offset2 = ScrollOffset { item: 1, line: 23 };
    let r3 = ensure_selected_visible(&items, offset2, 1, 10, 1);
    let r4 = ensure_selected_visible(&items, r3, 1, 10, 1);
    assert_eq!(r3, r4, "must be stable from bottom-aligned start");
}
