//! Shared utilities for popup overlays.

use ratatui::layout::Rect;

/// Computes a centered rectangle within the given area.
///
/// `percent_x` and `percent_y` control the popup size as a percentage
/// of the available area. Minimum size is clamped to 30x10.
pub fn centered_rect(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let popup_width = (area.width * percent_x / 100).max(30).min(area.width);
    let popup_height = (area.height * percent_y / 100).max(10).min(area.height);

    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_height)) / 2;

    Rect::new(x, y, popup_width, popup_height)
}

#[cfg(test)]
mod tests {
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
}
