//! Renders a centered help popup overlay showing available hotkeys.

use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph},
    Frame,
};

use crate::domain::{
    help_content::{self, HotkeyEntry},
    shell_state::ActivePane,
};

use super::styles;

/// Renders the help popup as an overlay on top of existing content.
///
/// The popup is centered and sized proportionally to the terminal area.
/// Content depends on which pane is currently active.
pub fn render_help_popup(frame: &mut Frame<'_>, area: Rect, active_pane: ActivePane) {
    let popup_area = centered_rect(area, 50, 70);

    // Erase underlying content under the popup.
    frame.render_widget(Clear, popup_area);

    let (title, entries) = match active_pane {
        ActivePane::ChatList => ("Help — Chat List", help_content::chat_list_hotkeys()),
        ActivePane::Messages => ("Help — Messages", help_content::messages_hotkeys()),
        // MessageInput should not open help, but handle gracefully.
        ActivePane::MessageInput => ("Help — Messages", help_content::messages_hotkeys()),
    };

    let block = Block::default()
        .title(format!(" {title} "))
        .borders(Borders::ALL)
        .border_style(styles::help_popup_border_style())
        .padding(Padding::new(2, 2, 1, 1));

    let mut lines = build_hotkey_lines(entries);
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press q, ? or Esc to close",
        styles::help_popup_footer_style(),
    )));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, popup_area);
}

/// Builds styled lines from hotkey entries, aligning keys and action names.
fn build_hotkey_lines(entries: &[HotkeyEntry]) -> Vec<Line<'static>> {
    // Find max key label width for alignment.
    let max_key_width = entries.iter().map(|e| e.key_label.len()).max().unwrap_or(0);

    entries
        .iter()
        .map(|entry| {
            let padded_key = format!("{:<width$}", entry.key_label, width = max_key_width);
            Line::from(vec![
                Span::styled(padded_key, styles::help_popup_key_style()),
                Span::styled("  ", styles::help_popup_action_style()),
                Span::styled(
                    entry.action_name.to_owned(),
                    styles::help_popup_action_style(),
                ),
            ])
        })
        .collect()
}

/// Computes a centered rectangle within the given area.
///
/// `percent_x` and `percent_y` control the popup size as a percentage
/// of the available area. Minimum size is clamped to 30x10.
fn centered_rect(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
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
    fn build_hotkey_lines_aligns_keys() {
        let entries = &[
            HotkeyEntry {
                key_label: "j",
                action_name: "next",
            },
            HotkeyEntry {
                key_label: "Enter / l",
                action_name: "open",
            },
        ];
        let lines = build_hotkey_lines(entries);
        assert_eq!(lines.len(), 2);
        // First line key span should be padded to match "Enter / l" width (9 chars)
        let first_key = &lines[0].spans[0];
        assert_eq!(first_key.content.len(), 9);
    }

    #[test]
    fn build_hotkey_lines_empty_input() {
        let lines = build_hotkey_lines(&[]);
        assert!(lines.is_empty());
    }

    #[test]
    fn build_hotkey_lines_single_entry_no_padding_needed() {
        let entries = &[HotkeyEntry {
            key_label: "q",
            action_name: "quit",
        }];
        let lines = build_hotkey_lines(entries);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans.len(), 3);
        // key, separator, action
        assert_eq!(lines[0].spans[0].content, "q");
        assert_eq!(lines[0].spans[1].content, "  ");
        assert_eq!(lines[0].spans[2].content, "quit");
    }

    #[test]
    fn build_hotkey_lines_has_correct_span_count() {
        let entries = &[
            HotkeyEntry {
                key_label: "j",
                action_name: "next",
            },
            HotkeyEntry {
                key_label: "k",
                action_name: "prev",
            },
        ];
        let lines = build_hotkey_lines(entries);
        for line in &lines {
            assert_eq!(line.spans.len(), 3, "each line should have 3 spans");
        }
    }

    #[test]
    fn build_hotkey_lines_uses_correct_styles() {
        let entries = &[HotkeyEntry {
            key_label: "j",
            action_name: "next",
        }];
        let lines = build_hotkey_lines(entries);
        assert_eq!(lines[0].spans[0].style, styles::help_popup_key_style());
        assert_eq!(lines[0].spans[2].style, styles::help_popup_action_style());
    }

    #[test]
    fn centered_rect_with_non_zero_origin() {
        let area = Rect::new(10, 5, 100, 40);
        let result = centered_rect(area, 50, 50);
        assert!(result.x >= area.x);
        assert!(result.y >= area.y);
        assert!(result.right() <= area.right());
        assert!(result.bottom() <= area.bottom());
        // Should be centered within the area
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

    #[test]
    fn build_hotkey_lines_with_real_chat_list_data() {
        let entries = help_content::chat_list_hotkeys();
        let lines = build_hotkey_lines(entries);
        assert_eq!(lines.len(), entries.len());
    }

    #[test]
    fn build_hotkey_lines_with_real_messages_data() {
        let entries = help_content::messages_hotkeys();
        let lines = build_hotkey_lines(entries);
        assert_eq!(lines.len(), entries.len());
    }
}
