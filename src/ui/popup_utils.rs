//! Shared utilities for popup overlays.

use ratatui::{
    layout::Rect,
    widgets::{Block, Paragraph},
};
use unicode_width::UnicodeWidthStr;

/// Keeps short states like "Loading..." from collapsing the popup and the
/// width from jumping when the content arrives.
pub(super) const MIN_CONTENT_WIDTH: u16 = 30;
const MAX_CONTENT_WIDTH_PERCENT: u32 = 80;
/// Rows kept free above and below a content-sized popup.
const VERTICAL_MARGIN: u16 = 1;
/// The block title sits on the top border between the two corner cells.
const TITLE_CORNERS_WIDTH: u16 = 2;

/// Computes a centered rectangle within the given area.
///
/// `percent_x` and `percent_y` control the popup size as a percentage
/// of the available area. Minimum size is clamped to 30x10.
pub fn centered_rect(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let popup_width = (area.width * percent_x / 100).max(30).min(area.width);
    let popup_height = (area.height * percent_y / 100).max(10).min(area.height);

    centered(area, popup_width, popup_height)
}

/// Computes a centered rectangle sized to fit `content` framed by `block`.
///
/// Width follows the widest content line or the `title`, clamped between
/// `MIN_CONTENT_WIDTH` and `MAX_CONTENT_WIDTH_PERCENT` of the area; `content`
/// should wrap so that longer lines stay visible. Height follows the line
/// count at that width, capped at the area height minus `VERTICAL_MARGIN` on
/// each side; anything beyond the cap is cut off.
///
/// `content` must not have a block set: `block` is measured separately.
pub fn content_sized_rect(
    area: Rect,
    title: &str,
    block: &Block<'_>,
    content: &Paragraph<'_>,
) -> Rect {
    let unbounded = Rect::new(0, 0, u16::MAX, u16::MAX);
    let inner = block.inner(unbounded);
    let chrome_width = unbounded.width - inner.width;
    let chrome_height = unbounded.height - inner.height;

    let max_width = (u32::from(area.width) * MAX_CONTENT_WIDTH_PERCENT / 100) as u16;
    let max_width = max_width.max(MIN_CONTENT_WIDTH).min(area.width);
    let max_height = area.height.saturating_sub(2 * VERTICAL_MARGIN);

    let content_width = saturating_u16(content.line_width()).saturating_add(chrome_width);
    let title_width = saturating_u16(title.width()).saturating_add(TITLE_CORNERS_WIDTH);
    let width = content_width
        .max(title_width)
        .max(MIN_CONTENT_WIDTH)
        .min(max_width);

    let content_height = saturating_u16(content.line_count(width.saturating_sub(chrome_width)));
    let height = content_height.saturating_add(chrome_height).min(max_height);

    centered(area, width, height)
}

pub(super) fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;

    Rect::new(x, y, width, height)
}

pub(super) fn saturating_u16(value: usize) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

#[cfg(test)]
pub(super) fn rendered_border_rect(buffer: &ratatui::buffer::Buffer) -> Option<Rect> {
    let area = buffer.area;
    let cells = || {
        (area.top()..area.bottom()).flat_map(|y| (area.left()..area.right()).map(move |x| (x, y)))
    };
    let (left, top) = cells().find(|&pos| buffer[pos].symbol() == "┌")?;
    let (right, bottom) = cells().find(|&pos| buffer[pos].symbol() == "┘")?;

    Some(Rect::new(left, top, right - left + 1, bottom - top + 1))
}

#[cfg(test)]
mod tests {
    use ratatui::{
        text::Line,
        widgets::{Borders, Padding, Wrap},
    };

    use super::*;

    #[test]
    fn centered_rect_is_within_bounds() {
        let area = Rect::new(0, 0, 100, 40);
        let result = centered_rect(area, 50, 70);
        assert!(result.x >= area.x);
        assert!(result.y >= area.y);
        assert!(result.right() <= area.right());
        assert!(result.bottom() <= area.bottom());
    }

    #[test]
    fn centered_rect_is_centered() {
        let area = Rect::new(0, 0, 100, 40);
        let result = centered_rect(area, 50, 70);
        assert_eq!(result.width, 50);
        assert_eq!(result.height, 28);
        assert_eq!(result.x, 25);
        assert_eq!(result.y, 6);
    }

    #[test]
    fn centered_rect_clamps_to_minimum() {
        let area = Rect::new(0, 0, 40, 12);
        let result = centered_rect(area, 10, 10);
        assert_eq!(result.width, 30);
        assert_eq!(result.height, 10);
    }

    #[test]
    fn centered_rect_does_not_exceed_area() {
        let area = Rect::new(0, 0, 20, 8);
        let result = centered_rect(area, 200, 200);
        assert_eq!(result.width, 20);
        assert_eq!(result.height, 8);
    }

    #[test]
    fn centered_rect_with_non_zero_origin() {
        let area = Rect::new(10, 5, 100, 40);
        let result = centered_rect(area, 50, 50);
        assert!(result.x >= area.x);
        assert!(result.y >= area.y);
        assert!(result.right() <= area.right());
        assert!(result.bottom() <= area.bottom());
        let expected_x = 10 + (100 - 50) / 2;
        let expected_y = 5 + (40 - 20) / 2;
        assert_eq!(result.x, expected_x);
        assert_eq!(result.y, expected_y);
    }

    #[test]
    fn centered_rect_zero_percent_uses_minimum() {
        let area = Rect::new(0, 0, 80, 24);
        let result = centered_rect(area, 0, 0);
        assert_eq!(result.width, 30);
        assert_eq!(result.height, 10);
    }

    fn popup_block() -> Block<'static> {
        Block::default()
            .borders(Borders::ALL)
            .padding(Padding::new(2, 2, 1, 1))
    }

    fn wrapped(lines: Vec<String>) -> Paragraph<'static> {
        Paragraph::new(lines.into_iter().map(Line::from).collect::<Vec<_>>())
            .wrap(Wrap { trim: false })
    }

    #[test]
    fn content_sized_rect_keeps_minimum_width_for_short_content() {
        let area = Rect::new(0, 0, 100, 40);
        let content = wrapped(vec!["Loading...".into()]);
        let result = content_sized_rect(area, " Info ", &popup_block(), &content);
        assert_eq!(result, Rect::new(35, 17, 30, 5));
    }

    #[test]
    fn content_sized_rect_height_follows_line_count() {
        let area = Rect::new(0, 0, 100, 40);
        let content = wrapped(vec!["a".into(), "b".into(), "c".into()]);
        let result = content_sized_rect(area, "", &popup_block(), &content);
        assert_eq!(result.height, 3 + 4);
    }

    #[test]
    fn content_sized_rect_width_follows_widest_line() {
        let area = Rect::new(0, 0, 100, 40);
        let content = wrapped(vec!["short".into(), "x".repeat(50)]);
        let result = content_sized_rect(area, "", &popup_block(), &content);
        assert_eq!(result.width, 50 + 6);
        assert_eq!(result.height, 2 + 4);
        assert_eq!(result.x, (100 - 56) / 2);
    }

    #[test]
    fn content_sized_rect_measures_display_width() {
        let area = Rect::new(0, 0, 100, 40);
        let content = wrapped(vec!["Привет — 👍".repeat(4)]);
        let result = content_sized_rect(area, "", &popup_block(), &content);
        assert_eq!(result.width, 44 + 6);
    }

    #[test]
    fn content_sized_rect_fits_title_wider_than_content() {
        let area = Rect::new(0, 0, 100, 40);
        let title = format!(" {} ", "t".repeat(40));
        let content = wrapped(vec!["Loading...".into()]);
        let result = content_sized_rect(area, &title, &popup_block(), &content);
        assert_eq!(result.width, 42 + 2);
    }

    #[test]
    fn content_sized_rect_caps_width_and_wraps_long_lines() {
        let area = Rect::new(0, 0, 100, 40);
        let content = wrapped(vec!["word ".repeat(30).trim_end().to_owned()]);
        let result = content_sized_rect(area, "", &popup_block(), &content);
        assert_eq!(result.width, 80);
        // 74 inner columns fit 15 of the 30 words per row.
        assert_eq!(result.height, 2 + 4);
    }

    #[test]
    fn content_sized_rect_caps_title_width() {
        let area = Rect::new(0, 0, 100, 40);
        let title = "t".repeat(200);
        let content = wrapped(vec!["Loading...".into()]);
        let result = content_sized_rect(area, &title, &popup_block(), &content);
        assert_eq!(result.width, 80);
    }

    #[test]
    fn content_sized_rect_caps_height_with_vertical_margin() {
        let area = Rect::new(0, 0, 100, 40);
        let content = wrapped((0..100).map(|i| i.to_string()).collect());
        let result = content_sized_rect(area, "", &popup_block(), &content);
        assert_eq!(result.height, 38);
        assert_eq!(result.y, 1);
    }

    #[test]
    fn content_sized_rect_minimum_width_does_not_exceed_area() {
        let area = Rect::new(0, 0, 20, 10);
        let content = wrapped(vec!["Loading...".into()]);
        let result = content_sized_rect(area, "", &popup_block(), &content);
        assert_eq!(result.width, 20);
        assert_eq!(result.x, 0);
    }

    #[test]
    fn content_sized_rect_does_not_overflow_on_wide_area() {
        let area = Rect::new(0, 0, 1000, 40);
        let content = wrapped(vec!["x".repeat(900)]);
        let result = content_sized_rect(area, "", &popup_block(), &content);
        assert_eq!(result.width, 800);
    }

    #[test]
    fn content_sized_rect_with_non_zero_origin() {
        let area = Rect::new(10, 5, 100, 40);
        let content = wrapped(vec!["Loading...".into()]);
        let result = content_sized_rect(area, "", &popup_block(), &content);
        assert_eq!(result, Rect::new(10 + 35, 5 + 17, 30, 5));
    }
}
